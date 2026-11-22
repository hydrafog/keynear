package org.keynear.app.data

import androidx.compose.ui.graphics.Color
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.keynear.app.bridge.CoreBridge
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.CoreState
import org.keynear.app.ui.model.ShareValidationResult
import org.keynear.app.ui.model.VisibilityState
import org.keynear.app.bridge.validateShareIntent
import org.keynear.designsystem.DarkKeynearPalette
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.LightKeynearPalette

class CoreAndDesignTest {
    private class FakeCoreBridge : CoreBridge {
        private var current = VisibilityState.PAIRED_ONLY

        private fun snapshot(): CoreSnapshot = CoreSnapshot(
            version = "0.5.0",
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
        assertTrue(initial.snapshot.version.isNotBlank())
        assertEquals(VisibilityState.PAIRED_ONLY, initial.snapshot.visibility)
        assertTrue(initial.snapshot.maxControlFrameBytes > 0L)

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
        assertEquals(Color(0xFFF0F4FA), LightKeynearPalette.canvas)
        assertEquals(Color(0xFF0B1016), DarkKeynearPalette.canvas)
        assertEquals(Color(0xFF004CED), LightKeynearPalette.accent)
        assertEquals(Color(0xFF0172F9), LightKeynearPalette.focusRing)
        assertEquals(48, KeynearSpacing.MIN_TOUCH_TARGET_DP)
    }
}

