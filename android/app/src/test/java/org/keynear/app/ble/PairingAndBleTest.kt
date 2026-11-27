package org.keynear.app.ble

import android.Manifest
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.keynear.app.ble.KeynearBleConstants
import org.keynear.app.ble.PairingNotifier
import org.keynear.app.bridge.InMemorySecretStorage
import org.keynear.app.bridge.KeystoreSecretStore
import org.keynear.app.bridge.PeerSecretRecord
import org.keynear.app.bridge.SecretCipher
import org.keynear.app.bridge.SecretStorageBackend
import org.keynear.app.bridge.computeRemainingPairingSecs
import org.keynear.app.ui.model.PendingPairingUi
import org.keynear.app.ui.model.VisibilityState
import uniffi.keynear_core.FfiPairingState

private class TestXorCipher : SecretCipher {
    override fun encrypt(plaintext: ByteArray): ByteArray =
        ByteArray(plaintext.size) { idx -> (plaintext[idx].toInt() xor 0x5A).toByte() }

    override fun decrypt(ciphertext: ByteArray): ByteArray =
        ByteArray(ciphertext.size) { idx -> (ciphertext[idx].toInt() xor 0x5A).toByte() }
}

private class FailingPutStorage : SecretStorageBackend {
    override fun getString(key: String): String? = null
    override fun putString(key: String, value: String): Boolean = false
    override fun remove(key: String): Boolean = false
    override fun allKeys(): Set<String> = emptySet()
}

class PairingAndBleTest {
    @Test
    fun everyoneNearbyFormatsCountdownAndClampsNegativeRemaining() {
        assertEquals("Everyone (09:59)", VisibilityState.EVERYONE_NEARBY.formattedStatus(599L))
        assertEquals("Everyone (00:00)", VisibilityState.EVERYONE_NEARBY.formattedStatus(0L))
        assertEquals("Everyone (00:00)", VisibilityState.EVERYONE_NEARBY.formattedStatus(-1L))
    }

    @Test
    fun keystoreSecretStoreRoundTripsAndDeletesSecrets() {
        val store = KeystoreSecretStore(
            backend = InMemorySecretStorage(),
            cipher = TestXorCipher(),
        )

        val generatedKey = ByteArray(32) { (it + 1).toByte() }
        val first = store.getOrCreateStaticPrivateKey { generatedKey }
        val second = store.getOrCreateStaticPrivateKey { ByteArray(32) { 99 } }
        assertArrayEquals(generatedKey, first)
        assertArrayEquals(generatedKey, second)

        val irk = ByteArray(32) { (it * 3).toByte() }
        val reconnect = ByteArray(32) { (it * 7).toByte() }
        val record = PeerSecretRecord(irk = irk.copyOf(), reconnectSecret = reconnect.copyOf())
        assertFalse(record.toString().contains(irk[0].toString()))
        assertTrue(record.toString().contains("REDACTED"))

        store.putPeerSecrets("desktop-alpha", record)
        assertEquals(listOf("desktop-alpha"), store.listPeerIds())

        val loaded = store.getPeerSecrets("desktop-alpha")!!
        assertArrayEquals(irk, loaded.irk)
        assertArrayEquals(reconnect, loaded.reconnectSecret)

        store.deletePeerSecrets("desktop-alpha")
        assertNull(store.getPeerSecrets("desktop-alpha"))
        assertEquals(emptyList<String>(), store.listPeerIds())
    }

    @Test
    fun keystoreSecretStoreCounterAndFailurePropagation() {
        val store = KeystoreSecretStore(
            backend = InMemorySecretStorage(),
            cipher = TestXorCipher(),
        )
        assertEquals(1L, store.nextPresenceCounter())
        assertEquals(2L, store.nextPresenceCounter())

        val failingStore = KeystoreSecretStore(
            backend = FailingPutStorage(),
            cipher = TestXorCipher(),
        )
        val failed = runCatching {
            failingStore.putPeerSecrets(
                "desktop-alpha",
                PeerSecretRecord(ByteArray(32) { 1 }, ByteArray(32) { 2 }),
            )
        }
        assertTrue(failed.isFailure)

        val invalidId = runCatching {
            store.getPeerSecrets("../escape")
        }
        assertTrue(invalidId.isFailure)
    }

    @Test
    fun bleConstantsEnforceGattFrameBoundsAndPermissions() {
        assertFalse(KeynearBleConstants.isValidGattFrame(ByteArray(0)))
        assertTrue(KeynearBleConstants.isValidGattFrame(ByteArray(1)))
        assertTrue(KeynearBleConstants.isValidGattFrame(ByteArray(512)))
        assertFalse(KeynearBleConstants.isValidGattFrame(ByteArray(513)))

        val api31Perms = KeynearBleConstants.requiredPermissions(31)
        assertEquals(3, api31Perms.size)
        assertTrue(api31Perms.contains(Manifest.permission.BLUETOOTH_SCAN))
        assertTrue(api31Perms.contains(Manifest.permission.BLUETOOTH_CONNECT))
        assertTrue(api31Perms.contains(Manifest.permission.BLUETOOTH_ADVERTISE))

        val api33Perms = KeynearBleConstants.requiredPermissions(33)
        assertEquals(4, api33Perms.size)
        assertTrue(api33Perms.contains(Manifest.permission.POST_NOTIFICATIONS))
    }

    @Test
    fun pairingNotificationFormatsSixDigitCode() {
        assertEquals("482 193", PairingNotifier.formatCode("482193"))
        assertEquals("007 042", PairingNotifier.formatCode("007042"))
        assertEquals("bad", PairingNotifier.formatCode("bad"))
    }

    @Test
    fun repeatedTapForSamePeerWhilePairingIsInFlightIsIgnored() {
        assertTrue(GattSession.shouldIgnorePairingTap("AA:BB:CC:DD:EE:FF", "AA:BB:CC:DD:EE:FF"))
        assertTrue(GattSession.shouldIgnorePairingTap("aa:bb:cc:dd:ee:ff", "AA:BB:CC:DD:EE:FF"))
    }

    @Test
    fun tapForDifferentOrNoActivePeerStartsPairing() {
        assertFalse(GattSession.shouldIgnorePairingTap(null, "AA:BB:CC:DD:EE:FF"))
        assertFalse(GattSession.shouldIgnorePairingTap("11:22:33:44:55:66", "AA:BB:CC:DD:EE:FF"))
    }

    @Test
    fun decideApprovalActionDistinguishesMissingApprovedAndAwaitingStates() {
        assertEquals(GattSession.ApprovalAction.NO_SESSION, GattSession.decideApprovalAction(null))
        assertEquals(
            GattSession.ApprovalAction.NO_SESSION,
            GattSession.decideApprovalAction(FfiPairingState.HANDSHAKE_M2),
        )
        assertEquals(
            GattSession.ApprovalAction.ALREADY_APPROVED,
            GattSession.decideApprovalAction(FfiPairingState.APPROVED),
        )
        assertEquals(
            GattSession.ApprovalAction.APPROVE_NOW,
            GattSession.decideApprovalAction(FfiPairingState.AWAITING_CONFIRMATION),
        )
    }

    @Test
    fun duplicateLocalApprovalTransitionsOnceClearsPromptAndNeverRejects() {
        var state = FfiPairingState.AWAITING_CONFIRMATION
        var exportBuildCount = 0
        var sendCount = 0
        var clearPromptCount = 0
        var rejectCount = 0
        var completeCount = 0

        val firstCompleted = GattSession.approveSessionStep(
            state = state,
            hasBufferedPeerExport = false,
            approveAndBuildExport = {
                exportBuildCount++
                state = FfiPairingState.APPROVED
                byteArrayOf(1, 2, 3)
            },
            sendExport = { sendCount++ },
            clearPendingPrompt = { clearPromptCount++ },
            completeBuffered = {
                completeCount++
                true
            },
            onFailure = { rejectCount++ },
        )
        val secondCompleted = GattSession.approveSessionStep(
            state = state,
            hasBufferedPeerExport = false,
            approveAndBuildExport = {
                exportBuildCount++
                throw IllegalStateException("already approved")
            },
            sendExport = { sendCount++ },
            clearPendingPrompt = { clearPromptCount++ },
            completeBuffered = {
                completeCount++
                true
            },
            onFailure = { rejectCount++ },
        )

        assertFalse(firstCompleted)
        assertFalse(secondCompleted)
        assertEquals(FfiPairingState.APPROVED, state)
        assertEquals(1, exportBuildCount)
        assertEquals(1, sendCount)
        assertEquals(2, clearPromptCount)
        assertEquals(0, rejectCount)
        assertEquals(0, completeCount)
    }

    @Test
    fun approvalWithBufferedPeerExportCompletesImmediately() {
        var state = FfiPairingState.AWAITING_CONFIRMATION
        var sentExport: ByteArray? = null
        var clearedPrompt = false
        var completedBuffered = false

        val completed = GattSession.approveSessionStep(
            state = state,
            hasBufferedPeerExport = true,
            approveAndBuildExport = {
                state = FfiPairingState.APPROVED
                byteArrayOf(7, 8, 9)
            },
            sendExport = { sentExport = it },
            clearPendingPrompt = { clearedPrompt = true },
            completeBuffered = {
                completedBuffered = true
                true
            },
            onFailure = { throw AssertionError("unexpected failure", it) },
        )

        assertTrue(completed)
        assertArrayEquals(byteArrayOf(7, 8, 9), sentExport)
        assertTrue(clearedPrompt)
        assertTrue(completedBuffered)
    }

    @Test
    fun computeRemainingPairingSecsDoesNotDecrementWithinSameSecondAndExpiresAtDeadline() {
        val prompt = PendingPairingUi(
            peerAddress = "AA:BB:CC:DD:EE:FF",
            peerName = "workstation",
            verificationCode = "482193",
            remainingSecs = 120L,
        )
        val deadlineMs = 121_000L

        assertEquals(120L, computeRemainingPairingSecs(prompt, deadlineMs, 1_000L)?.remainingSecs)
        assertEquals(120L, computeRemainingPairingSecs(prompt, deadlineMs, 1_250L)?.remainingSecs)
        assertEquals(120L, computeRemainingPairingSecs(prompt, deadlineMs, 1_999L)?.remainingSecs)
        assertEquals(119L, computeRemainingPairingSecs(prompt, deadlineMs, 2_000L)?.remainingSecs)
        assertEquals(1L, computeRemainingPairingSecs(prompt, deadlineMs, 120_500L)?.remainingSecs)
        assertNull(computeRemainingPairingSecs(prompt, deadlineMs, 121_000L))
        assertNull(computeRemainingPairingSecs(prompt, deadlineMs, 125_000L))
        assertNull(computeRemainingPairingSecs(null, deadlineMs, 1_000L))
    }

    @Test
    fun disconnectIdentityCheckClearsOnlyMatchingGattAndAllowsSubsequentTap() {
        val activeGatt = Any()
        val staleGatt = Any()

        assertFalse(GattSession.shouldClearOnDisconnect(activeGatt, staleGatt))
        assertFalse(GattSession.shouldClearOnDisconnect(null, activeGatt))
        assertTrue(GattSession.shouldClearOnDisconnect(activeGatt, activeGatt))
        assertFalse(GattSession.shouldIgnorePairingTap(null, "AA:BB:CC:DD:EE:FF"))
    }
}

