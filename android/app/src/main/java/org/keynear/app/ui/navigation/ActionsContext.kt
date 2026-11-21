package org.keynear.app.ui.navigation

import android.Manifest
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.keynear.app.ble.KeynearBleConstants
import org.keynear.app.ble.BleScanner
import org.keynear.app.ble.GattSession
import org.keynear.app.bridge.CoreBridge
import org.keynear.app.ui.model.CoreState
import org.keynear.app.ui.model.DaemonConfigUi
import org.keynear.app.ui.model.PairedDeviceUi
import org.keynear.app.ui.model.VisibilityState
import org.keynear.app.data.LocalNetworkConstants
import org.keynear.app.data.LanManager
import org.keynear.app.service.KeynearForegroundService

internal data class ActionsContext(
    val context: Context,
    val scope: CoroutineScope,
    val bridge: CoreBridge,
    val onStateUpdated: (CoreState) -> Unit,
    val onShowSnackbar: (String) -> Unit,
    val onLaunchPermissions: (Array<String>) -> Unit,
    val onNavigateTab: (KeynearTab) -> Unit,
    val onCloseSubScreen: () -> Unit,
    val onCancelUnpair: () -> Unit,
)

private fun visibilityLabel(vis: VisibilityState): String = when (vis) {
    VisibilityState.HIDDEN -> "Hidden"
    VisibilityState.PAIRED_ONLY -> "Paired devices only"
    VisibilityState.EVERYONE_NEARBY -> "Everyone nearby (10 min)"
}

internal fun ActionsContext.setVisibility(mode: VisibilityState) {
    scope.launch {
        val updated = withContext(Dispatchers.IO) { bridge.setVisibility(mode) }
        val vis = snapshotFromState(updated)?.visibility ?: mode
        withContext(Dispatchers.IO) {
            BleScanner.startOrSync(context, vis)
            KeynearForegroundService.syncIfPermitted(context)
        }
        onStateUpdated(withContext(Dispatchers.IO) { bridge.loadState() })
        onShowSnackbar("Visibility: ${visibilityLabel(vis)}")
    }
}

internal fun ActionsContext.requestBluetoothPermissions() {
    val missing = KeynearBleConstants.missingPermissions(context, Build.VERSION.SDK_INT)
    if (missing.isNotEmpty()) {
        onLaunchPermissions(missing.toTypedArray())
    }
}

internal fun ActionsContext.openBluetoothSettings() {
    launchFirstResolvableActivity(context, listOf(Intent(Settings.ACTION_BLUETOOTH_SETTINGS)))
}

internal fun ActionsContext.requestLocalNetworkPermission() {
    if (LocalNetworkConstants.requiresLocalNetworkPermission(Build.VERSION.SDK_INT)) {
        onLaunchPermissions(arrayOf(LocalNetworkConstants.PERMISSION_ACCESS_LOCAL_NETWORK))
    }
    scope.launch(Dispatchers.IO) {
        LanManager.requestPickerDiscovery(context)
        onStateUpdated(bridge.loadState())
    }
}

internal fun ActionsContext.requestNearbyOnboardingPermissions() {
    val missing = KeynearBleConstants.missingPermissions(context, Build.VERSION.SDK_INT)
        .toMutableList()
    if (Build.VERSION.SDK_INT >= 33) {
        missing.add(Manifest.permission.NEARBY_WIFI_DEVICES)
    }
    if (missing.isNotEmpty()) {
        onLaunchPermissions(missing.distinct().toTypedArray())
    } else {
        launchFirstResolvableActivity(
            context,
            listOf(
                Intent(
                    Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                    Uri.fromParts("package", context.packageName, null),
                ),
            ),
        )
    }
}

internal fun ActionsContext.requestNotificationsPermission() {
    if (Build.VERSION.SDK_INT >= 33) {
        onLaunchPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS))
    } else {
        launchFirstResolvableActivity(
            context,
            listOf(
                Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS).apply {
                    putExtra(Settings.EXTRA_APP_PACKAGE, context.packageName)
                },
                Intent(
                    Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                    Uri.fromParts("package", context.packageName, null),
                ),
            ),
        )
    }
}

internal fun ActionsContext.requestBatteryOptimization() {
    val pkgUri = Uri.parse("package:${context.packageName}")
    launchFirstResolvableActivity(
        context,
        listOf(
            Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS, pkgUri),
            Intent(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS),
            Intent(
                Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                Uri.fromParts("package", context.packageName, null),
            ),
        ),
    )
}

internal fun ActionsContext.openAppSettings() {
    launchFirstResolvableActivity(
        context,
        listOf(
            Intent(
                Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                Uri.fromParts("package", context.packageName, null),
            ),
        ),
    )
}

internal fun ActionsContext.initiatePairing(addr: String) {
    scope.launch {
        withContext(Dispatchers.IO) { GattSession.initiatePairing(context, addr) }
        onStateUpdated(withContext(Dispatchers.IO) { bridge.loadState() })
        onShowSnackbar("Pairing requested")
    }
}

internal fun ActionsContext.resolvePairing(approve: Boolean) {
    scope.launch {
        withContext(Dispatchers.IO) {
            if (approve) {
                GattSession.approveActivePairing(context)
            } else {
                GattSession.rejectActivePairing(context)
            }
        }
        onStateUpdated(withContext(Dispatchers.IO) { bridge.loadState() })
        onShowSnackbar(if (approve) "Device paired" else "Pairing declined")
    }
}

internal fun ActionsContext.toggleAutoAccept(deviceId: String, enabled: Boolean) {
    scope.launch {
        onStateUpdated(withContext(Dispatchers.IO) { bridge.setAutoAcceptFiles(deviceId, enabled) })
        onShowSnackbar(if (enabled) "Auto-accept enabled" else "Auto-accept disabled")
    }
}

internal fun ActionsContext.renameDevice(deviceId: String, newName: String) {
    scope.launch {
        onStateUpdated(withContext(Dispatchers.IO) { bridge.renamePairedDevice(deviceId, newName) })
        onShowSnackbar("Renamed to ${newName.trim()}")
    }
}

internal fun ActionsContext.resetBleBond(deviceId: String) {
    scope.launch {
        onStateUpdated(withContext(Dispatchers.IO) { bridge.resetBleBond(deviceId) })
        onShowSnackbar("Bluetooth bond cleared")
    }
}

internal fun ActionsContext.changeTransfer(transferId: String, operation: (String) -> CoreState) {
    scope.launch {
        onStateUpdated(withContext(Dispatchers.IO) { operation(transferId) })
    }
}

internal fun ActionsContext.confirmUnpair(deviceId: String) {
    onCancelUnpair()
    onCloseSubScreen()
    scope.launch {
        onStateUpdated(withContext(Dispatchers.IO) { bridge.unpairDevice(deviceId) })
        onShowSnackbar("Device removed")
    }
}

internal fun ActionsContext.stageClipboard() {
    val clipboard = context.getSystemService(ClipboardManager::class.java)
    val clipText = clipboard?.primaryClip
        ?.takeIf { it.itemCount > 0 }
        ?.getItemAt(0)
        ?.coerceToText(context)
        ?.toString()
        ?.trim()
        .orEmpty()
    if (clipText.isEmpty()) {
        onShowSnackbar("Clipboard has no text item")
    } else {
        scope.launch {
            onStateUpdated(withContext(Dispatchers.IO) { bridge.stageClipboardText(clipText) })
            onShowSnackbar("Clipboard snippet staged")
        }
    }
}

internal fun ActionsContext.stageTextSnippet(draft: String) {
    scope.launch {
        onStateUpdated(withContext(Dispatchers.IO) { bridge.stageTextSnippet(draft) })
        onShowSnackbar("Text snippet staged")
    }
}

internal fun ActionsContext.changeQueue(operation: (CoreBridge) -> CoreState) {
    scope.launch {
        onStateUpdated(withContext(Dispatchers.IO) { operation(bridge) })
    }
}

internal fun ActionsContext.sendQueueToPeer(device: PairedDeviceUi) {
    scope.launch {
        val res = withContext(Dispatchers.IO) { bridge.sendQueueToPeer(device.deviceId) }
        onStateUpdated(withContext(Dispatchers.IO) { bridge.loadState() })
        res.onSuccess {
            onNavigateTab(KeynearTab.RECEIVE)
            onShowSnackbar("Sending to ${device.displayName}")
        }.onFailure { err ->
            onShowSnackbar(err.message ?: "Unable to start transfer")
        }
    }
}

internal fun ActionsContext.updateConfig(transform: (DaemonConfigUi) -> DaemonConfigUi) {
    scope.launch {
        onStateUpdated(withContext(Dispatchers.IO) { bridge.updateDaemonConfig(transform) })
    }
}

internal fun ActionsContext.jumpToSendTab() {
    onCloseSubScreen()
    onNavigateTab(KeynearTab.SEND)
}
