package org.keynear.app.ui.navigation

import android.content.Context
import kotlinx.coroutines.CoroutineScope
import org.keynear.app.bridge.CoreBridge
import org.keynear.app.ui.model.CoreState

fun buildKeynearActions(
    context: Context,
    scope: CoroutineScope,
    bridge: CoreBridge,
    onStateUpdated: (CoreState) -> Unit,
    onShowSnackbar: (String) -> Unit,
    onLaunchPermissions: (Array<String>) -> Unit,
    onPickFiles: () -> Unit,
    onPickFolder: () -> Unit,
    onPickSaveFolder: () -> Unit,
    onRequestUnpair: (String) -> Unit,
    onCancelUnpair: () -> Unit,
    onOpenPermissions: () -> Unit,
    onOpenDeviceDetail: (String) -> Unit,
    onNavigateTab: (KeynearTab) -> Unit,
    onCloseSubScreen: () -> Unit,
): KeynearActions {
    val ctx = ActionsContext(
        context = context,
        scope = scope,
        bridge = bridge,
        onStateUpdated = onStateUpdated,
        onShowSnackbar = onShowSnackbar,
        onLaunchPermissions = onLaunchPermissions,
        onNavigateTab = onNavigateTab,
        onCloseSubScreen = onCloseSubScreen,
        onCancelUnpair = onCancelUnpair,
    )
    return KeynearActions(
        onSetVisibility = { ctx.setVisibility(it) },
        onRequestBluetoothPermissions = { ctx.requestBluetoothPermissions() },
        onOpenBluetoothSettings = { ctx.openBluetoothSettings() },
        onRequestLocalNetworkPermission = { ctx.requestLocalNetworkPermission() },
        onRequestNearbyOnboardingPermissions = { ctx.requestNearbyOnboardingPermissions() },
        onRequestNotificationsPermission = { ctx.requestNotificationsPermission() },
        onRequestBatteryOptimization = { ctx.requestBatteryOptimization() },
        onOpenAppSettings = { ctx.openAppSettings() },
        onInitiatePairing = { ctx.initiatePairing(it) },
        onApprovePairing = { ctx.resolvePairing(true) },
        onRejectPairing = { ctx.resolvePairing(false) },
        onToggleAutoAccept = { deviceId, enabled -> ctx.toggleAutoAccept(deviceId, enabled) },
        onRenameDevice = { deviceId, newName -> ctx.renameDevice(deviceId, newName) },
        onResetBleBond = { ctx.resetBleBond(it) },
        onAcceptTransfer = { ctx.changeTransfer(it, bridge::acceptTransfer) },
        onDeclineTransfer = { ctx.changeTransfer(it, bridge::declineTransfer) },
        onCancelTransfer = { ctx.changeTransfer(it, bridge::cancelTransfer) },
        onRequestUnpair = onRequestUnpair,
        onCancelUnpair = onCancelUnpair,
        onConfirmUnpair = { ctx.confirmUnpair(it) },
        onPickFiles = onPickFiles,
        onPickFolder = onPickFolder,
        onStageClipboard = { ctx.stageClipboard() },
        onStageTextSnippet = { ctx.stageTextSnippet(it) },
        onRemoveQueueItem = { id -> ctx.changeQueue { bridge.removeSendQueueItem(id) } },
        onClearQueue = { ctx.changeQueue { bridge.clearSendQueue() } },
        onSendQueueToPeer = { ctx.sendQueueToPeer(it) },
        onUpdateConfig = { ctx.updateConfig(it) },
        onPickSaveFolder = onPickSaveFolder,
        onOpenPermissions = onOpenPermissions,
        onOpenDeviceDetail = onOpenDeviceDetail,
        onJumpToSendTab = { ctx.jumpToSendTab() },
    )
}
