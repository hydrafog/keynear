package org.keynear.app

import android.Manifest
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.DocumentsContract
import android.provider.Settings
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.keynear.app.ble.KeynearBleConstants
import org.keynear.app.ble.KeynearBleManager
import org.keynear.app.core.CoreBridge
import org.keynear.app.core.CoreClientProvider
import org.keynear.app.core.CoreSnapshot
import org.keynear.app.core.CoreState
import org.keynear.app.core.VisibilityState
import org.keynear.app.core.evaluatePermissionsChecklist
import org.keynear.app.lan.LocalNetworkConstants
import org.keynear.app.lan.LocalNetworkManager
import org.keynear.app.service.KeynearForegroundService
import org.keynear.app.ui.components.ActionButton
import org.keynear.app.ui.components.TactileSurface
import org.keynear.app.ui.devices.DeviceDetailScreen
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.app.ui.navigation.KeynearBottomNav
import org.keynear.app.ui.navigation.KeynearSnackbarHost
import org.keynear.app.ui.navigation.KeynearSubScreen
import org.keynear.app.ui.navigation.KeynearTab
import org.keynear.app.ui.navigation.KeynearTopAppBar
import org.keynear.app.ui.receive.ReceiveScreen
import org.keynear.app.ui.send.SendScreen
import org.keynear.app.ui.settings.PermissionsScreen
import org.keynear.app.ui.settings.SettingsScreen
import org.keynear.app.ui.theme.KeynearTheme
import org.keynear.app.ui.theme.LocalKeynearPalette

private fun launchFirstResolvableActivity(
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

class MainActivity : ComponentActivity() {
    private var resumeTick by mutableIntStateOf(0)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val bridge = CoreClientProvider.init(applicationContext)
        setContent {
            KeynearAppRoot(bridge = bridge, resumeTick = resumeTick)
        }
    }

    override fun onResume() {
        super.onResume()
        resumeTick += 1
    }
}

private fun snapshotFromState(state: CoreState): CoreSnapshot? = when (state) {
    is CoreState.Empty -> state.snapshot
    is CoreState.Ready -> state.snapshot
    is CoreState.Error -> state.previous
    CoreState.Loading -> null
}

private fun enumerateTreeChildUris(context: Context, treeUri: Uri): Pair<String, List<String>> {
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

@Composable
private fun KeynearAppRoot(
    bridge: CoreBridge,
    resumeTick: Int = 0,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var uiState by remember { mutableStateOf<CoreState>(CoreState.Loading) }
    var permissionsChecklist by remember { mutableStateOf(evaluatePermissionsChecklist(context)) }
    var confirmUnpairDeviceId by remember { mutableStateOf<String?>(null) }
    var activeTab by rememberSaveable { mutableStateOf(KeynearTab.RECEIVE) }
    var subScreen by remember { mutableStateOf<KeynearSubScreen?>(null) }
    var snackbarMessage by remember { mutableStateOf<String?>(null) }
    var sheetExpanded by rememberSaveable { mutableStateOf(false) }

    val snapshot = snapshotFromState(uiState)
    val darkTheme = snapshot?.daemonConfig?.darkTheme ?: isSystemInDarkTheme()

    val showSnackbar: (String) -> Unit = { msg ->
        snackbarMessage = msg
    }

    LaunchedEffect(snackbarMessage) {
        if (snackbarMessage != null) {
            delay(2600L)
            snackbarMessage = null
        }
    }

    val syncBleAndLoad: suspend () -> Unit = {
        val loaded = withContext(Dispatchers.IO) { bridge.loadState() }
        val vis = snapshotFromState(loaded)?.visibility ?: VisibilityState.PAIRED_ONLY
        withContext(Dispatchers.IO) {
            KeynearBleManager.startOrSync(context, vis)
            KeynearForegroundService.syncIfPermitted(context)
        }
        uiState = withContext(Dispatchers.IO) { bridge.loadState() }
        permissionsChecklist = evaluatePermissionsChecklist(context)
    }

    val permissionLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.RequestMultiplePermissions(),
    ) {
        scope.launch { syncBleAndLoad() }
    }

    val pickFilesLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenMultipleDocuments(),
    ) { uris ->
        if (uris.isNotEmpty()) {
            uris.forEach { uri ->
                runCatching {
                    context.contentResolver.takePersistableUriPermission(
                        uri,
                        Intent.FLAG_GRANT_READ_URI_PERMISSION,
                    )
                }
            }
            scope.launch {
                uiState = withContext(Dispatchers.IO) {
                    bridge.stageFileUris(uris.map { it.toString() })
                }
                showSnackbar("Added ${uris.size} file${if (uris.size == 1) "" else "s"} to queue")
            }
        }
    }

    val pickFolderLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocumentTree(),
    ) { treeUri ->
        if (treeUri != null) {
            runCatching {
                context.contentResolver.takePersistableUriPermission(
                    treeUri,
                    Intent.FLAG_GRANT_READ_URI_PERMISSION,
                )
            }
            scope.launch {
                val (folderName, childUris) = withContext(Dispatchers.IO) {
                    enumerateTreeChildUris(context, treeUri)
                }
                uiState = withContext(Dispatchers.IO) {
                    bridge.stageFolderUri(treeUri.toString(), folderName, childUris)
                }
                showSnackbar(
                    if (childUris.isEmpty()) "Staged folder $folderName"
                    else "Added ${childUris.size} files from $folderName",
                )
            }
        }
    }

    val pickSaveFolderLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocumentTree(),
    ) { treeUri ->
        if (treeUri != null) {
            val treeId = runCatching { DocumentsContract.getTreeDocumentId(treeUri) }.getOrNull()
            val displayPath = treeId?.substringAfter(':')?.takeIf { it.isNotBlank() }
                ?.let { "~/$it" } ?: treeUri.toString()
            scope.launch {
                uiState = withContext(Dispatchers.IO) {
                    bridge.updateDaemonConfig { it.copy(incomingDirectory = displayPath) }
                }
                showSnackbar("Save folder updated to $displayPath")
            }
        }
    }

    LaunchedEffect(bridge, resumeTick) {
        syncBleAndLoad()
    }

    DisposableEffect(bridge) {
        KeynearBleManager.setUiRefreshListener {
            scope.launch {
                uiState = withContext(Dispatchers.IO) { bridge.loadState() }
            }
        }
        onDispose {
            KeynearBleManager.setUiRefreshListener(null)
        }
    }

    LaunchedEffect(bridge) {
        while (true) {
            delay(1000L)
            val prevVis = snapshotFromState(uiState)?.visibility
            val nextState = withContext(Dispatchers.IO) { bridge.tick() }
            val nextVis = snapshotFromState(nextState)?.visibility
            if (prevVis == VisibilityState.EVERYONE_NEARBY && nextVis != null && nextVis != prevVis) {
                withContext(Dispatchers.IO) {
                    KeynearBleManager.startOrSync(context, nextVis)
                    KeynearForegroundService.syncIfPermitted(context)
                }
            }
            uiState = nextState
        }
    }

    BackHandler(enabled = subScreen != null) {
        subScreen = null
        confirmUnpairDeviceId = null
    }

    val actions = remember(context, bridge) {
        KeynearActions(
            onSetVisibility = { mode ->
                scope.launch {
                    val updated = withContext(Dispatchers.IO) { bridge.setVisibility(mode) }
                    val vis = snapshotFromState(updated)?.visibility ?: mode
                    withContext(Dispatchers.IO) {
                        KeynearBleManager.startOrSync(context, vis)
                        KeynearForegroundService.syncIfPermitted(context)
                    }
                    uiState = withContext(Dispatchers.IO) { bridge.loadState() }
                    val label = when (vis) {
                        VisibilityState.HIDDEN -> "Hidden"
                        VisibilityState.PAIRED_ONLY -> "Paired devices only"
                        VisibilityState.EVERYONE_NEARBY -> "Everyone nearby (10 min)"
                    }
                    showSnackbar("Visibility: $label")
                }
            },
            onRequestBluetoothPermissions = {
                val missing = KeynearBleConstants.missingPermissions(context, Build.VERSION.SDK_INT)
                if (missing.isNotEmpty()) {
                    permissionLauncher.launch(missing.toTypedArray())
                }
            },
            onOpenBluetoothSettings = {
                launchFirstResolvableActivity(context, listOf(Intent(Settings.ACTION_BLUETOOTH_SETTINGS)))
            },
            onRequestLocalNetworkPermission = {
                if (LocalNetworkConstants.requiresLocalNetworkPermission(Build.VERSION.SDK_INT)) {
                    permissionLauncher.launch(arrayOf(LocalNetworkConstants.PERMISSION_ACCESS_LOCAL_NETWORK))
                }
                scope.launch(Dispatchers.IO) {
                    LocalNetworkManager.requestPickerDiscovery(context)
                    uiState = bridge.loadState()
                }
            },
            onRequestNearbyOnboardingPermissions = {
                val missing = KeynearBleConstants.missingPermissions(context, Build.VERSION.SDK_INT)
                    .toMutableList()
                if (Build.VERSION.SDK_INT >= 33) {
                    missing.add(Manifest.permission.NEARBY_WIFI_DEVICES)
                }
                if (missing.isNotEmpty()) {
                    permissionLauncher.launch(missing.distinct().toTypedArray())
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
                    permissionLauncher.launch(arrayOf(Manifest.permission.POST_NOTIFICATIONS))
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
                    withContext(Dispatchers.IO) { KeynearBleManager.initiatePairing(context, addr) }
                    uiState = withContext(Dispatchers.IO) { bridge.loadState() }
                    showSnackbar("Pairing requested")
                }
            },
            onApprovePairing = {
                scope.launch {
                    withContext(Dispatchers.IO) { KeynearBleManager.approveActivePairing(context) }
                    uiState = withContext(Dispatchers.IO) { bridge.loadState() }
                    showSnackbar("Device paired")
                }
            },
            onRejectPairing = {
                scope.launch {
                    withContext(Dispatchers.IO) { KeynearBleManager.rejectActivePairing(context) }
                    uiState = withContext(Dispatchers.IO) { bridge.loadState() }
                    showSnackbar("Pairing declined")
                }
            },
            onToggleAutoAccept = { deviceId, enabled ->
                scope.launch {
                    uiState = withContext(Dispatchers.IO) { bridge.setAutoAcceptFiles(deviceId, enabled) }
                    showSnackbar(if (enabled) "Auto-accept enabled" else "Auto-accept disabled")
                }
            },
            onRenameDevice = { deviceId, newName ->
                scope.launch {
                    uiState = withContext(Dispatchers.IO) { bridge.renamePairedDevice(deviceId, newName) }
                    showSnackbar("Renamed to ${newName.trim()}")
                }
            },
            onResetBleBond = { deviceId ->
                scope.launch {
                    uiState = withContext(Dispatchers.IO) { bridge.resetBleBond(deviceId) }
                    showSnackbar("Bluetooth bond cleared")
                }
            },
            onAcceptTransfer = { transferId ->
                scope.launch {
                    uiState = withContext(Dispatchers.IO) { bridge.acceptTransfer(transferId) }
                }
            },
            onDeclineTransfer = { transferId ->
                scope.launch {
                    uiState = withContext(Dispatchers.IO) { bridge.declineTransfer(transferId) }
                }
            },
            onCancelTransfer = { transferId ->
                scope.launch {
                    uiState = withContext(Dispatchers.IO) { bridge.cancelTransfer(transferId) }
                }
            },
            onRequestUnpair = { deviceId -> confirmUnpairDeviceId = deviceId },
            onCancelUnpair = { confirmUnpairDeviceId = null },
            onConfirmUnpair = { deviceId ->
                confirmUnpairDeviceId = null
                subScreen = null
                scope.launch {
                    uiState = withContext(Dispatchers.IO) { bridge.unpairDevice(deviceId) }
                    showSnackbar("Device removed")
                }
            },
            onPickFiles = { pickFilesLauncher.launch(arrayOf("*/*")) },
            onPickFolder = { pickFolderLauncher.launch(null) },
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
                    showSnackbar("Clipboard has no text item")
                } else {
                    scope.launch {
                        uiState = withContext(Dispatchers.IO) { bridge.stageClipboardText(clipText) }
                        showSnackbar("Clipboard snippet staged")
                    }
                }
            },
            onStageTextSnippet = { draft ->
                scope.launch {
                    uiState = withContext(Dispatchers.IO) { bridge.stageTextSnippet(draft) }
                    showSnackbar("Text snippet staged")
                }
            },
            onRemoveQueueItem = { itemId ->
                scope.launch {
                    uiState = withContext(Dispatchers.IO) { bridge.removeSendQueueItem(itemId) }
                }
            },
            onClearQueue = {
                scope.launch {
                    uiState = withContext(Dispatchers.IO) { bridge.clearSendQueue() }
                }
            },
            onSendQueueToPeer = { device ->
                scope.launch {
                    val res = withContext(Dispatchers.IO) { bridge.sendQueueToPeer(device.deviceId) }
                    uiState = withContext(Dispatchers.IO) { bridge.loadState() }
                    res.onSuccess {
                        activeTab = KeynearTab.RECEIVE
                        showSnackbar("Sending to ${device.displayName}")
                    }.onFailure { err ->
                        showSnackbar(err.message ?: "Unable to start transfer")
                    }
                }
            },
            onUpdateConfig = { transform ->
                scope.launch {
                    uiState = withContext(Dispatchers.IO) { bridge.updateDaemonConfig(transform) }
                }
            },
            onPickSaveFolder = { pickSaveFolderLauncher.launch(null) },
            onOpenPermissions = { subScreen = KeynearSubScreen.Permissions },
            onOpenDeviceDetail = { deviceId ->
                confirmUnpairDeviceId = null
                subScreen = KeynearSubScreen.DeviceDetail(deviceId)
            },
            onJumpToSendTab = {
                subScreen = null
                activeTab = KeynearTab.SEND
            },
        )
    }

    KeynearTheme(darkTheme = darkTheme) {
        val palette = LocalKeynearPalette.current
        val localName = remember { Build.MODEL?.takeIf { it.isNotBlank() } ?: "Android" }
        val selectedDevice = snapshot?.pairedDevices?.firstOrNull {
            (subScreen as? KeynearSubScreen.DeviceDetail)?.deviceId == it.deviceId
        }

        Scaffold(
            modifier = Modifier
                .fillMaxSize()
                .background(palette.canvas),
            containerColor = palette.canvas,
            topBar = {
                KeynearTopAppBar(
                    activeTab = activeTab,
                    subScreen = subScreen,
                    deviceDisplayName = selectedDevice?.displayName,
                    onBack = {
                        subScreen = null
                        confirmUnpairDeviceId = null
                    },
                )
            },
            bottomBar = {
                KeynearBottomNav(
                    activeTab = activeTab,
                    subScreen = subScreen,
                    onSelectTab = { tab ->
                        subScreen = null
                        confirmUnpairDeviceId = null
                        activeTab = tab
                    },
                )
            },
        ) { innerPadding ->
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(innerPadding),
            ) {
                if (snapshot == null) {
                    Box(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(24.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        when (val current = uiState) {
                            CoreState.Loading -> CircularProgressIndicator(color = palette.loading)
                            is CoreState.Error -> {
                                TactileSurface(
                                    modifier = Modifier.fillMaxWidth(),
                                    shape = RoundedCornerShape(16.dp),
                                    containerColor = palette.errorTonal,
                                    shadowElevation = 0.dp,
                                ) {
                                    Column(
                                        modifier = Modifier.padding(16.dp),
                                        verticalArrangement = Arrangement.spacedBy(10.dp),
                                    ) {
                                        Text(
                                            text = current.message,
                                            color = palette.onDangerTonal,
                                            fontSize = 14.sp,
                                            fontWeight = FontWeight.Medium,
                                        )
                                        ActionButton(
                                            label = "Retry",
                                            primary = true,
                                            onClick = { scope.launch { syncBleAndLoad() } },
                                        )
                                    }
                                }
                            }
                            else -> {}
                        }
                    }
                } else {
                    when (val currentSub = subScreen) {
                        KeynearSubScreen.Permissions -> PermissionsScreen(
                            permissions = permissionsChecklist,
                            actions = actions,
                        )
                        is KeynearSubScreen.DeviceDetail -> DeviceDetailScreen(
                            device = snapshot.pairedDevices.firstOrNull { it.deviceId == currentSub.deviceId },
                            confirmingUnpair = confirmUnpairDeviceId == currentSub.deviceId,
                            actions = actions,
                        )
                        null -> when (activeTab) {
                            KeynearTab.RECEIVE -> ReceiveScreen(
                                snapshot = snapshot,
                                permissions = permissionsChecklist,
                                localDeviceName = localName,
                                sheetExpanded = sheetExpanded,
                                onToggleSheetExpanded = { sheetExpanded = !sheetExpanded },
                                actions = actions,
                            )
                            KeynearTab.SEND -> SendScreen(
                                snapshot = snapshot,
                                actions = actions,
                            )
                            KeynearTab.SETTINGS -> SettingsScreen(
                                config = snapshot.daemonConfig,
                                permissions = permissionsChecklist,
                                actions = actions,
                            )
                        }
                    }
                }

                Box(modifier = Modifier.align(Alignment.BottomCenter)) {
                    KeynearSnackbarHost(message = snackbarMessage)
                }
            }
        }
    }
}
