package org.keynear.app.ui.navigation

import org.keynear.app.ui.model.DeviceRoleUi
import org.keynear.app.ui.model.PairedDeviceUi

enum class KeynearTab(val label: String) {
    RECEIVE("Receive"),
    SEND("Send"),
    SETTINGS("Settings"),
}

sealed interface KeynearSubScreen {
    data object Permissions : KeynearSubScreen
    data class DeviceDetail(val deviceId: String) : KeynearSubScreen
}

data class OrbitPinUi(
    val id: String,
    val displayName: String,
    val statusLabel: String,
    val role: DeviceRoleUi,
    val rssi: Int?,
    val isPaired: Boolean,
    val isOnline: Boolean,
    val peerAddress: String,
    val isVisible: Boolean = true,
)

data class KeynearActions(
    val onSetVisibility: (org.keynear.app.ui.model.VisibilityState) -> Unit,
    val onRequestBluetoothPermissions: () -> Unit,
    val onOpenBluetoothSettings: () -> Unit,
    val onRequestLocalNetworkPermission: () -> Unit,
    val onRequestNearbyOnboardingPermissions: () -> Unit,
    val onRequestNotificationsPermission: () -> Unit,
    val onRequestBatteryOptimization: () -> Unit,
    val onOpenAppSettings: () -> Unit,
    val onInitiatePairing: (String) -> Unit,
    val onApprovePairing: () -> Unit,
    val onRejectPairing: () -> Unit,
    val onToggleAutoAccept: (String, Boolean) -> Unit,
    val onRenameDevice: (String, String) -> Unit,
    val onResetBleBond: (String) -> Unit,
    val onAcceptTransfer: (String) -> Unit,
    val onDeclineTransfer: (String) -> Unit,
    val onCancelTransfer: (String) -> Unit,
    val onRequestUnpair: (String) -> Unit,
    val onCancelUnpair: () -> Unit,
    val onConfirmUnpair: (String) -> Unit,
    val onPickFiles: () -> Unit,
    val onPickFolder: () -> Unit,
    val onStageClipboard: () -> Unit,
    val onStageTextSnippet: (String) -> Unit,
    val onRemoveQueueItem: (String) -> Unit,
    val onClearQueue: () -> Unit,
    val onSendQueueToPeer: (PairedDeviceUi) -> Unit,
    val onUpdateConfig: ((org.keynear.app.ui.model.DaemonConfigUi) -> org.keynear.app.ui.model.DaemonConfigUi) -> Unit,
    val onPickSaveFolder: () -> Unit,
    val onOpenPermissions: () -> Unit,
    val onOpenDeviceDetail: (String) -> Unit,
    val onJumpToSendTab: () -> Unit,
)
