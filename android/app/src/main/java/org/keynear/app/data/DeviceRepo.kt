package org.keynear.app.data

import android.content.Context
import org.keynear.app.bridge.EndpointHexCodec
import org.keynear.app.bridge.KeystoreSecretStore
import org.keynear.app.ui.model.DeviceRoleUi
import org.keynear.app.ui.model.PairedDeviceUi
import uniffi.keynear_core.CoreHandle
import uniffi.keynear_core.FfiPairedDevice
import uniffi.keynear_core.OrionNodeHandle
import java.util.UUID

object DeviceRepo {
    private const val DEVICE_IDENTITY_PREFS = "keynear_device_identity"
    private const val KEY_DEVICE_ID = "device_id"
    private const val GENERATED_ID_SUFFIX_LEN = 12
    private const val FINGERPRINT_HEX_LEN = 12
    private const val RSSI_ONLINE = -52
    private const val RSSI_STANDBY = -76

    fun resolveStableLocalDeviceId(context: Context): String {
        val prefs = context.getSharedPreferences(DEVICE_IDENTITY_PREFS, Context.MODE_PRIVATE)
        val existing = prefs.getString(KEY_DEVICE_ID, null)
        if (!existing.isNullOrBlank()) {
            return existing
        }
        val generated = "android-${UUID.randomUUID().toString().take(GENERATED_ID_SUFFIX_LEN)}"
        prefs.edit().putString(KEY_DEVICE_ID, generated).apply()
        return generated
    }

    fun getPairedDevices(handle: CoreHandle, devicesPath: String?): List<FfiPairedDevice> {
        val path = devicesPath ?: return emptyList()
        return runCatching { handle.loadPairedDevices(path) }.getOrDefault(emptyList())
    }

    fun mapPairedDevices(rawPaired: List<FfiPairedDevice>): List<PairedDeviceUi> =
        rawPaired.map { dev ->
            val online = dev.lastLanAddress.isNotBlank()
            PairedDeviceUi(
                deviceId = dev.deviceId,
                displayName = dev.displayName,
                role = DeviceRoleUi.fromFfi(dev.deviceRole),
                publicKeyFingerprint = dev.staticX25519Pubkey.take(FINGERPRINT_HEX_LEN),
                lastBleAddress = dev.lastBleAddress,
                pairedAt = dev.pairedAt,
                autoAcceptFiles = dev.autoAcceptFiles,
                lastEndpointId = dev.lastEndpointId,
                lastLanAddress = dev.lastLanAddress,
                isOnline = online,
                rssi = if (online) RSSI_ONLINE else RSSI_STANDBY,
            )
        }

    fun syncAllPairedPeersToOrion(
        handle: CoreHandle,
        devicesPath: String?,
        secretStore: KeystoreSecretStore?,
        node: OrionNodeHandle?,
    ) {
        val activeNode = node ?: return
        val store = secretStore ?: return
        for (dev in getPairedDevices(handle, devicesPath)) {
            val eidBytes = EndpointHexCodec.decode32(dev.lastEndpointId) ?: continue
            val secrets = store.getPeerSecrets(dev.deviceId) ?: continue
            try {
                val addrs = if (dev.lastLanAddress.isBlank()) emptyList() else listOf(dev.lastLanAddress)
                runCatching {
                    activeNode.syncPeer(
                        deviceId = dev.deviceId,
                        endpointId = eidBytes,
                        socketAddrs = addrs,
                        reconnectSecret = secrets.reconnectSecret,
                        autoAcceptFiles = dev.autoAcceptFiles,
                    )
                }
            } finally {
                secrets.zeroize()
            }
        }
    }
}

