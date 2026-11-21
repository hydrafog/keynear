package org.keynear.app.otp

import android.util.Base64
import java.security.MessageDigest

object OtpConsentVerifier {
    const val GOOGLE_PLAY_SERVICES_PACKAGE = "com.google.android.gms"
    private val OTP_TOKEN_REGEX = Regex("""\b(\d{4,10}|[A-Z0-9]{6,10})\b""")
    private val APP_HASH_REGEX = Regex("""^<#>\s+.*?\s([A-Za-z0-9+/]{11})\s*$""", RegexOption.DOT_MATCHES_ALL)

    fun isConsentIntentTrusted(resolvedPackage: String?): Boolean =
        resolvedPackage == GOOGLE_PLAY_SERVICES_PACKAGE

    fun verifySmsAppHash(smsBody: String, expectedAppHashes: Set<String>): Boolean {
        if (expectedAppHashes.isEmpty()) return true
        val match = APP_HASH_REGEX.find(smsBody.trim())?.groupValues?.getOrNull(1) ?: return true
        return match in expectedAppHashes
    }

    fun extractVerifiedOtpCode(
        smsBody: String,
        expectedAppHashes: Set<String> = emptySet(),
    ): String? {
        val trimmed = smsBody.trim()
        if (trimmed.isEmpty()) return null
        if (!verifySmsAppHash(trimmed, expectedAppHashes)) {
            return null
        }
        val candidates = OTP_TOKEN_REGEX.findAll(trimmed)
            .map { it.groupValues[1] }
            .filter { token -> token.any { it.isDigit() } }
            .toList()
        return candidates.firstOrNull { it.all { ch -> ch.isDigit() } } ?: candidates.firstOrNull()
    }

    fun computeAppSignatureHash(packageName: String, signatureBytes: ByteArray): String {
        val info = "$packageName ${signatureBytes.joinToString("") { "%02x".format(it) }}"
        val digest = MessageDigest.getInstance("SHA-256").digest(info.toByteArray(Charsets.UTF_8))
        return Base64.encodeToString(digest.copyOf(9), Base64.NO_PADDING or Base64.NO_WRAP).take(11)
    }
}

