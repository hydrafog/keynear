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
import org.keynear.app.ui.model.VisibilityState

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
}

