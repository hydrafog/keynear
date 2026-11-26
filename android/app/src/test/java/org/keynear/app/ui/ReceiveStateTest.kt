package org.keynear.app.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.DeviceRoleUi
import org.keynear.app.ui.model.DiscoveredPeerUi
import org.keynear.app.ui.model.PairedDeviceUi
import org.keynear.app.ui.model.VisibilityState
import org.keynear.app.ui.receive.ReceiveStateHolder

class ReceiveStateTest {
    @Test
    fun buildMapPinsDeduplicatesNearbyPeersAgainstPairedDevices() {
        val snapshot = snapshot(
            visibility = VisibilityState.PAIRED_ONLY,
            pairedDevices = listOf(
                paired("dev-1", "Studio Laptop", ble = "AA:BB:CC:DD:EE:FF", lan = "192.168.1.4"),
            ),
            nearbyPeers = listOf(
                peer("aa:bb:cc:dd:ee:ff", "Another Name"),
                peer("11:22:33:44:55:66", "  studio laptop  "),
                peer("99:88:77:66:55:44", "Fresh Phone"),
            ),
        )

        val pins = ReceiveStateHolder.buildMapPins(snapshot)

        assertEquals(2, pins.size)
        assertTrue(pins[0].isPaired)
        assertEquals("Wi-Fi", pins[0].statusLabel)
        assertFalse(pins[1].isPaired)
        assertEquals("Fresh Phone", pins[1].displayName)
        assertEquals("Tap to pair", pins[1].statusLabel)
    }

    @Test
    fun buildMapPinsGatesNearbyVisibilityByVisibilityState() {
        fun pinsFor(visibility: VisibilityState) = ReceiveStateHolder.buildMapPins(
            snapshot(visibility = visibility, nearbyPeers = listOf(peer("11:22:33:44:55:66", "Fresh Phone"))),
        ).single()

        assertTrue(pinsFor(VisibilityState.EVERYONE_NEARBY).isVisible)
        assertTrue(pinsFor(VisibilityState.PAIRED_ONLY).isVisible)
        assertFalse(pinsFor(VisibilityState.HIDDEN).isVisible)
    }

    @Test
    fun buildMapPinsPassesThroughMeasuredAndNullRssi() {
        val defaultPaired = PairedDeviceUi(
            deviceId = "dev-default",
            displayName = "Default",
            role = DeviceRoleUi.DESKTOP,
            publicKeyFingerprint = "a1b2c3d4e5f6",
            lastBleAddress = "AA:BB:CC:DD:EE:FF",
            pairedAt = "1731585600",
        )
        assertNull(defaultPaired.rssi)

        val withoutMeasurement = paired("dev-1", "Studio Laptop")
        val pins = ReceiveStateHolder.buildMapPins(
            snapshot(
                visibility = VisibilityState.EVERYONE_NEARBY,
                pairedDevices = listOf(
                    withoutMeasurement,
                    paired("dev-2", "Work Laptop", rssi = -63),
                ),
                nearbyPeers = listOf(peer("11:22:33:44:55:66", "Fresh Phone", rssi = -71)),
            ),
        )

        assertEquals(3, pins.size)
        assertNull(pins[0].rssi)
        assertEquals("Standby", pins[0].statusLabel)
        assertEquals(-63, pins[1].rssi)
        assertEquals(-71, pins[2].rssi)
    }

    private fun snapshot(
        visibility: VisibilityState,
        pairedDevices: List<PairedDeviceUi> = emptyList(),
        nearbyPeers: List<DiscoveredPeerUi> = emptyList(),
    ) = CoreSnapshot(
        version = "0.5.0",
        maxControlFrameBytes = 65_536L,
        ready = true,
        visibility = visibility,
        configSummary = "visibility=${visibility.label()}",
        pairedDevices = pairedDevices,
        nearbyPeers = nearbyPeers,
    )

    private fun paired(
        deviceId: String,
        displayName: String,
        ble: String = "AA:BB:CC:DD:EE:FF",
        lan: String = "",
        rssi: Int? = null,
    ) = PairedDeviceUi(
        deviceId = deviceId,
        displayName = displayName,
        role = DeviceRoleUi.DESKTOP,
        publicKeyFingerprint = "a1b2c3d4e5f6",
        lastBleAddress = ble,
        pairedAt = "1731585600",
        lastLanAddress = lan,
        rssi = rssi,
    )

    private fun peer(address: String, displayName: String, rssi: Int = -55) = DiscoveredPeerUi(
        address = address,
        displayName = displayName,
        role = DeviceRoleUi.MOBILE,
        rssi = rssi,
        capabilityFlags = 0L,
        isPaired = false,
    )
}
