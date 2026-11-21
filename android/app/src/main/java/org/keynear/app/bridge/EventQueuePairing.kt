package org.keynear.app.bridge

import android.content.ClipData
import android.content.ClipboardManager
import android.content.ContentValues
import android.content.Context
import android.net.Uri
import android.os.Environment
import android.os.Handler
import android.os.Looper
import android.provider.MediaStore
import android.util.Log
import org.keynear.app.ble.KeynearBleConstants
import org.keynear.app.ble.PairingNotifier
import org.keynear.app.data.DeviceRepo
import org.keynear.app.data.PairingRepo
import org.keynear.app.data.TransferRepo
import org.keynear.app.share.DirectShareManager
import org.keynear.app.ui.model.BluetoothStatus
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.CoreState
import org.keynear.app.ui.model.DaemonConfigUi
import org.keynear.app.ui.model.DiscoveredPeerUi
import org.keynear.app.ui.model.LocalNetworkStatus
import org.keynear.app.ui.model.PendingPairingUi
import org.keynear.app.ui.model.SendItemKind
import org.keynear.app.ui.model.TransferStateUi
import org.keynear.app.ui.model.TransferUi
import org.keynear.app.ui.model.VisibilityState
import org.keynear.app.ui.model.validateDisplayNameInput
import uniffi.keynear_core.CoreHandle
import uniffi.keynear_core.FfiClipboardItem
import uniffi.keynear_core.FfiDeviceRole
import uniffi.keynear_core.FfiLanEndpoint
import uniffi.keynear_core.FfiPairedDevice
import uniffi.keynear_core.OrionNodeHandle
import uniffi.keynear_core.defaultConfigSummary
import uniffi.keynear_core.everyoneNearbyTimeoutSecs
import java.io.File
import java.util.Locale
import java.util.concurrent.ConcurrentHashMap

internal fun EventQueue.localLanEndpoint(): FfiLanEndpoint? = orionNode?.localLanEndpoint()

fun syncAllPairedPeersToOrion(node: OrionNodeHandle? = orionNode) {
    DeviceRepo.syncAllPairedPeersToOrion(handle, devicesPath, secretStore, node)
}

fun encodeLocalLanEndpointForBleAddress(bleAddress: String): ByteArray? {
    val dev = getPairedDevices().firstOrNull { it.lastBleAddress.equals(bleAddress, ignoreCase = true) }
        ?: return null
    return encodeLocalLanEndpointForPeer(dev.deviceId)
}
internal fun EventQueue.encodeLocalLanEndpointForPeer(deviceId: String): ByteArray? =
    PairingRepo.encodeLocalLanEndpointForPeer(handle, orionNode, secretStore, deviceId)
internal fun EventQueue.onEncryptedLanEndpointFromBle(encoded: ByteArray, bleAddress: String? = null): FfiLanEndpoint? {
    val store = secretStore ?: return null
    val decoded = PairingRepo.decodeEncryptedLanEndpointFromBle(handle, store, getPairedDevices(), encoded, bleAddress)
    if (decoded != null) {
        onPeerLanEndpointDiscovered(decoded.deviceId, decoded.endpointId, decoded.socketAddrs)
    }
    return decoded
}
internal fun EventQueue.onPeerLanEndpointDiscovered(
    deviceId: String,
    endpointId: ByteArray,
    socketAddrs: List<String>,
) {
    PairingRepo.onPeerLanEndpointDiscovered(
        handle = handle,
        devicesPath = devicesPath,
        orionNode = orionNode,
        store = secretStore,
        devices = getPairedDevices(),
        deviceId = deviceId,
        endpointId = endpointId,
        socketAddrs = socketAddrs,
    )
}
internal fun EventQueue.saveCompletedPairing(
    peerDeviceId: String,
    peerDisplayName: String,
    peerRole: FfiDeviceRole,
    peerStaticPubkeyHex: String,
    peerCapabilityFlags: UInt,
    bleAddress: String,
    reconnectSecret: ByteArray,
) {
    PairingRepo.saveCompletedPairing(
        handle = handle,
        devicesPath = devicesPath,
        secretStore = secretStore,
        peerDeviceId = peerDeviceId,
        peerDisplayName = peerDisplayName,
        peerRole = peerRole,
        peerStaticPubkeyHex = peerStaticPubkeyHex,
        peerCapabilityFlags = peerCapabilityFlags,
        bleAddress = bleAddress,
        reconnectSecret = reconnectSecret,
    )
}
