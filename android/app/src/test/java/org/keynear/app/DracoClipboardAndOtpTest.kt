package org.keynear.app

import android.os.Build
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.keynear.app.core.CoreBridge
import org.keynear.app.core.CoreSnapshot
import org.keynear.app.core.CoreState
import org.keynear.app.core.DaemonConfigUi
import org.keynear.app.core.VisibilityState
import org.keynear.app.core.shouldIgnoreClipboardMimes
import org.keynear.app.core.shouldReadClipboardOnFocus
import org.keynear.app.core.shouldSkipClipDescription
import org.keynear.app.core.usePendingIntentTileCollapse
import org.keynear.app.otp.OtpConsentVerifier

class DracoClipboardAndOtpTest {
    @Test
    fun preReadMimeFilterAndSensitiveFlagSkipProtectedClips() {
        assertTrue(
            shouldIgnoreClipboardMimes(
                offeredMimes = listOf("text/plain;charset=utf-8", "x-kde-passwordManagerHint"),
                ignoredMimes = emptyList(),
            ),
        )
        assertTrue(
            shouldIgnoreClipboardMimes(
                offeredMimes = listOf("application/x-password"),
                ignoredMimes = emptyList(),
            ),
        )
        assertTrue(
            shouldIgnoreClipboardMimes(
                offeredMimes = listOf("text/plain", "Application/X-Custom-Vault; version=2"),
                ignoredMimes = listOf("application/x-custom-vault"),
            ),
        )
        assertTrue(
            shouldSkipClipDescription(
                offeredMimes = listOf("text/plain"),
                isSensitive = true,
                ignoredMimes = emptyList(),
            ),
        )
        assertFalse(
            shouldSkipClipDescription(
                offeredMimes = listOf("text/plain;charset=utf-8", "image/png"),
                isSensitive = false,
                ignoredMimes = listOf("application/x-custom-vault"),
            ),
        )
    }

    @Test
    fun clipboardReadFocusGuardAndTileCollapseApiLevels() {
        assertFalse(shouldReadClipboardOnFocus(hasFocus = false, alreadyHandled = false))
        assertTrue(shouldReadClipboardOnFocus(hasFocus = true, alreadyHandled = false))
        assertFalse(shouldReadClipboardOnFocus(hasFocus = true, alreadyHandled = true))

        assertFalse(usePendingIntentTileCollapse(Build.VERSION_CODES.S))
        assertFalse(usePendingIntentTileCollapse(Build.VERSION_CODES.TIRAMISU))
        assertTrue(usePendingIntentTileCollapse(Build.VERSION_CODES.UPSIDE_DOWN_CAKE))
        assertTrue(usePendingIntentTileCollapse(36))
    }

    @Test
    fun smsUserConsentVerifierValidatesOriginHashAndCode() {
        assertTrue(OtpConsentVerifier.isConsentIntentTrusted("com.google.android.gms"))
        assertFalse(OtpConsentVerifier.isConsentIntentTrusted("com.untrusted.spoofer"))
        assertFalse(OtpConsentVerifier.isConsentIntentTrusted(null))

        assertEquals(
            "847291",
            OtpConsentVerifier.extractVerifiedOtpCode("Your Keynear code is 847291. Valid immediately."),
        )
        assertEquals(
            "482910",
            OtpConsentVerifier.extractVerifiedOtpCode(
                smsBody = "<#> Use 482910 to sign in FA+9qCX9VSu",
                expectedAppHashes = setOf("FA+9qCX9VSu"),
            ),
        )
        assertNull(
            OtpConsentVerifier.extractVerifiedOtpCode(
                smsBody = "<#> Use 482910 to sign in WRONGHASH11",
                expectedAppHashes = setOf("FA+9qCX9VSu"),
            ),
        )
        assertNull(OtpConsentVerifier.extractVerifiedOtpCode("No digits present here"))
    }

    @Test
    fun otpPassthroughDisabledBlocksVerifiedOtpForwarding() {
        val bridge = object : CoreBridge {
            var config = DaemonConfigUi(otpPassthroughEnabled = false)
            val sent = mutableListOf<String>()

            override fun loadState(): CoreState = CoreState.Empty(
                CoreSnapshot(
                    version = "0.5.0",
                    maxControlFrameBytes = 65_536L,
                    ready = true,
                    visibility = VisibilityState.PAIRED_ONLY,
                    configSummary = "visibility=PairedOnly",
                    daemonConfig = config,
                ),
            )

            override fun cycleVisibility(): CoreState = loadState()

            override fun sendVerifiedOtp(code: String): Result<Unit> {
                if (!config.otpPassthroughEnabled) {
                    return Result.failure(IllegalStateException("OTP pass-through is disabled"))
                }
                sent.add(code)
                return Result.success(Unit)
            }
        }

        assertTrue(bridge.sendVerifiedOtp("847291").isFailure)
        assertTrue(bridge.sent.isEmpty())

        bridge.config = bridge.config.copy(otpPassthroughEnabled = true)
        assertTrue(bridge.sendVerifiedOtp("847291").isSuccess)
        assertEquals(listOf("847291"), bridge.sent)
    }
}

