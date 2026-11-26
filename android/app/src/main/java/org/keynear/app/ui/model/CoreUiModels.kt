package org.keynear.app.ui.model


data class TransferUi(
    val transferId: String,
    val peerDeviceId: String,
    val peerDisplayName: String,
    val summaryName: String,
    val mimeType: String,
    val isDirectory: Boolean,
    val direction: TransferDirectionUi,
    val totalBytes: Long,
    val transferredBytes: Long,
    val state: TransferStateUi,
    val startedAt: String,
)

data class DiscoveredPeerUi(
    val address: String,
    val displayName: String,
    val role: DeviceRoleUi,
    val rssi: Int,
    val capabilityFlags: Long,
    val isPaired: Boolean,
    val batteryLevel: Int? = null,
)

data class PairedDeviceUi(
    val deviceId: String,
    val displayName: String,
    val role: DeviceRoleUi,
    val publicKeyFingerprint: String,
    val lastBleAddress: String,
    val pairedAt: String,
    val autoAcceptFiles: Boolean = false,
    val lastEndpointId: String = "",
    val lastLanAddress: String = "",
    val isOnline: Boolean = lastLanAddress.isNotBlank(),
    val rssi: Int? = null,
)

data class PendingPairingUi(
    val peerAddress: String,
    val peerName: String,
    val verificationCode: String,
    val remainingSecs: Long,
)

data class CoreSnapshot(
    val version: String,
    val maxControlFrameBytes: Long,
    val ready: Boolean,
    val visibility: VisibilityState,
    val configSummary: String,
    val everyoneRemainingSecs: Long = 0L,
    val pairedDevices: List<PairedDeviceUi> = emptyList(),
    val nearbyPeers: List<DiscoveredPeerUi> = emptyList(),
    val pendingPairing: PendingPairingUi? = null,
    val bluetoothStatus: BluetoothStatus = BluetoothStatus.READY,
    val warningMessage: String? = null,
    val localNetworkStatus: LocalNetworkStatus = LocalNetworkStatus.READY,
    val localNetworkMessage: String? = null,
    val activeTransfers: List<TransferUi> = emptyList(),
    val daemonConfig: DaemonConfigUi = DaemonConfigUi(),
    val sendQueue: List<SendQueueItemUi> = emptyList(),
)

sealed interface CoreState {
    data object Loading : CoreState
    data class Empty(val snapshot: CoreSnapshot) : CoreState
    data class Ready(val snapshot: CoreSnapshot) : CoreState
    data class Error(val message: String, val previous: CoreSnapshot? = null) : CoreState
}

sealed interface ShareValidationResult {
    data class Valid(val itemCount: Int, val hasText: Boolean) : ShareValidationResult
    data class Invalid(val reason: String) : ShareValidationResult
}
