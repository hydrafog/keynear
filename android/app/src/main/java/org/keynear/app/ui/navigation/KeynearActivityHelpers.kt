package org.keynear.app.ui.navigation

import android.Manifest
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.DocumentsContract
import android.provider.Settings
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.keynear.app.ble.KeynearBleConstants
import org.keynear.app.ble.BleScanner
import org.keynear.app.ble.GattSession
import org.keynear.app.bridge.CoreBridge
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.CoreState
import org.keynear.app.ui.model.VisibilityState
import org.keynear.app.data.LocalNetworkConstants
import org.keynear.app.data.LanManager
import org.keynear.app.service.KeynearForegroundService

fun launchFirstResolvableActivity(
    context: Context,
    candidates: List<Intent>,
): Boolean {
    for (candidate in candidates) {
        val intent = Intent(candidate).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        if (intent.resolveActivity(context.packageManager) != null) {
            runCatching {
                context.startActivity(intent)
                return true
            }
        }
    }
    return false
}

fun snapshotFromState(state: CoreState): CoreSnapshot? = when (state) {
    is CoreState.Empty -> state.snapshot
    is CoreState.Ready -> state.snapshot
    is CoreState.Error -> state.previous
    CoreState.Loading -> null
}

fun enumerateTreeChildUris(context: Context, treeUri: Uri): Pair<String, List<String>> {
    val treeDocId = DocumentsContract.getTreeDocumentId(treeUri)
    val folderLabel = treeDocId.substringAfterLast(':').substringAfterLast('/').ifBlank { "Selected folder" }
    val childrenUri = DocumentsContract.buildChildDocumentsUriUsingTree(treeUri, treeDocId)
    val collected = mutableListOf<String>()
    runCatching {
        context.contentResolver.query(
            childrenUri,
            arrayOf(
                DocumentsContract.Document.COLUMN_DOCUMENT_ID,
                DocumentsContract.Document.COLUMN_MIME_TYPE,
            ),
            null,
            null,
            null,
        )?.use { cursor ->
            val idIdx = cursor.getColumnIndex(DocumentsContract.Document.COLUMN_DOCUMENT_ID)
            val mimeIdx = cursor.getColumnIndex(DocumentsContract.Document.COLUMN_MIME_TYPE)
            while (cursor.moveToNext()) {
                val docId = if (idIdx >= 0) cursor.getString(idIdx) else null
                val mime = if (mimeIdx >= 0) cursor.getString(mimeIdx) else null
                if (!docId.isNullOrBlank() && mime != DocumentsContract.Document.MIME_TYPE_DIR) {
                    collected.add(DocumentsContract.buildDocumentUriUsingTree(treeUri, docId).toString())
                }
            }
        }
    }
    return folderLabel to collected
}

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
): KeynearActions = KeynearActions(
    onSetVisibility = { mode ->
        scope.launch {
            val updated = withContext(Dispatchers.IO) { bridge.setVisibility(mode) }
            val vis = snapshotFromState(updated)?.visibility ?: mode
            withContext(Dispatchers.IO) {
                BleScanner.startOrSync(context, vis)
                KeynearForegroundService.syncIfPermitted(context)
            }
            onStateUpdated(withContext(Dispatchers.IO) { bridge.loadState() })
            val label = when (vis) {
                VisibilityState.HIDDEN -> "Hidden"
                VisibilityState.PAIRED_ONLY -> "Paired devices only"
                VisibilityState.EVERYONE_NEARBY -> "Everyone nearby (10 min)"
            }
            onShowSnackbar("Visibility: $label")
        }
    },
    onRequestBluetoothPermissions = {
        val missing = KeynearBleConstants.missingPermissions(context, Build.VERSION.SDK_INT)
        if (missing.isNotEmpty()) {
            onLaunchPermissions(missing.toTypedArray())
        }
    },
    onOpenBluetoothSettings = {
        launchFirstResolvableActivity(context, listOf(Intent(Settings.ACTION_BLUETOOTH_SETTINGS)))
    },
    onRequestLocalNetworkPermission = {
        if (LocalNetworkConstants.requiresLocalNetworkPermission(Build.VERSION.SDK_INT)) {
            onLaunchPermissions(arrayOf(LocalNetworkConstants.PERMISSION_ACCESS_LOCAL_NETWORK))
        }
        scope.launch(Dispatchers.IO) {
            LanManager.requestPickerDiscovery(context)
            onStateUpdated(bridge.loadState())
        }
    },
    onRequestNearbyOnboardingPermissions = {
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
    },
    onRequestNotificationsPermission = {
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
    },
    onRequestBatteryOptimization = {
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
    },
    onOpenAppSettings = {
        launchFirstResolvableActivity(
            context,
            listOf(
                Intent(
                    Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                    Uri.fromParts("package", context.packageName, null),
                ),
            ),
        )
    },
    onInitiatePairing = { addr ->
        scope.launch {
            withContext(Dispatchers.IO) { GattSession.initiatePairing(context, addr) }
            onStateUpdated(withContext(Dispatchers.IO) { bridge.loadState() })
            onShowSnackbar("Pairing requested")
        }
    },
    onApprovePairing = {
        scope.launch {
            withContext(Dispatchers.IO) { GattSession.approveActivePairing(context) }
            onStateUpdated(withContext(Dispatchers.IO) { bridge.loadState() })
            onShowSnackbar("Device paired")
        }
    },
    onRejectPairing = {
        scope.launch {
            withContext(Dispatchers.IO) { GattSession.rejectActivePairing(context) }
            onStateUpdated(withContext(Dispatchers.IO) { bridge.loadState() })
            onShowSnackbar("Pairing declined")
        }
    },
    onToggleAutoAccept = { deviceId, enabled ->
        scope.launch {
            onStateUpdated(withContext(Dispatchers.IO) { bridge.setAutoAcceptFiles(deviceId, enabled) })
            onShowSnackbar(if (enabled) "Auto-accept enabled" else "Auto-accept disabled")
        }
    },
    onRenameDevice = { deviceId, newName ->
        scope.launch {
            onStateUpdated(withContext(Dispatchers.IO) { bridge.renamePairedDevice(deviceId, newName) })
            onShowSnackbar("Renamed to ${newName.trim()}")
        }
    },
    onResetBleBond = { deviceId ->
        scope.launch {
            onStateUpdated(withContext(Dispatchers.IO) { bridge.resetBleBond(deviceId) })
            onShowSnackbar("Bluetooth bond cleared")
        }
    },
    onAcceptTransfer = { transferId ->
        scope.launch {
            onStateUpdated(withContext(Dispatchers.IO) { bridge.acceptTransfer(transferId) })
        }
    },
    onDeclineTransfer = { transferId ->
        scope.launch {
            onStateUpdated(withContext(Dispatchers.IO) { bridge.declineTransfer(transferId) })
        }
    },
    onCancelTransfer = { transferId ->
        scope.launch {
            onStateUpdated(withContext(Dispatchers.IO) { bridge.cancelTransfer(transferId) })
        }
    },
    onRequestUnpair = onRequestUnpair,
    onCancelUnpair = onCancelUnpair,
    onConfirmUnpair = { deviceId ->
        onCancelUnpair()
        onCloseSubScreen()
        scope.launch {
            onStateUpdated(withContext(Dispatchers.IO) { bridge.unpairDevice(deviceId) })
            onShowSnackbar("Device removed")
        }
    },
    onPickFiles = onPickFiles,
    onPickFolder = onPickFolder,
    onStageClipboard = {
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
    },
    onStageTextSnippet = { draft ->
        scope.launch {
            onStateUpdated(withContext(Dispatchers.IO) { bridge.stageTextSnippet(draft) })
            onShowSnackbar("Text snippet staged")
        }
    },
    onRemoveQueueItem = { itemId ->
        scope.launch {
            onStateUpdated(withContext(Dispatchers.IO) { bridge.removeSendQueueItem(itemId) })
        }
    },
    onClearQueue = {
        scope.launch {
            onStateUpdated(withContext(Dispatchers.IO) { bridge.clearSendQueue() })
        }
    },
    onSendQueueToPeer = { device ->
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
    },
    onUpdateConfig = { transform ->
        scope.launch {
            onStateUpdated(withContext(Dispatchers.IO) { bridge.updateDaemonConfig(transform) })
        }
    },
    onPickSaveFolder = onPickSaveFolder,
    onOpenPermissions = onOpenPermissions,
    onOpenDeviceDetail = onOpenDeviceDetail,
    onJumpToSendTab = {
        onCloseSubScreen()
        onNavigateTab(KeynearTab.SEND)
    },
)

