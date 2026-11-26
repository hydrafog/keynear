package org.keynear.app.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Test
import org.keynear.app.bridge.resolveDownloadMimeType
import org.keynear.app.share.DirectShareManager
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.CoreState
import org.keynear.app.ui.model.TransferProgressModel
import org.keynear.app.ui.model.TransferProgressStage
import org.keynear.app.ui.model.TransferStateUi
import org.keynear.app.ui.model.VisibilityState
import org.keynear.app.ui.model.computeTransferProgressModel
import org.keynear.app.ui.model.formatBytes
import org.keynear.app.ui.navigation.snapshotFromState

class BridgeHelpersTest {
    @Test
    fun resolveDownloadMimeTypePassesThroughExplicitMimeType() {
        assertEquals("image/png", resolveDownloadMimeType("photo.png", "image/png"))
        assertEquals("text/plain", resolveDownloadMimeType("notes.md", "  text/plain  "))
    }

    @Test
    fun resolveDownloadMimeTypeMapsKnownExtensionsForBlankAndOctetStream() {
        val extensionMimes = mapOf(
            "release.apk" to "application/vnd.android.package-archive",
            "notes.md" to "text/plain",
            "trace.log" to "text/plain",
            "readme.txt" to "text/plain",
            "payload.json" to "application/json",
            "manual.pdf" to "application/pdf",
        )
        for (rawMime in listOf("", "   ", "application/octet-stream")) {
            for ((fileName, expected) in extensionMimes) {
                assertEquals(expected, resolveDownloadMimeType(fileName, rawMime))
            }
        }
    }

    @Test
    fun extractDeviceIdStripsShortcutPrefixAndTrimsBothForms() {
        val prefix = DirectShareManager.SHORTCUT_ID_PREFIX
        assertEquals("dev-42", DirectShareManager.extractDeviceId("${prefix}dev-42"))
        assertEquals("dev-42", DirectShareManager.extractDeviceId("  ${prefix}  dev-42  "))
        assertEquals("dev-7", DirectShareManager.extractDeviceId("  dev-7  "))
    }

    @Test
    fun extractDeviceIdFallsBackToExplicitIdWhenShortcutIsBlank() {
        assertEquals("dev-9", DirectShareManager.extractDeviceId("   ", "  dev-9  "))
        assertEquals("dev-9", DirectShareManager.extractDeviceId(null, "dev-9"))
        assertNull(DirectShareManager.extractDeviceId("   ", "   "))
        assertNull(DirectShareManager.extractDeviceId(null, null))
        assertNull(
            DirectShareManager.extractDeviceId(
                "${DirectShareManager.SHORTCUT_ID_PREFIX}   ",
                "fallback-device",
            ),
        )
    }

    @Test
    fun snapshotFromStateReturnsExactSnapshotForEmptyAndReady() {
        val snapshot = snapshot()
        assertSame(snapshot, snapshotFromState(CoreState.Empty(snapshot)))
        assertSame(snapshot, snapshotFromState(CoreState.Ready(snapshot)))
    }

    @Test
    fun snapshotFromStateReturnsPreviousForErrorAndNullForLoading() {
        val snapshot = snapshot()
        assertSame(snapshot, snapshotFromState(CoreState.Error("boom", snapshot)))
        assertNull(snapshotFromState(CoreState.Error("boom")))
        assertNull(snapshotFromState(CoreState.Loading))
    }

    @Test
    fun computeTransferProgressUsesExplicitSdkIntForApi36Style() {
        assertEquals(
            TransferProgressModel(TransferProgressStage.COMPLETE, 100, 100, usesProgressStyleApi36 = false),
            computeTransferProgressModel(TransferStateUi.COMPLETED, 0L, 0L, sdkInt = 35),
        )
        assertEquals(
            TransferProgressModel(TransferProgressStage.COMPLETE, 100, 100, usesProgressStyleApi36 = true),
            computeTransferProgressModel(TransferStateUi.COMPLETED, 0L, 0L, sdkInt = 36),
        )
    }

    @Test
    fun computeTransferProgressMapsTerminalAndConsentStates() {
        assertEquals(
            TransferProgressModel(TransferProgressStage.PREPARE, 10, 10, usesProgressStyleApi36 = false),
            computeTransferProgressModel(TransferStateUi.PENDING_CONSENT, 0L, 100L, sdkInt = 35),
        )
        assertEquals(
            TransferProgressModel(TransferProgressStage.PREPARE, 0, 0, usesProgressStyleApi36 = true),
            computeTransferProgressModel(TransferStateUi.CANCELLED, 0L, 0L, sdkInt = 36),
        )
        assertEquals(
            TransferProgressModel(TransferProgressStage.PREPARE, 0, 0, usesProgressStyleApi36 = true),
            computeTransferProgressModel(TransferStateUi.FAILED, 0L, 0L, sdkInt = 36),
        )
    }

    @Test
    fun computeTransferProgressMapsActiveStatesByByteCounts() {
        assertEquals(
            TransferProgressModel(TransferProgressStage.PREPARE, 15, 15, usesProgressStyleApi36 = false),
            computeTransferProgressModel(TransferStateUi.ACTIVE, 0L, 1024L, sdkInt = 35),
        )
        assertEquals(
            TransferProgressModel(TransferProgressStage.TRANSFER, 50, 52, usesProgressStyleApi36 = true),
            computeTransferProgressModel(TransferStateUi.ACTIVE, 512L, 1024L, sdkInt = 36),
        )
        assertEquals(
            TransferProgressModel(TransferProgressStage.TRANSFER, 50, 52, usesProgressStyleApi36 = true),
            computeTransferProgressModel(TransferStateUi.ACTIVE, 512L, 0L, sdkInt = 36),
        )
    }

    @Test
    fun formatBytesHandlesZeroNegativeAndSubKilobyteValues() {
        assertEquals("0 B", formatBytes(0L))
        assertEquals("0 B", formatBytes(-1L))
        assertEquals("1 B", formatBytes(1L))
        assertEquals("1023 B", formatBytes(1023L))
    }

    @Test
    fun formatBytesFormatsKilobyteMegabyteAndGigabyteBoundaries() {
        assertEquals("1.0 KB", formatBytes(1024L))
        assertEquals("1.5 KB", formatBytes(1536L))
        assertEquals("1.0 MB", formatBytes(1024L * 1024L))
        assertEquals("1.0 GB", formatBytes(1024L * 1024L * 1024L))
    }

    private fun snapshot(): CoreSnapshot = CoreSnapshot(
        version = "0.5.0",
        maxControlFrameBytes = 65_536L,
        ready = true,
        visibility = VisibilityState.PAIRED_ONLY,
        configSummary = "visibility=PairedOnly",
    )
}
