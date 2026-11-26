package org.keynear.app.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.keynear.app.bridge.CoreBridge
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.CoreState
import org.keynear.app.ui.model.DaemonConfigUi
import org.keynear.app.ui.model.DeviceRoleUi
import org.keynear.app.ui.model.MAX_DISPLAY_NAME_LEN
import org.keynear.app.ui.model.PairedDeviceUi
import org.keynear.app.ui.model.PermissionsChecklistState
import org.keynear.app.ui.model.SendItemKind
import org.keynear.app.bridge.SendQueueStore
import org.keynear.app.ui.model.TransferDirectionUi
import org.keynear.app.ui.model.TransferProgressStage
import org.keynear.app.ui.model.TransferStateUi
import org.keynear.app.ui.model.TransferUi
import org.keynear.app.ui.model.VisibilityState
import org.keynear.app.ui.model.RadarLayout
import org.keynear.app.ui.model.computeTransferProgressModel
import org.keynear.app.ui.model.formatBytes
import org.keynear.app.ui.model.orbitPlacementsFor
import org.keynear.app.ui.model.radarLayoutFor
import org.keynear.app.ui.model.validateDisplayNameInput
import uniffi.keynear_core.FfiVisibilityMode
import org.keynear.app.service.TileTransitionDecision
import org.keynear.app.service.evaluateTileTransition
import org.keynear.app.share.DirectShareManager
import kotlin.math.abs

class SurfacesAndOnboardingTest {
    @Test
    fun tileTransitionsProtectEveryoneNearbyWhileLockedAndAllowSafeToggle() {
        assertEquals(
            VisibilityState.PAIRED_ONLY to TileTransitionDecision.APPLY_DIRECTLY,
            evaluateTileTransition(VisibilityState.HIDDEN, isDeviceLocked = true),
        )
        assertEquals(
            VisibilityState.EVERYONE_NEARBY to TileTransitionDecision.REQUIRE_UNLOCK,
            evaluateTileTransition(VisibilityState.PAIRED_ONLY, isDeviceLocked = true),
        )
        assertEquals(
            VisibilityState.PAIRED_ONLY to TileTransitionDecision.APPLY_DIRECTLY,
            evaluateTileTransition(VisibilityState.EVERYONE_NEARBY, isDeviceLocked = true),
        )

        assertEquals(
            VisibilityState.PAIRED_ONLY to TileTransitionDecision.APPLY_DIRECTLY,
            evaluateTileTransition(VisibilityState.HIDDEN, isDeviceLocked = false),
        )
        assertEquals(
            VisibilityState.EVERYONE_NEARBY to TileTransitionDecision.APPLY_DIRECTLY,
            evaluateTileTransition(VisibilityState.PAIRED_ONLY, isDeviceLocked = false),
        )
        assertEquals(
            VisibilityState.HIDDEN to TileTransitionDecision.APPLY_DIRECTLY,
            evaluateTileTransition(VisibilityState.EVERYONE_NEARBY, isDeviceLocked = false),
        )
    }

    @Test
    fun transferProgressModelUsesThreeSegmentsOnApi36AndCompatFallbackBefore() {
        val preparing = computeTransferProgressModel(
            state = TransferStateUi.PENDING_CONSENT,
            transferredBytes = 0L,
            totalBytes = 1_000L,
            sdkInt = 36,
        )
        assertEquals(TransferProgressStage.PREPARE, preparing.stage)
        assertTrue(preparing.usesProgressStyleApi36)
        assertEquals(listOf(20, 65, 15), preparing.segmentWeights)
        assertEquals(10, preparing.segmentedProgress)

        val transferringApi36 = computeTransferProgressModel(
            state = TransferStateUi.ACTIVE,
            transferredBytes = 500L,
            totalBytes = 1_000L,
            sdkInt = 36,
        )
        assertEquals(TransferProgressStage.TRANSFER, transferringApi36.stage)
        assertEquals(50, transferringApi36.overallPercent)
        assertEquals(52, transferringApi36.segmentedProgress)
        assertTrue(transferringApi36.usesProgressStyleApi36)

        val transferringApi34 = computeTransferProgressModel(
            state = TransferStateUi.ACTIVE,
            transferredBytes = 500L,
            totalBytes = 1_000L,
            sdkInt = 34,
        )
        assertEquals(TransferProgressStage.TRANSFER, transferringApi34.stage)
        assertEquals(50, transferringApi34.overallPercent)
        assertFalse(transferringApi34.usesProgressStyleApi36)

        val completedApi31 = computeTransferProgressModel(
            state = TransferStateUi.COMPLETED,
            transferredBytes = 1_000L,
            totalBytes = 1_000L,
            sdkInt = 31,
        )
        assertEquals(TransferProgressStage.COMPLETE, completedApi31.stage)
        assertEquals(100, completedApi31.overallPercent)
        assertEquals(100, completedApi31.segmentedProgress)
        assertFalse(completedApi31.usesProgressStyleApi36)
    }

    @Test
    fun directShareSpecsDeduplicateCapAndPruneDeletedDevices() {
        val paired = listOf(
            sampleDevice("dev-1", "Pixel 9"),
            sampleDevice("dev-1", "Pixel 9 Duplicate"),
            sampleDevice("dev-2", "Work Laptop"),
            sampleDevice("dev-3", "Media Box"),
            sampleDevice("dev-4", "Travel Phone"),
            sampleDevice("dev-5", "Extra Device"),
        )
        val specs = DirectShareManager.buildShortcutSpecs(paired, maxShortcuts = 4)
        assertEquals(4, specs.size)
        assertEquals("keynear_peer_dev-1", specs[0].shortcutId)
        assertEquals("Pixel 9", specs[0].shortLabel)
        assertEquals("Send to Work Laptop", specs[1].longLabel)
        assertEquals(3, specs[3].rank)

        val obsolete = DirectShareManager.diffObsoleteShortcutIds(
            existingShortcutIds = setOf(
                "keynear_peer_dev-1",
                "keynear_peer_unpaired",
                "non_keynear_shortcut",
            ),
            desiredSpecs = specs,
        )
        assertEquals(listOf("keynear_peer_unpaired"), obsolete)

        assertEquals("dev-2", DirectShareManager.extractDeviceId("keynear_peer_dev-2", null))
        assertEquals("dev-3", DirectShareManager.extractDeviceId(null, "dev-3"))
        assertNull(DirectShareManager.extractDeviceId("keynear_peer_", "  "))
    }

    @Test
    fun permissionsChecklistTracksRequiredAndRecommendedStates() {
        val requiredOnly = PermissionsChecklistState(
            nearbyGranted = true,
            notificationsGranted = true,
            batteryOptimizationIgnored = false,
        )
        assertTrue(requiredOnly.allRequiredGranted)
        assertFalse(requiredOnly.allRecommendedComplete)
        assertEquals(2, requiredOnly.completedCount)

        val missingNotifications = PermissionsChecklistState(
            nearbyGranted = true,
            notificationsGranted = false,
            batteryOptimizationIgnored = true,
        )
        assertFalse(missingNotifications.allRequiredGranted)
        assertFalse(missingNotifications.allRecommendedComplete)
        assertEquals(2, missingNotifications.completedCount)

        val fullyReady = PermissionsChecklistState(
            nearbyGranted = true,
            notificationsGranted = true,
            batteryOptimizationIgnored = true,
        )
        assertTrue(fullyReady.allRequiredGranted)
        assertTrue(fullyReady.allRecommendedComplete)
        assertEquals(3, fullyReady.completedCount)
    }

    @Test
    fun displayNameValidationAndBridgeRenameEnforceBounds() {
        assertTrue(validateDisplayNameInput("   ").isFailure)
        assertTrue(validateDisplayNameInput("a".repeat(MAX_DISPLAY_NAME_LEN + 1)).isFailure)
        assertTrue(validateDisplayNameInput("Bad\nName").isFailure)
        assertEquals(
            "Studio Desktop",
            validateDisplayNameInput("  Studio Desktop  ").getOrNull(),
        )

        var devices = listOf(sampleDevice("dev-1", "Old Name"))
        val bridge = object : CoreBridge {
            override fun loadState(): CoreState = CoreState.Ready(
                CoreSnapshot(
                    version = "0.5.0",
                    maxControlFrameBytes = 65_536L,
                    ready = true,
                    visibility = VisibilityState.PAIRED_ONLY,
                    configSummary = "visibility=PairedOnly",
                    pairedDevices = devices,
                ),
            )

            override fun cycleVisibility(): CoreState = loadState()

            override fun renamePairedDevice(deviceId: String, newDisplayName: String): CoreState {
                val valid = validateDisplayNameInput(newDisplayName).getOrElse { err ->
                    return CoreState.Error(err.message ?: "Invalid")
                }
                devices = devices.map {
                    if (it.deviceId == deviceId) it.copy(displayName = valid) else it
                }
                return loadState()
            }
        }

        val failed = bridge.renamePairedDevice("dev-1", "   ")
        assertTrue(failed is CoreState.Error)

        val renamed = bridge.renamePairedDevice("dev-1", "  New Studio Name ") as CoreState.Ready
        assertEquals("New Studio Name", renamed.snapshot.pairedDevices.single().displayName)
    }

    @Test
    fun orbitPlacementsUseThreeRingsAndShareEvenly() {
        val wide = radarLayoutFor(640f, 720f)

        val single = orbitPlacementsFor(listOf("a"), wide)
        assertEquals(155, single.single().radiusPx)
        assertEquals(40, single.single().durationSec)

        val two = orbitPlacementsFor(listOf("b", "a"), wide)
        assertEquals(listOf(138, 172), two.map { it.radiusPx }.sorted())

        val three = orbitPlacementsFor(listOf("a", "b", "c"), wide)
        assertEquals(listOf(138, 155, 172), three.map { it.radiusPx })
        assertEquals(0f, three[0].phaseSec, 0.001f)
        assertEquals(40f / 3f, three[1].phaseSec, 0.001f)
        assertEquals(80f / 3f, three[2].phaseSec, 0.001f)

        val five = orbitPlacementsFor(listOf("a", "b", "c", "d", "e"), wide)
        assertEquals(listOf(138, 155, 172, 138, 155), five.map { it.radiusPx })
        assertEquals(20f, five[3].phaseSec, 0.001f)
    }

    @Test
    fun orbitPlacementsForceMiddleRingIdsAndSpaceTheirPhasesEvenly() {
        val wide = radarLayoutFor(640f, 720f)

        val twoMiddle = orbitPlacementsFor(listOf("a", "b", "c"), wide, setOf("a", "b"))
        assertEquals(listOf(155, 155, 172), twoMiddle.map { it.radiusPx })
        assertEquals(20f, abs(twoMiddle[1].phaseSec - twoMiddle[0].phaseSec), 0.001f)

        val fourMiddle = orbitPlacementsFor(listOf("a", "b", "c", "d"), wide, setOf("a", "b", "c", "d"))
        assertEquals(listOf(155, 155, 155, 155), fourMiddle.map { it.radiusPx })
        assertEquals(10f, abs(fourMiddle[1].phaseSec - fourMiddle[0].phaseSec), 0.001f)
    }

    @Test
    fun orbitPlacementsStayStableWhenIdsReorder() {
        val wide = radarLayoutFor(640f, 720f)
        val sortedIds = listOf("a", "b", "c", "d", "e")
        val expected = sortedIds.zip(orbitPlacementsFor(sortedIds, wide)).toMap()

        val reversedIds = listOf("e", "d", "c", "b", "a")
        val reversed = orbitPlacementsFor(reversedIds, wide)
        reversedIds.forEachIndexed { idx, id ->
            val placement = expected.getValue(id)
            assertEquals(placement.radiusPx, reversed[idx].radiusPx)
            assertEquals(placement.phaseSec, reversed[idx].phaseSec, 0.001f)
        }
    }

    @Test
    fun orbitPlacementsHandleEmptyList() {
        assertTrue(orbitPlacementsFor(emptyList<String>(), radarLayoutFor(640f, 720f)).isEmpty())
    }

    @Test
    fun radarLayoutForGuardsZeroAndClampsNarrowAndWideViewports() {
        assertEquals(RadarLayout(scale = 1f, maxRadiusDp = 116f), radarLayoutFor(0f, 400f))
        assertEquals(RadarLayout(scale = 1f, maxRadiusDp = 116f), radarLayoutFor(360f, -12f))

        val narrow = radarLayoutFor(widthDp = 280f, heightDp = 300f)
        assertTrue(narrow.scale >= 0.62f && narrow.scale < 1f)
        assertTrue(narrow.maxRadiusDp in 116f..172f)

        val cramped = radarLayoutFor(widthDp = 120f, heightDp = 120f)
        assertEquals(0.62f, cramped.scale, 0.001f)
        assertEquals(116f, cramped.maxRadiusDp, 0.001f)

        val wide = radarLayoutFor(widthDp = 640f, heightDp = 720f)
        assertEquals(1f, wide.scale, 0.001f)
        assertEquals(172f, wide.maxRadiusDp, 0.001f)

        val ringPlacements = orbitPlacementsFor(listOf("a", "b", "c"), wide)
        assertEquals(listOf(138, 155, 172), ringPlacements.map { it.radiusPx })
        assertEquals(0f, ringPlacements[0].phaseSec, 0.001f)
        assertEquals(40f / 3f, ringPlacements[1].phaseSec, 0.001f)
        assertEquals(80f / 3f, ringPlacements[2].phaseSec, 0.001f)
    }

    @Test
    fun sendQueueStagesFileFolderClipboardAndTextAndSendsToPeer() {
        val store = SendQueueStore()
        var lastTarget: String? = null
        var lastCount = 0

        val bridge = object : CoreBridge {
            override fun loadState(): CoreState = CoreState.Ready(
                CoreSnapshot(
                    version = "0.5.0",
                    maxControlFrameBytes = 65_536L,
                    ready = true,
                    visibility = VisibilityState.PAIRED_ONLY,
                    configSummary = "visibility=PairedOnly",
                    pairedDevices = listOf(sampleDevice("dev-1", "Fedora Laptop")),
                    sendQueue = store.snapshot(),
                ),
            )

            override fun cycleVisibility(): CoreState = loadState()

            override fun stageFileUris(uris: List<String>): CoreState {
                uris.forEach { raw ->
                    store.addFile(raw.substringAfterLast('/'), 2048L, "application/pdf", raw)
                }
                return loadState()
            }

            override fun stageFolderUri(
                folderUri: String,
                displayName: String,
                sizeBytes: Long,
            ): CoreState {
                store.addFolder(displayName, folderUri, if (sizeBytes > 0L) sizeBytes else 4096L)
                return loadState()
            }

            override fun stageClipboardText(text: String): CoreState {
                store.addClipboard(text)
                return loadState()
            }

            override fun stageTextSnippet(text: String): CoreState {
                store.addTextSnippet(text)
                return loadState()
            }

            override fun removeSendQueueItem(itemId: String): CoreState {
                store.remove(itemId)
                return loadState()
            }

            override fun sendQueueToPeer(peerDeviceId: String): Result<TransferUi> {
                val staged = store.snapshot()
                lastTarget = peerDeviceId
                lastCount = staged.size
                store.clear()
                return Result.success(
                    TransferUi(
                        transferId = "tx-1",
                        peerDeviceId = peerDeviceId,
                        peerDisplayName = "Fedora Laptop",
                        summaryName = staged.first().name,
                        mimeType = staged.first().mime,
                        isDirectory = false,
                        direction = TransferDirectionUi.SEND,
                        totalBytes = staged.sumOf { it.sizeBytes },
                        transferredBytes = 0L,
                        state = TransferStateUi.ACTIVE,
                        startedAt = "1731585600",
                    ),
                )
            }
        }

        bridge.stageFileUris(listOf("content://docs/report.pdf"))
        bridge.stageFolderUri("content://tree/photos", "Photos")
        bridge.stageClipboardText("copied secret phrase")
        val stagedState = bridge.stageTextSnippet("quick note") as CoreState.Ready
        assertEquals(4, stagedState.snapshot.sendQueue.size)
        assertEquals(
            listOf(
                SendItemKind.FILE,
                SendItemKind.FOLDER,
                SendItemKind.CLIPBOARD,
                SendItemKind.TEXT,
            ),
            stagedState.snapshot.sendQueue.map { it.kind },
        )
        assertEquals("2.0 KB", formatBytes(2048L))

        val removed = bridge.removeSendQueueItem(stagedState.snapshot.sendQueue[1].id) as CoreState.Ready
        assertEquals(3, removed.snapshot.sendQueue.size)

        val result = bridge.sendQueueToPeer("dev-1")
        assertTrue(result.isSuccess)
        assertEquals("dev-1", lastTarget)
        assertEquals(3, lastCount)
        assertTrue((bridge.loadState() as CoreState.Ready).snapshot.sendQueue.isEmpty())
    }

    @Test
    fun daemonConfigPersistsAllPrototypeSettingsAndIgnoredMimeTypes() {
        val customConfig = DaemonConfigUi(
            incomingDirectory = "~/Public/Keynear",
            autoAcceptFromPaired = true,
            clipboardSyncEnabled = false,
            clipboardImagesEnabled = false,
            otpPassthroughEnabled = true,
            audioHandoffEnabled = true,
            ignoredMimeTypes = listOf("x-kde-passwordManagerHint", "application/x-custom"),
            notifyAutoAcceptComplete = true,
            darkTheme = true,
            accentColorHex = "#C594E9",
        )
        val ffi = customConfig.toFfi(VisibilityState.EVERYONE_NEARBY)
        assertEquals(FfiVisibilityMode.EVERYONE_NEARBY, ffi.visibilityMode)
        assertEquals("~/Public/Keynear", ffi.incomingDirectory)
        assertTrue(ffi.autoAcceptFromPaired)
        assertFalse(ffi.clipboardSyncEnabled)
        assertFalse(ffi.clipboardImagesEnabled)
        assertTrue(ffi.otpPassthroughEnabled)
        assertTrue(ffi.audioHandoffEnabled)
        assertEquals(listOf("x-kde-passwordManagerHint", "application/x-custom"), ffi.ignoredMimeTypes)
        assertTrue(ffi.notifyAutoAcceptComplete)

        val roundTripped = DaemonConfigUi.fromFfi(ffi, darkTheme = true, accentColorHex = "#C594E9")
        assertEquals(customConfig, roundTripped)

        val missingOne = PermissionsChecklistState(
            nearbyGranted = true,
            notificationsGranted = true,
            batteryOptimizationIgnored = true,
            localNetworkGranted = false,
        )
        assertEquals(1, missingOne.missingPermissionsCount)
        assertFalse(missingOne.allRequiredGranted)
    }

    private fun sampleDevice(deviceId: String, displayName: String): PairedDeviceUi =
        PairedDeviceUi(
            deviceId = deviceId,
            displayName = displayName,
            role = DeviceRoleUi.DESKTOP,
            publicKeyFingerprint = "a1b2c3d4e5f6",
            lastBleAddress = "AA:BB:CC:DD:EE:FF",
            pairedAt = "1731585600",
            autoAcceptFiles = false,
            lastEndpointId = "0123456789abcdef",
        )
}

