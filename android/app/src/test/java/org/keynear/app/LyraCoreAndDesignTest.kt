package org.keynear.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.keynear.app.core.CoreBridge
import org.keynear.app.core.CoreSnapshot
import org.keynear.app.core.CoreState
import org.keynear.app.core.ShareValidationResult
import org.keynear.app.core.VisibilityState
import org.keynear.app.core.validateShareIntent
import org.keynear.app.ui.theme.KeynearSpacing
import org.keynear.app.ui.theme.KeynearTokenHex

class LyraCoreAndDesignTest {
    private class FakeCoreBridge : CoreBridge {
        private var current = VisibilityState.PAIRED_ONLY

        private fun snapshot(): CoreSnapshot = CoreSnapshot(
            version = "0.3.0",
            maxControlFrameBytes = 65_536L,
            ready = true,
            visibility = current,
            configSummary = "visibility=PairedOnly,incoming_dir=~/Downloads/Keynear",
        )

        override fun loadState(): CoreState = CoreState.Empty(snapshot())

        override fun cycleVisibility(): CoreState {
            current = current.next()
            return CoreState.Empty(snapshot())
        }
    }

    @Test
    fun visibilityStateCyclesThroughAllModes() {
        assertEquals(VisibilityState.PAIRED_ONLY, VisibilityState.HIDDEN.next())
        assertEquals(VisibilityState.EVERYONE_NEARBY, VisibilityState.PAIRED_ONLY.next())
        assertEquals(VisibilityState.HIDDEN, VisibilityState.EVERYONE_NEARBY.next())
    }

    @Test
    fun coreBridgeTransitionsStateCleanly() {
        val bridge = FakeCoreBridge()
        val initial = bridge.loadState() as CoreState.Empty
        assertEquals("0.3.0", initial.snapshot.version)
        assertEquals(VisibilityState.PAIRED_ONLY, initial.snapshot.visibility)
        assertEquals(65_536L, initial.snapshot.maxControlFrameBytes)

        val second = bridge.cycleVisibility() as CoreState.Empty
        assertEquals(VisibilityState.EVERYONE_NEARBY, second.snapshot.visibility)

        val third = bridge.cycleVisibility() as CoreState.Empty
        assertEquals(VisibilityState.HIDDEN, third.snapshot.visibility)
    }

    @Test
    fun shareIntentValidationAllowsContentAndFileSchemesAndRejectsUntrusted() {
        val validSingle = validateShareIntent(
            action = "android.intent.action.SEND",
            streamUris = listOf("content://org.example.provider/item/1"),
            sharedText = null,
        )
        assertEquals(ShareValidationResult.Valid(itemCount = 1, hasText = false), validSingle)

        val validMultiple = validateShareIntent(
            action = "android.intent.action.SEND_MULTIPLE",
            streamUris = listOf("file:///sdcard/Download/a.txt", "content://media/external/file/2"),
            sharedText = null,
        )
        assertEquals(ShareValidationResult.Valid(itemCount = 2, hasText = false), validMultiple)

        val validText = validateShareIntent(
            action = "android.intent.action.SEND",
            streamUris = emptyList(),
            sharedText = "hello nearby",
        )
        assertEquals(ShareValidationResult.Valid(itemCount = 0, hasText = true), validText)

        val rejectedJavascript = validateShareIntent(
            action = "android.intent.action.SEND",
            streamUris = listOf("javascript:alert(1)"),
            sharedText = null,
        )
        assertTrue(rejectedJavascript is ShareValidationResult.Invalid)

        val rejectedHttp = validateShareIntent(
            action = "android.intent.action.SEND",
            streamUris = listOf("http://example.com/file.bin"),
            sharedText = null,
        )
        assertTrue(rejectedHttp is ShareValidationResult.Invalid)

        val rejectedEmpty = validateShareIntent(
            action = "android.intent.action.SEND",
            streamUris = emptyList(),
            sharedText = "   ",
        )
        assertTrue(rejectedEmpty is ShareValidationResult.Invalid)

        val rejectedAction = validateShareIntent(
            action = "android.intent.action.VIEW",
            streamUris = listOf("content://org.example.provider/item/1"),
            sharedText = null,
        )
        assertTrue(rejectedAction is ShareValidationResult.Invalid)
    }

    @Test
    fun designTokensMatchPrototypePaletteAndTouchFloor() {
        assertEquals(0xFFF0F4FA, KeynearTokenHex.LIGHT_CANVAS)
        assertEquals(0xFF0B1016, KeynearTokenHex.DARK_CANVAS)
        assertEquals(0xFF004CED, KeynearTokenHex.ACCENT_BLUE_600)
        assertEquals(0xFF0172F9, KeynearTokenHex.FOCUS_BLUE_500)
        assertEquals(48, KeynearSpacing.MIN_TOUCH_TARGET_DP)
    }
}

