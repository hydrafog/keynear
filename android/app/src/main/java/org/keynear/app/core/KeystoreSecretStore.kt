package org.keynear.app.core

import android.content.Context
import android.content.SharedPreferences
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.security.KeyStore
import java.util.Base64
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

data class PeerSecretRecord(
    val irk: ByteArray,
    val reconnectSecret: ByteArray,
) {
    fun zeroize() {
        irk.fill(0)
        reconnectSecret.fill(0)
    }

    override fun toString(): String = "PeerSecretRecord(irk=[REDACTED], reconnectSecret=[REDACTED])"
}

interface SecretStorageBackend {
    fun getString(key: String): String?
    fun putString(key: String, value: String): Boolean
    fun remove(key: String): Boolean
    fun allKeys(): Set<String>
}

interface SecretCipher {
    fun encrypt(plaintext: ByteArray): ByteArray
    fun decrypt(ciphertext: ByteArray): ByteArray
}

class InMemorySecretStorage : SecretStorageBackend {
    private val map = linkedMapOf<String, String>()

    @Synchronized
    override fun getString(key: String): String? = map[key]

    @Synchronized
    override fun putString(key: String, value: String): Boolean {
        map[key] = value
        return true
    }

    @Synchronized
    override fun remove(key: String): Boolean {
        map.remove(key)
        return true
    }

    @Synchronized
    override fun allKeys(): Set<String> = map.keys.toSet()
}

class SharedPrefsSecretStorage(
    private val prefs: SharedPreferences,
) : SecretStorageBackend {
    override fun getString(key: String): String? = prefs.getString(key, null)

    override fun putString(key: String, value: String): Boolean =
        prefs.edit().putString(key, value).commit()

    override fun remove(key: String): Boolean =
        prefs.edit().remove(key).commit()

    override fun allKeys(): Set<String> = prefs.all.keys.toSet()
}

class AndroidKeystoreCipher(
    private val keyAlias: String = KEY_ALIAS,
) : SecretCipher {
    private fun getOrCreateKey(): SecretKey {
        val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        val existing = keyStore.getKey(keyAlias, null) as? SecretKey
        if (existing != null) {
            return existing
        }
        val keyGenerator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE)
        val spec = KeyGenParameterSpec.Builder(
            keyAlias,
            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
        )
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setKeySize(256)
            .setUserAuthenticationRequired(false)
            .build()
        keyGenerator.init(spec)
        return keyGenerator.generateKey()
    }

    override fun encrypt(plaintext: ByteArray): ByteArray {
        val cipher = Cipher.getInstance(AES_GCM_TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, getOrCreateKey())
        val iv = cipher.iv
        require(iv.size == GCM_IV_BYTES) { "unexpected GCM IV length" }
        val sealed = cipher.doFinal(plaintext)
        return ByteArray(GCM_IV_BYTES + sealed.size).apply {
            System.arraycopy(iv, 0, this, 0, GCM_IV_BYTES)
            System.arraycopy(sealed, 0, this, GCM_IV_BYTES, sealed.size)
        }
    }

    override fun decrypt(ciphertext: ByteArray): ByteArray {
        require(ciphertext.size > GCM_IV_BYTES) { "encrypted secret payload truncated" }
        val iv = ciphertext.copyOfRange(0, GCM_IV_BYTES)
        val body = ciphertext.copyOfRange(GCM_IV_BYTES, ciphertext.size)
        val cipher = Cipher.getInstance(AES_GCM_TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, getOrCreateKey(), GCMParameterSpec(GCM_TAG_BITS, iv))
        return cipher.doFinal(body)
    }

    companion object {
        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
        private const val KEY_ALIAS = "keynear_cygnus_master_key"
        private const val AES_GCM_TRANSFORMATION = "AES/GCM/NoPadding"
        private const val GCM_IV_BYTES = 12
        private const val GCM_TAG_BITS = 128
    }
}

class KeystoreSecretStore(
    private val backend: SecretStorageBackend,
    private val cipher: SecretCipher,
) {
    @Synchronized
    fun getOrCreateStaticPrivateKey(generator: () -> ByteArray): ByteArray {
        val existing = backend.getString(KEY_STATIC_PRIVATE)
        if (existing != null) {
            val raw = Base64.getDecoder().decode(existing)
            val decrypted = cipher.decrypt(raw)
            require(decrypted.size == SECRET_BYTES) { "invalid static private key length" }
            return decrypted
        }
        val created = generator()
        require(created.size == SECRET_BYTES) { "generated static key must be 32 bytes" }
        val sealed = cipher.encrypt(created)
        val ok = backend.putString(KEY_STATIC_PRIVATE, Base64.getEncoder().encodeToString(sealed))
        check(ok) { "failed to commit static keypair to storage" }
        return created
    }

    @Synchronized
    fun putPeerSecrets(deviceId: String, record: PeerSecretRecord) {
        val cleanId = sanitizeId(deviceId)
        require(record.irk.size == SECRET_BYTES) { "irk must be 32 bytes" }
        require(record.reconnectSecret.size == SECRET_BYTES) { "reconnectSecret must be 32 bytes" }
        val combined = ByteArray(SECRET_BYTES * 2)
        try {
            System.arraycopy(record.irk, 0, combined, 0, SECRET_BYTES)
            System.arraycopy(record.reconnectSecret, 0, combined, SECRET_BYTES, SECRET_BYTES)
            val sealed = cipher.encrypt(combined)
            val encoded = Base64.getEncoder().encodeToString(sealed)
            val committed = backend.putString("$PEER_PREFIX$cleanId", encoded)
            check(committed) { "failed to persist peer secrets" }
        } finally {
            combined.fill(0)
        }
    }

    @Synchronized
    fun getPeerSecrets(deviceId: String): PeerSecretRecord? {
        val cleanId = sanitizeId(deviceId)
        val encoded = backend.getString("$PEER_PREFIX$cleanId") ?: return null
        val sealed = Base64.getDecoder().decode(encoded)
        val combined = cipher.decrypt(sealed)
        return try {
            require(combined.size == SECRET_BYTES * 2) { "corrupted peer secret record" }
            PeerSecretRecord(
                irk = combined.copyOfRange(0, SECRET_BYTES),
                reconnectSecret = combined.copyOfRange(SECRET_BYTES, SECRET_BYTES * 2),
            )
        } finally {
            combined.fill(0)
        }
    }

    @Synchronized
    fun deletePeerSecrets(deviceId: String) {
        val cleanId = sanitizeId(deviceId)
        backend.remove("$PEER_PREFIX$cleanId")
    }

    @Synchronized
    fun nextPresenceCounter(): Long {
        val raw = backend.getString(KEY_PRESENCE_COUNTER)?.toLongOrNull() ?: 0L
        val next = raw + 1L
        check(next <= 0xFFFF_FFFFL) { "presence counter exhausted u32 range" }
        val committed = backend.putString(KEY_PRESENCE_COUNTER, next.toString())
        check(committed) { "failed to persist presence counter" }
        return next
    }

    @Synchronized
    fun listPeerIds(): List<String> = backend.allKeys()
        .filter { it.startsWith(PEER_PREFIX) }
        .map { it.removePrefix(PEER_PREFIX) }
        .sorted()

    private fun sanitizeId(raw: String): String {
        val trimmed = raw.trim()
        require(trimmed.isNotEmpty() && trimmed.length <= 64) { "invalid device_id length" }
        require(!trimmed.contains("..") && trimmed.all { it.isLetterOrDigit() || it == '-' || it == '_' || it == '.' }) {
            "invalid device_id characters"
        }
        return trimmed
    }

    companion object {
        private const val PREFS_NAME = "keynear_cygnus_keystore_vault"
        private const val KEY_STATIC_PRIVATE = "local_static_private_v1"
        private const val KEY_PRESENCE_COUNTER = "presence_counter_v1"
        private const val PEER_PREFIX = "peer_v1:"
        private const val SECRET_BYTES = 32

        fun open(context: Context): KeystoreSecretStore {
            val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            return KeystoreSecretStore(
                backend = SharedPrefsSecretStorage(prefs),
                cipher = AndroidKeystoreCipher(),
            )
        }
    }
}
