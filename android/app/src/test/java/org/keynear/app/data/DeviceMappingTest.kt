package org.keynear.app.data

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.keynear.app.ui.model.DeviceRoleUi
import uniffi.keynear_core.FfiDeviceRole
import uniffi.keynear_core.FfiPairedDevice

class DeviceMappingTest {
    private fun ffi(
        deviceId: String = "dev-1",
        lan: String = "",
        role: FfiDeviceRole = FfiDeviceRole.DESKTOP,
        pubkey: String = "a1b2c3d4e5f6FF00",
    ) = FfiPairedDevice(
        deviceId = deviceId,
        displayName = "Studio Laptop",
        deviceRole = role,
        staticX25519Pubkey = pubkey,
        lastBleAddress = "AA:BB:CC:DD:EE:FF",
        lastLanAddress = lan,
        lastEndpointId = "0123456789abcdef",
        capabilityFlags = 0u,
        autoAcceptFiles = true,
        pairedAt = "1731585600",
    )

    @Test
    fun mapPairedDevicesDerivesOnlineFromLanAndNeverFabricatesRssi() {
        val mapped = DeviceRepo.mapPairedDevices(
            listOf(ffi(lan = "192.168.1.4"), ffi(deviceId = "dev-2", lan = "   ")),
        )
        assertEquals(2, mapped.size)
        assertTrue(mapped[0].isOnline)
        assertFalse(mapped[1].isOnline)
        assertNull(mapped[0].rssi)
        assertNull(mapped[1].rssi)
        assertEquals(DeviceRoleUi.DESKTOP, mapped[0].role)
        assertTrue(DeviceRepo.mapPairedDevices(emptyList()).isEmpty())
    }

    @Test
    fun mapPairedDevicesMapsRoleFingerprintAndPassThroughFields() {
        val mapped = DeviceRepo.mapPairedDevices(
            listOf(ffi(role = FfiDeviceRole.MOBILE, pubkey = "abcdef1234567890")),
        ).single()
        assertEquals(DeviceRoleUi.MOBILE, mapped.role)
        assertEquals("abcdef123456", mapped.publicKeyFingerprint)
        assertEquals("0123456789abcdef", mapped.lastEndpointId)
        assertTrue(mapped.autoAcceptFiles)
        assertEquals("AA:BB:CC:DD:EE:FF", mapped.lastBleAddress)
        assertEquals("1731585600", mapped.pairedAt)
        assertEquals("Studio Laptop", mapped.displayName)
    }
}
