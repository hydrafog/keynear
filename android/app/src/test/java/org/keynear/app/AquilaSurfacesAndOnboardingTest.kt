package org.keynear.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.keynear.app.core.CoreBridge
import org.keynear.app.core.CoreSnapshot
import org.keynear.app.core.CoreState
import org.keynear.app.core.DeviceRoleUi
import org.keynear.app.core.MAX_DISPLAY_NAME_LEN
import org.keynear.app.core.PairedDeviceUi
import org.keynear.app.core.PermissionsChecklistState
import org.keynear.app.core.TransferProgressStage
import org.keynear.app.core.TransferStateUi
import org.keynear.app.core.VisibilityState
import org.keynear.app.core.computeTransferProgressModel
import org.keynear.app.core.validateDisplayNameInput
import org.keynear.app.service.TileTransitionDecision
import org.keynear.app.service.evaluateTileTransition
import org.keynear.app.share.DirectShareManager

class AquilaSurfacesAndOnboardingTest {
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
            companionAssociated = false,
        )
        assertTrue(requiredOnly.allRequiredGranted)
        assertFalse(requiredOnly.allRecommendedComplete)
        assertEquals(2, requiredOnly.completedCount)

        val missingNotifications = PermissionsChecklistState(
            nearbyGranted = true,
            notificationsGranted = false,
            batteryOptimizationIgnored = true,
            companionAssociated = true,
        )
        assertFalse(missingNotifications.allRequiredGranted)
        assertFalse(missingNotifications.allRecommendedComplete)
        assertEquals(3, missingNotifications.completedCount)

        val fullyReady = PermissionsChecklistState(
            nearbyGranted = true,
            notificationsGranted = true,
            batteryOptimizationIgnored = true,
            companionAssociated = false,
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
                    version = "0.4.0",
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

