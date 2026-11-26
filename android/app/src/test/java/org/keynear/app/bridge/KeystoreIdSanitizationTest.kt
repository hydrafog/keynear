package org.keynear.app.bridge

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class KeystoreIdSanitizationTest {
    private class XorCipher : SecretCipher {
        override fun encrypt(plaintext: ByteArray): ByteArray =
            ByteArray(plaintext.size) { (plaintext[it].toInt() xor 0x5A).toByte() }

        override fun decrypt(ciphertext: ByteArray): ByteArray =
            ByteArray(ciphertext.size) { (ciphertext[it].toInt() xor 0x5A).toByte() }
    }

    private fun store() = KeystoreSecretStore(InMemorySecretStorage(), XorCipher())

    private fun record() = PeerSecretRecord(ByteArray(32) { 1 }, ByteArray(32) { 2 })

    @Test
    fun putPeerSecretsTrimsAndPersistsUnderSanitizedId() {
        val store = store()
        store.putPeerSecrets("  dev-1  ", record())
        val read = store.getPeerSecrets("dev-1")!!
        assertTrue(read.irk.contentEquals(ByteArray(32) { 1 }))
        assertTrue(read.reconnectSecret.contentEquals(ByteArray(32) { 2 }))
        assertNotNull(store.getPeerSecrets("  dev-1  "))
        assertEquals(listOf("dev-1"), store.listPeerIds())

        store.deletePeerSecrets("  dev-1  ")
        assertNull(store.getPeerSecrets("dev-1"))
        assertTrue(store.listPeerIds().isEmpty())
    }

    @Test
    fun malformedDeviceIdsAreRejectedThroughObservablePeerOperations() {
        val store = store()
        val invalidIds = listOf("", "   ", "a".repeat(65), "dev..1", "dev/1", "dev@1", "dev 1", "dev\n1")
        for (id in invalidIds) {
            assertTrue("put should reject [$id]", runCatching { store.putPeerSecrets(id, record()) }.isFailure)
            assertTrue("get should reject [$id]", runCatching { store.getPeerSecrets(id) }.isFailure)
            assertTrue("delete should reject [$id]", runCatching { store.deletePeerSecrets(id) }.isFailure)
        }
    }

    @Test
    fun validIdAllowsDotsDashesAndUnderscoresWithinLength() {
        val store = store()
        val id = "dev.1_2-3"
        store.putPeerSecrets(id, record())
        assertNotNull(store.getPeerSecrets(id))
        assertEquals(listOf(id), store.listPeerIds())
    }
}
