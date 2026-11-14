package org.keynear.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.keynear.app.ble.KeynearBleConstants
import org.keynear.app.core.EndpointHexCodec
import org.keynear.app.core.InMemorySecretStorage
import org.keynear.app.core.KeystoreSecretStore
import org.keynear.app.core.LocalNetworkStatus
import org.keynear.app.core.PendingMediaStoreTracker
import org.keynear.app.core.PendingRowOps
import org.keynear.app.core.SecretCipher
import org.keynear.app.core.TransferStateUi
import org.keynear.app.core.evaluateConsentRequirement
import org.keynear.app.core.formatByteCount
import org.keynear.app.lan.LocalNetworkConstants

private class FakeXorCipher : SecretCipher {
    override fun encrypt(plaintext: ByteArray): ByteArray =
        ByteArray(plaintext.size) { idx -> (plaintext[idx].toInt() xor 0x3C).toByte() }

    override fun decrypt(ciphertext: ByteArray): ByteArray =
        ByteArray(ciphertext.size) { idx -> (ciphertext[idx].toInt() xor 0x3C).toByte() }
}

private class FakeMediaStoreOps(
    private val failHandOff: Boolean = false,
) : PendingRowOps<String> {
    val rows = linkedMapOf<String, Int>()
    private var nextId = 1

    override fun createPending(fileName: String, mimeType: String): String {
        val uri = "content://media/external/downloads/${nextId++}/$fileName"
        rows[uri] = 1
        return uri
    }

    override fun openDetachedFd(handle: String): Int? =
        if (failHandOff) null else 42

    override fun clearPendingFlag(handle: String) {
        if (rows.containsKey(handle)) {
            rows[handle] = 0
        }
    }

    override fun deleteRow(handle: String) {
        rows.remove(handle)
    }
}

class OrionTransferAndLnpTest {
    @Test
    fun consentTransitionsAndAutoAcceptRespectPairedPolicy() {
        assertFalse(evaluateConsentRequirement(isPaired = false, deviceAutoAccept = true, globalAutoAccept = true))
        assertFalse(evaluateConsentRequirement(isPaired = true, deviceAutoAccept = false, globalAutoAccept = false))
        assertTrue(evaluateConsentRequirement(isPaired = true, deviceAutoAccept = true, globalAutoAccept = false))
        assertTrue(evaluateConsentRequirement(isPaired = true, deviceAutoAccept = false, globalAutoAccept = true))

        assertFalse(TransferStateUi.PENDING_CONSENT.isTerminal())
        assertFalse(TransferStateUi.ACTIVE.isTerminal())
        assertTrue(TransferStateUi.COMPLETED.isTerminal())
        assertTrue(TransferStateUi.CANCELLED.isTerminal())
        assertTrue(TransferStateUi.FAILED.isTerminal())
    }

    @Test
    fun lnpSdk36ReflectionGuardAndBlockedReasonDetection() {
        assertFalse(LocalNetworkConstants.requiresLocalNetworkPermission(36))
        assertTrue(LocalNetworkConstants.requiresLocalNetworkPermission(37))

        assertNull(LocalNetworkConstants.buildDiscoveryRequestOrNull(sdkInt = 36))
        assertNull(LocalNetworkConstants.buildDiscoveryRequestOrNull(sdkInt = 37))

        assertTrue(LocalNetworkConstants.isLnpBlockedReason(LocalNetworkConstants.ANDROID_NETWORK_BLOCKED_REASON_LNP))
        assertFalse(LocalNetworkConstants.isLnpBlockedReason(0))

        assertEquals(
            LocalNetworkStatus.READY,
            LocalNetworkConstants.evaluateLnpStatus(
                sdkInt = 36,
                hasPermission = true,
                networkBlocked = false,
                blockedReasons = null,
            ),
        )
        assertEquals(
            LocalNetworkStatus.MISSING_PERMISSION,
            LocalNetworkConstants.evaluateLnpStatus(
                sdkInt = 37,
                hasPermission = false,
                networkBlocked = false,
                blockedReasons = null,
            ),
        )
        assertEquals(
            LocalNetworkStatus.BLOCKED_LNP,
            LocalNetworkConstants.evaluateLnpStatus(
                sdkInt = 37,
                hasPermission = true,
                networkBlocked = true,
                blockedReasons = LocalNetworkConstants.ANDROID_NETWORK_BLOCKED_REASON_LNP,
            ),
        )
    }

    @Test
    fun mediaStorePendingRowClearsOnCompleteAndDeletesOnCancelOrError() {
        val ops = FakeMediaStoreOps()
        val tracker = PendingMediaStoreTracker(ops)

        assertTrue(
            tracker.prepareAndHandOff("tx-ok", "photo.png", "image/png") { fd ->
                assertEquals(42, fd)
            },
        )
        assertEquals(1, ops.rows.size)
        assertEquals(1, ops.rows.values.first())
        tracker.reconcileState("tx-ok", TransferStateUi.COMPLETED)
        assertEquals(0, ops.rows.values.first())
        assertFalse(tracker.hasPending("tx-ok"))

        assertTrue(
            tracker.prepareAndHandOff("tx-cancel", "notes.txt", "text/plain") { _ -> },
        )
        tracker.reconcileState("tx-cancel", TransferStateUi.CANCELLED)
        assertFalse(ops.rows.keys.any { it.contains("notes.txt") })

        val failedHandOff = runCatching {
            tracker.prepareAndHandOff("tx-throw", "bad.bin", "application/octet-stream") { _ ->
                error("ffi rejected fd")
            }
        }
        assertTrue(failedHandOff.isFailure)
        assertFalse(ops.rows.keys.any { it.contains("bad.bin") })

        val nullFdOps = FakeMediaStoreOps(failHandOff = true)
        val nullFdTracker = PendingMediaStoreTracker(nullFdOps)
        assertFalse(nullFdTracker.prepareAndHandOff("tx-null", "empty.bin", "application/octet-stream") { _ -> })
        assertTrue(nullFdOps.rows.isEmpty())
    }

    @Test
    fun keystorePersistsQuicKeyAndBleLanCharUuidMatchesCore() {
        val store = KeystoreSecretStore(
            backend = InMemorySecretStorage(),
            cipher = FakeXorCipher(),
        )
        val seed = ByteArray(32) { (it + 11).toByte() }
        val first = store.getOrCreateQuicSecretKey { seed }
        val second = store.getOrCreateQuicSecretKey { ByteArray(32) { 0x55 } }
        assertTrue(first.contentEquals(seed))
        assertTrue(second.contentEquals(seed))

        assertEquals(
            "0000fe70-0000-1000-8000-00805f9b34fb",
            KeynearBleConstants.LAN_ENDPOINT_CHAR_UUID.toString().lowercase(),
        )

        val eid = ByteArray(32) { it.toByte() }
        val hex = EndpointHexCodec.encode(eid)
        assertEquals(64, hex.length)
        assertTrue(EndpointHexCodec.decode32(hex)!!.contentEquals(eid))
        assertNull(EndpointHexCodec.decode32("not-hex"))
        assertEquals("1.0 KB", formatByteCount(1024L))
    }
}

