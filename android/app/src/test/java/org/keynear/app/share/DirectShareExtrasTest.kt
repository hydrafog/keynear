package org.keynear.app.share

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.keynear.app.ui.model.DeviceRoleUi
import org.keynear.app.ui.model.PairedDeviceUi

class DirectShareExtrasTest {
    private fun device(deviceId: String, displayName: String) = PairedDeviceUi(
        deviceId = deviceId,
        displayName = displayName,
        role = DeviceRoleUi.DESKTOP,
        publicKeyFingerprint = "a1b2c3d4e5f6",
        lastBleAddress = "AA:BB:CC:DD:EE:FF",
        pairedAt = "1731585600",
    )

    @Test
    fun shortcutIdForDeviceUsesStablePrefix() {
        assertEquals("keynear_peer_dev-7", DirectShareManager.shortcutIdForDevice("dev-7"))
        assertEquals("keynear_peer_", DirectShareManager.shortcutIdForDevice(""))
    }

    @Test
    fun buildShortcutSpecsHonorsZeroAndNegativeLimits() {
        assertTrue(DirectShareManager.buildShortcutSpecs(listOf(device("a", "A")), maxShortcuts = 0).isEmpty())
        assertTrue(DirectShareManager.buildShortcutSpecs(listOf(device("a", "A")), maxShortcuts = -2).isEmpty())
        assertTrue(DirectShareManager.buildShortcutSpecs(emptyList()).isEmpty())
    }

    @Test
    fun buildShortcutSpecsTrimsIdsAndCapsLabels() {
        val longName = "L".repeat(60)
        val spec = DirectShareManager.buildShortcutSpecs(listOf(device("  dev-9  ", longName))).single()
        assertEquals("dev-9", spec.deviceId)
        assertEquals("keynear_peer_dev-9", spec.shortcutId)
        assertEquals("L".repeat(25), spec.shortLabel)
        assertEquals(("Send to $longName").take(60), spec.longLabel)
        assertEquals(60, spec.longLabel.length)

        val blankNamed = DirectShareManager.buildShortcutSpecs(listOf(device("dev-3", "   "))).single()
        assertEquals("dev-3", blankNamed.shortLabel)
    }

    @Test
    fun diffObsoleteShortcutIdsHandlesEmptyDesiredAndKeepsNonKeynearIds() {
        val existing = setOf("keynear_peer_a", "keynear_peer_b", "other_c")
        assertEquals(
            listOf("keynear_peer_a", "keynear_peer_b"),
            DirectShareManager.diffObsoleteShortcutIds(existing, emptyList()),
        )
        assertTrue(DirectShareManager.diffObsoleteShortcutIds(emptySet(), emptyList()).isEmpty())
    }
}
