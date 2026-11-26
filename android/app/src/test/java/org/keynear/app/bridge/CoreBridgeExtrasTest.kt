package org.keynear.app.bridge

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.keynear.app.ui.model.ShareValidationResult

class CoreBridgeExtrasTest {
    @Test
    fun normalizeClipboardMimeStripsParametersTrimsAndLowercases() {
        assertEquals("text/plain", normalizeClipboardMime("  Text/Plain; charset=utf-8  "))
        assertEquals("application/x-password", normalizeClipboardMime("APPLICATION/X-PASSWORD; version=2"))
        assertEquals("", normalizeClipboardMime("   "))
        assertEquals("", normalizeClipboardMime("; charset=utf-8"))
    }

    @Test
    fun validateShareIntentAcceptsUppercaseSchemesAndCombinedTextPayload() {
        assertEquals(
            ShareValidationResult.Valid(itemCount = 1, hasText = false),
            validateShareIntent("android.intent.action.SEND", listOf(" CONTENT://host/item/1 "), null),
        )
        assertEquals(
            ShareValidationResult.Valid(itemCount = 1, hasText = true),
            validateShareIntent(
                "android.intent.action.SEND_MULTIPLE",
                listOf("file:///sdcard/a.txt"),
                "  note  ",
            ),
        )
    }

    @Test
    fun validateShareIntentRejectsMissingSchemeAndMixedUriList() {
        assertEquals(
            ShareValidationResult.Invalid("missing_uri_scheme"),
            validateShareIntent("android.intent.action.SEND", listOf(":no-scheme"), null),
        )
        assertEquals(
            ShareValidationResult.Invalid("disallowed_uri_scheme"),
            validateShareIntent(
                "android.intent.action.SEND",
                listOf("content://ok/1", "http://bad/2"),
                null,
            ),
        )
        assertEquals(
            ShareValidationResult.Invalid("missing_payload"),
            validateShareIntent("android.intent.action.SEND_MULTIPLE", emptyList(), "  "),
        )
    }

    @Test
    fun formatByteCountOrdersNegativeZeroBytesAndUnitBoundaries() {
        assertEquals("0 B", formatByteCount(-5L))
        assertEquals("0 B", formatByteCount(0L))
        assertEquals("1023 B", formatByteCount(1023L))
        assertEquals("1.0 KB", formatByteCount(1024L))
        assertEquals("1.0 MB", formatByteCount(1024L * 1024L))
        assertEquals("1.00 GB", formatByteCount(1024L * 1024L * 1024L))
    }

    @Test
    fun consentRequirementCoversRemainingPairedCombinations() {
        assertFalse(evaluateConsentRequirement(isPaired = false, deviceAutoAccept = false, globalAutoAccept = false))
        assertTrue(evaluateConsentRequirement(isPaired = true, deviceAutoAccept = true, globalAutoAccept = true))
    }

    @Test
    fun endpointHexCodecRoundTripsAndRejectsMalformedHex() {
        val bytes = ByteArray(32) { (it * 3).toByte() }
        val hex = EndpointHexCodec.encode(bytes)
        assertEquals(64, hex.length)
        assertTrue(EndpointHexCodec.decode32(hex)!!.contentEquals(bytes))
        assertTrue(EndpointHexCodec.decode32(hex.uppercase())!!.contentEquals(bytes))
        assertTrue(EndpointHexCodec.decode32("  $hex  ")!!.contentEquals(bytes))
        assertEquals("", EndpointHexCodec.encode(ByteArray(0)))
        assertNull(EndpointHexCodec.decode32(hex + "0"))
        assertNull(EndpointHexCodec.decode32("G" + hex.drop(1)))
    }
}
