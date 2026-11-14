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
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.keynear.app.ble.KeynearBleConstants
import org.keynear.app.ble.KeynearBleManager
import org.keynear.app.ble.PairingNotificationManager
import org.keynear.app.core.BluetoothStatus
import org.keynear.app.core.CoreBridge
import org.keynear.app.core.CoreClientProvider
import org.keynear.app.core.CoreSnapshot
import org.keynear.app.core.CoreState
import org.keynear.app.core.DaemonConfigUi
import org.keynear.app.core.DeviceRoleUi
import org.keynear.app.core.DiscoveredPeerUi
import org.keynear.app.core.LocalNetworkStatus
import org.keynear.app.core.PairedDeviceUi
import org.keynear.app.core.PendingPairingUi
import org.keynear.app.core.PermissionsChecklistState
import org.keynear.app.core.SendItemKind
import org.keynear.app.core.SendQueueItemUi
import org.keynear.app.core.TransferDirectionUi
import org.keynear.app.core.TransferStateUi
import org.keynear.app.core.TransferUi
import org.keynear.app.core.VisibilityState
import org.keynear.app.core.evaluatePermissionsChecklist
import org.keynear.app.core.formatBytes
import org.keynear.app.core.orbitFor
import org.keynear.app.core.validateDisplayNameInput
import org.keynear.app.lan.LocalNetworkConstants
import org.keynear.app.lan.LocalNetworkManager
import org.keynear.app.service.KeynearForegroundService
import org.keynear.app.ui.theme.KeynearMotion
import org.keynear.app.ui.theme.KeynearRadii
import org.keynear.app.ui.theme.KeynearSpacing
import org.keynear.app.ui.theme.KeynearTheme
import org.keynear.app.ui.theme.LocalKeynearPalette
import java.util.Locale
import kotlin.math.cos
import kotlin.math.roundToInt
import kotlin.math.sin

private enum class KeynearTab {
    RECEIVE,
    SEND,
    SETTINGS,
}

private sealed interface KeynearSubScreen {
    data object Permissions : KeynearSubScreen
    data class DeviceDetail(val deviceId: String) : KeynearSubScreen
}

private data class OrbitPinUi(
    val id: String,
    val displayName: String,
    val statusLabel: String,
    val role: DeviceRoleUi,
    val rssi: Int,
    val isPaired: Boolean,
    val isOnline: Boolean,
    val peerAddress: String,
)

private data class KeynearActions(
    val onSetVisibility: (VisibilityState) -> Unit,
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
    val onUpdateConfig: ((DaemonConfigUi) -> DaemonConfigUi) -> Unit,
    val onPickSaveFolder: () -> Unit,
    val onOpenPermissions: () -> Unit,
    val onOpenDeviceDetail: (String) -> Unit,
    val onJumpToSendTab: () -> Unit,
)

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
    val systemDark = isSystemInDarkTheme()
    val darkTheme = snapshot?.daemonConfig?.darkTheme ?: systemDark

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

    val actions = KeynearActions(
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
            launchFirstResolvableActivity(
                context,
                listOf(Intent(Settings.ACTION_BLUETOOTH_SETTINGS)),
            )
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
        onRequestUnpair = { deviceId ->
            confirmUnpairDeviceId = deviceId
        },
        onCancelUnpair = {
            confirmUnpairDeviceId = null
        },
        onConfirmUnpair = { deviceId ->
            confirmUnpairDeviceId = null
            subScreen = null
            scope.launch {
                uiState = withContext(Dispatchers.IO) { bridge.unpairDevice(deviceId) }
                showSnackbar("Device removed")
            }
        },
        onPickFiles = {
            pickFilesLauncher.launch(arrayOf("*/*"))
        },
        onPickFolder = {
            pickFolderLauncher.launch(null)
        },
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
        onPickSaveFolder = {
            pickSaveFolderLauncher.launch(null)
        },
        onOpenPermissions = {
            subScreen = KeynearSubScreen.Permissions
        },
        onOpenDeviceDetail = { deviceId ->
            confirmUnpairDeviceId = null
            subScreen = KeynearSubScreen.DeviceDetail(deviceId)
        },
        onJumpToSendTab = {
            subScreen = null
            activeTab = KeynearTab.SEND
        },
    )

    KeynearTheme(darkTheme = darkTheme) {
        val palette = LocalKeynearPalette.current
        val localName = remember { Build.MODEL?.takeIf { it.isNotBlank() } ?: "Android" }
        val appVersion = snapshot?.version ?: "0.4.0"

        Scaffold(
            modifier = Modifier
                .fillMaxSize()
                .background(palette.canvas),
            containerColor = palette.canvas,
            topBar = {
                KeynearTopAppBar(
                    activeTab = activeTab,
                    subScreen = subScreen,
                    localDeviceName = localName,
                    version = appVersion,
                    missingPermissionsCount = permissionsChecklist.missingPermissionsCount,
                    onBack = {
                        subScreen = null
                        confirmUnpairDeviceId = null
                    },
                    onOpenPermissions = actions.onOpenPermissions,
                )
            },
            bottomBar = {
                if (subScreen == null) {
                    KeynearBottomNav(
                        activeTab = activeTab,
                        missingPermissionsCount = permissionsChecklist.missingPermissionsCount,
                        onSelectTab = { tab ->
                            activeTab = tab
                        },
                    )
                }
            },
        ) { innerPadding ->
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(innerPadding),
            ) {
                if (uiState is CoreState.Loading) {
                    LoadingView()
                } else {
                    val activeSnapshot = snapshot
                    if (activeSnapshot == null) {
                        val err = uiState as? CoreState.Error
                        ErrorStateCard(
                            message = err?.message ?: "Unable to initialize Keynear",
                            onRetry = { scope.launch { syncBleAndLoad() } },
                            modifier = Modifier.padding(KeynearSpacing.Lg),
                        )
                    } else {
                        val currentSub = subScreen
                        Crossfade(
                            targetState = currentSub to activeTab,
                            animationSpec = tween(durationMillis = KeynearMotion.FADE_DURATION_MS),
                            label = "keynearScreenTransition",
                        ) { (targetSub, targetTab) ->
                            when (targetSub) {
                                KeynearSubScreen.Permissions -> PermissionsScreen(
                                    checklist = permissionsChecklist,
                                    actions = actions,
                                )
                                is KeynearSubScreen.DeviceDetail -> {
                                    val device = activeSnapshot.pairedDevices.firstOrNull {
                                        it.deviceId == targetSub.deviceId
                                    }
                                    if (device == null) {
                                        EmptyDetailFallback(onBack = { subScreen = null })
                                    } else {
                                        DeviceDetailScreen(
                                            device = device,
                                            confirmUnpair = confirmUnpairDeviceId == device.deviceId,
                                            actions = actions,
                                        )
                                    }
                                }
                                null -> when (targetTab) {
                                    KeynearTab.RECEIVE -> ReceiveScreen(
                                        snapshot = activeSnapshot,
                                        checklist = permissionsChecklist,
                                        errorBannerMessage = (uiState as? CoreState.Error)?.message,
                                        sheetExpanded = sheetExpanded,
                                        onToggleSheet = { sheetExpanded = !sheetExpanded },
                                        onRetryError = { scope.launch { syncBleAndLoad() } },
                                        actions = actions,
                                    )
                                    KeynearTab.SEND -> SendScreen(
                                        snapshot = activeSnapshot,
                                        actions = actions,
                                    )
                                    KeynearTab.SETTINGS -> SettingsScreen(
                                        snapshot = activeSnapshot,
                                        checklist = permissionsChecklist,
                                        actions = actions,
                                        onShowMessage = showSnackbar,
                                    )
                                }
                            }
                        }
                    }
                }

                AnimatedVisibility(
                    visible = snackbarMessage != null,
                    modifier = Modifier
                        .align(Alignment.BottomCenter)
                        .padding(horizontal = KeynearSpacing.Lg, vertical = 14.dp),
                ) {
                    snackbarMessage?.let { text ->
                        SnackbarPill(message = text)
                    }
                }
            }
        }
    }
}

@Composable
private fun KeynearTopAppBar(
    activeTab: KeynearTab,
    subScreen: KeynearSubScreen?,
    localDeviceName: String,
    version: String,
    missingPermissionsCount: Int,
    onBack: () -> Unit,
    onOpenPermissions: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .statusBarsPadding()
            .height(64.dp)
            .padding(horizontal = KeynearSpacing.Lg),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        if (subScreen != null) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
            ) {
                Surface(
                    onClick = onBack,
                    modifier = Modifier.size(KeynearSpacing.MinTouchTarget),
                    shape = CircleShape,
                    color = palette.surfaceTonal,
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        IconArrowLeft(color = palette.textPrimary)
                    }
                }
                Text(
                    text = when (subScreen) {
                        KeynearSubScreen.Permissions -> "Permissions"
                        is KeynearSubScreen.DeviceDetail -> "Device details"
                    },
                    fontSize = 22.sp,
                    fontWeight = FontWeight.ExtraBold,
                    color = palette.textPrimary,
                )
            }
        } else {
            when (activeTab) {
                KeynearTab.RECEIVE -> Column(modifier = Modifier.weight(1f)) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        Text(
                            text = "Keynear",
                            fontSize = 17.sp,
                            fontWeight = FontWeight.ExtraBold,
                            color = palette.textPrimary,
                        )
                        Box(
                            modifier = Modifier
                                .size(8.dp)
                                .clip(CircleShape)
                                .background(palette.loading),
                        )
                    }
                    Text(
                        text = "$localDeviceName · v$version",
                        fontSize = 12.sp,
                        color = palette.textMuted,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                KeynearTab.SEND -> Text(
                    text = "Send files",
                    fontSize = 22.sp,
                    fontWeight = FontWeight.ExtraBold,
                    color = palette.textPrimary,
                    modifier = Modifier.weight(1f),
                )
                KeynearTab.SETTINGS -> Text(
                    text = "Settings",
                    fontSize = 22.sp,
                    fontWeight = FontWeight.ExtraBold,
                    color = palette.textPrimary,
                    modifier = Modifier.weight(1f),
                )
            }

            if (missingPermissionsCount > 0) {
                Surface(
                    onClick = onOpenPermissions,
                    modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                    shape = RoundedCornerShape(999.dp),
                    color = palette.warningTonal,
                ) {
                    Row(
                        modifier = Modifier.padding(horizontal = 12.dp, vertical = 10.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                    ) {
                        IconAlertTriangle(color = palette.onWarningTonal, size = 15.dp)
                        Text(
                            text = "$missingPermissionsCount– permissions",
                            fontSize = 12.sp,
                            fontWeight = FontWeight.Bold,
                            color = palette.onWarningTonal,
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun KeynearBottomNav(
    activeTab: KeynearTab,
    missingPermissionsCount: Int,
    onSelectTab: (KeynearTab) -> Unit,
) {
    val palette = LocalKeynearPalette.current
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .background(palette.canvas)
            .navigationBarsPadding(),
    ) {
        HorizontalDivider(color = palette.switchTrackOff.copy(alpha = 0.7f))
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .height(74.dp)
                .padding(horizontal = KeynearSpacing.Lg),
            horizontalArrangement = Arrangement.SpaceEvenly,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            BottomNavItem(
                label = "Receive",
                selected = activeTab == KeynearTab.RECEIVE,
                showBadgeDot = false,
                onClick = { onSelectTab(KeynearTab.RECEIVE) },
                icon = { tint -> IconInbox(color = tint) },
            )
            BottomNavItem(
                label = "Send",
                selected = activeTab == KeynearTab.SEND,
                showBadgeDot = false,
                onClick = { onSelectTab(KeynearTab.SEND) },
                icon = { tint -> IconSend(color = tint) },
            )
            BottomNavItem(
                label = "Settings",
                selected = activeTab == KeynearTab.SETTINGS,
                showBadgeDot = missingPermissionsCount > 0,
                onClick = { onSelectTab(KeynearTab.SETTINGS) },
                icon = { tint -> IconSliders(color = tint) },
            )
        }
    }
}

@Composable
private fun BottomNavItem(
    label: String,
    selected: Boolean,
    showBadgeDot: Boolean,
    onClick: () -> Unit,
    icon: @Composable (Color) -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val interaction = remember { MutableInteractionSource() }
    Column(
        modifier = Modifier
            .defaultMinSize(minWidth = 72.dp, minHeight = KeynearSpacing.MinTouchTarget)
            .clip(RoundedCornerShape(16.dp))
            .clickable(
                interactionSource = interaction,
                indication = null,
                role = Role.Tab,
                onClick = onClick,
            )
            .padding(vertical = 6.dp, horizontal = 8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Box(
            modifier = Modifier
                .width(64.dp)
                .height(32.dp)
                .clip(RoundedCornerShape(16.dp))
                .background(if (selected) palette.accent else Color.Transparent),
            contentAlignment = Alignment.Center,
        ) {
            Box {
                icon(if (selected) palette.onAccent else palette.textMuted)
                if (showBadgeDot) {
                    Box(
                        modifier = Modifier
                            .align(Alignment.TopEnd)
                            .offset(x = 4.dp, y = (-2).dp)
                            .size(7.dp)
                            .clip(CircleShape)
                            .background(palette.error),
                    )
                }
            }
        }
        Text(
            text = label,
            fontSize = 12.sp,
            fontWeight = if (selected) FontWeight.Bold else FontWeight.Medium,
            color = if (selected) palette.textPrimary else palette.textMuted,
        )
    }
}

@Composable
private fun ReceiveScreen(
    snapshot: CoreSnapshot,
    checklist: PermissionsChecklistState,
    errorBannerMessage: String?,
    sheetExpanded: Boolean,
    onToggleSheet: () -> Unit,
    onRetryError: () -> Unit,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current
    val pins = remember(snapshot.visibility, snapshot.pairedDevices, snapshot.nearbyPeers) {
        if (snapshot.visibility == VisibilityState.HIDDEN) {
            emptyList()
        } else {
            val pairedPins = snapshot.pairedDevices.map { dev ->
                OrbitPinUi(
                    id = dev.deviceId,
                    displayName = dev.displayName,
                    statusLabel = if (dev.isOnline) "Wi-Fi" else "Standby",
                    role = dev.role,
                    rssi = dev.rssi,
                    isPaired = true,
                    isOnline = dev.isOnline,
                    peerAddress = dev.lastBleAddress,
                )
            }
            val nearbyPins = snapshot.nearbyPeers
                .filterNot { it.isPaired }
                .map { peer ->
                    OrbitPinUi(
                        id = peer.address,
                        displayName = peer.displayName,
                        statusLabel = "${peer.rssi} dBm",
                        role = peer.role,
                        rssi = peer.rssi,
                        isPaired = false,
                        isOnline = false,
                        peerAddress = peer.address,
                    )
                }
            pairedPins + nearbyPins
        }
    }

    Column(modifier = Modifier.fillMaxSize()) {
        Column(
            modifier = Modifier
                .weight(1f)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = KeynearSpacing.Lg),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
        ) {
            VisibilitySegmentControl(
                visibility = snapshot.visibility,
                everyoneRemainingSecs = snapshot.everyoneRemainingSecs,
                onSelect = actions.onSetVisibility,
            )

            if (errorBannerMessage != null) {
                ErrorStateCard(message = errorBannerMessage, onRetry = onRetryError)
            }

            if (snapshot.bluetoothStatus != BluetoothStatus.READY) {
                BluetoothWarningCard(
                    status = snapshot.bluetoothStatus,
                    message = snapshot.warningMessage,
                    onRequestPermissions = actions.onRequestBluetoothPermissions,
                    onOpenBluetoothSettings = actions.onOpenBluetoothSettings,
                )
            }

            if (snapshot.localNetworkStatus != LocalNetworkStatus.READY) {
                LocalNetworkWarningCard(
                    status = snapshot.localNetworkStatus,
                    message = snapshot.localNetworkMessage,
                    onRequestPermission = actions.onRequestLocalNetworkPermission,
                )
            }

            if (checklist.missingPermissionsCount > 0) {
                Surface(
                    onClick = actions.onOpenPermissions,
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(KeynearRadii.Xl),
                    color = palette.warningTonal,
                ) {
                    Row(
                        modifier = Modifier.padding(horizontal = 14.dp, vertical = 12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        IconAlertTriangle(color = palette.onWarningTonal, size = 18.dp)
                        Text(
                            text = "${checklist.missingPermissionsCount} permission(s) missing — tap to review",
                            fontSize = 13.sp,
                            fontWeight = FontWeight.SemiBold,
                            color = palette.onWarningTonal,
                            modifier = Modifier.weight(1f),
                        )
                        IconChevronRight(color = palette.onWarningTonal, size = 16.dp)
                    }
                }
            }

            snapshot.pendingPairing?.let { pending ->
                PendingPairingCard(
                    pending = pending,
                    onApprove = actions.onApprovePairing,
                    onReject = actions.onRejectPairing,
                )
            }

            if (snapshot.activeTransfers.isNotEmpty()) {
                ActiveTransfersCard(
                    transfers = snapshot.activeTransfers,
                    onAccept = actions.onAcceptTransfer,
                    onDecline = actions.onDeclineTransfer,
                    onCancel = actions.onCancelTransfer,
                )
            }

            OrbitRadarStage(
                visibility = snapshot.visibility,
                everyoneRemainingSecs = snapshot.everyoneRemainingSecs,
                pins = pins,
                onTapPin = { pin ->
                    if (pin.isPaired) {
                        actions.onOpenDeviceDetail(pin.id)
                    } else {
                        actions.onInitiatePairing(pin.peerAddress)
                    }
                },
            )
        }

        DevicesInRangeSheet(
            pairedDevices = snapshot.pairedDevices,
            nearbyPeers = snapshot.nearbyPeers.filterNot { it.isPaired },
            visibleToPeers = snapshot.visibility != VisibilityState.HIDDEN,
            expanded = sheetExpanded,
            onToggleExpand = onToggleSheet,
            onOpenDevice = actions.onOpenDeviceDetail,
            onSendToDevice = { actions.onJumpToSendTab() },
            onPairPeer = actions.onInitiatePairing,
        )
    }
}

@Composable
private fun VisibilitySegmentControl(
    visibility: VisibilityState,
    everyoneRemainingSecs: Long,
    onSelect: (VisibilityState) -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val countdown = formatCountdown(everyoneRemainingSecs)
    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.TwoXl),
        color = palette.trackWell,
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(4.dp),
            horizontalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            val segments = listOf(
                VisibilityState.HIDDEN to "Hidden",
                VisibilityState.PAIRED_ONLY to "Paired only",
                VisibilityState.EVERYONE_NEARBY to if (visibility == VisibilityState.EVERYONE_NEARBY) {
                    "Everyone $countdown"
                } else {
                    "Everyone"
                },
            )
            segments.forEach { (state, label) ->
                val active = visibility == state
                Surface(
                    onClick = { onSelect(state) },
                    modifier = Modifier
                        .weight(1f)
                        .defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                    shape = RoundedCornerShape(KeynearRadii.Xl),
                    color = if (active) palette.surface else Color.Transparent,
                    shadowElevation = if (active) 3.dp else 0.dp,
                ) {
                    Box(
                        modifier = Modifier.padding(horizontal = 6.dp, vertical = 10.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        Text(
                            text = label,
                            fontSize = 13.sp,
                            fontWeight = if (active) FontWeight.Bold else FontWeight.Medium,
                            color = if (active) palette.textPrimary else palette.textMuted,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                            textAlign = TextAlign.Center,
                        )
                    }
                }
            }
        }
    }
}

private fun formatCountdown(secs: Long): String {
    val clamped = secs.coerceAtLeast(0L)
    val mins = clamped / 60L
    val rem = clamped % 60L
    return String.format(Locale.US, "%02d:%02d", mins, rem)
}

@Composable
private fun OrbitRadarStage(
    visibility: VisibilityState,
    everyoneRemainingSecs: Long,
    pins: List<OrbitPinUi>,
    onTapPin: (OrbitPinUi) -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val transition = rememberInfiniteTransition(label = "orbitLoop")
    val orbitProgress by transition.animateFloat(
        initialValue = 0f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 40_000, easing = LinearEasing),
            repeatMode = RepeatMode.Restart,
        ),
        label = "orbitProgress",
    )

    TactileSurface(
        modifier = Modifier
            .fillMaxWidth()
            .height(352.dp),
        cornerRadius = 24.dp,
        color = palette.mapGround,
    ) {
        BoxWithConstraints(modifier = Modifier.fillMaxSize()) {
            val density = LocalDensity.current
            val widthPx = with(density) { maxWidth.toPx() }
            val heightPx = with(density) { maxHeight.toPx() }
            val centerX = widthPx / 2f
            val centerY = heightPx / 2f
            val minHalf = minOf(centerX, centerY)
            val maxOrbitPx = (minHalf - with(density) { 46.dp.toPx() }).coerceAtLeast(80f)
            val auraColor = palette.accent.copy(alpha = if (palette.isDark) 0.22f else 0.14f)
            val ringColor = if (palette.isDark) {
                Color(0x1E4199FF)
            } else {
                Color(0x28004CED)
            }

            Canvas(modifier = Modifier.fillMaxSize()) {
                if (visibility != VisibilityState.HIDDEN) {
                    drawCircle(
                        brush = Brush.radialGradient(
                            colors = listOf(auraColor, Color.Transparent),
                            center = Offset(centerX, centerY),
                            radius = maxOrbitPx * 1.15f,
                        ),
                        center = Offset(centerX, centerY),
                        radius = maxOrbitPx * 1.15f,
                    )
                }
                val dash = PathEffect.dashPathEffect(floatArrayOf(10f, 10f), 0f)
                listOf(0.45f, 0.72f, 1.0f).forEach { factor ->
                    drawCircle(
                        color = ringColor,
                        radius = maxOrbitPx * factor,
                        center = Offset(centerX, centerY),
                        style = Stroke(width = 1.5f, pathEffect = dash),
                    )
                }
            }

            Column(
                modifier = Modifier.align(Alignment.Center),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                Surface(
                    modifier = Modifier.size(72.dp),
                    shape = CircleShape,
                    color = if (visibility == VisibilityState.HIDDEN) palette.surfaceTonal else palette.accent,
                    shadowElevation = if (visibility == VisibilityState.HIDDEN) 1.dp else 8.dp,
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        if (visibility == VisibilityState.HIDDEN) {
                            IconEyeOff(color = palette.textMuted, size = 26.dp)
                        } else {
                            IconRadio(color = palette.onAccent, size = 28.dp)
                        }
                    }
                }
                Surface(
                    shape = RoundedCornerShape(999.dp),
                    color = palette.surface,
                    shadowElevation = 2.dp,
                ) {
                    Text(
                        text = when (visibility) {
                            VisibilityState.HIDDEN -> "Paused · Hidden"
                            VisibilityState.PAIRED_ONLY -> "Ready · Paired"
                            VisibilityState.EVERYONE_NEARBY -> "Visible · ${formatCountdown(everyoneRemainingSecs)}"
                        },
                        fontSize = 11.sp,
                        fontWeight = FontWeight.Bold,
                        color = palette.textSecondary,
                        modifier = Modifier.padding(horizontal = 10.dp, vertical = 4.dp),
                    )
                }
            }

            val pinWidthPx = with(density) { 104.dp.toPx() }
            val pinHeightPx = with(density) { 68.dp.toPx() }
            pins.forEachIndexed { idx, pin ->
                val placement = orbitFor(pin.rssi, idx, pins.size)
                val normalizedRadius = ((placement.radiusPx - 128f) / 48f).coerceIn(0f, 1f)
                val ringRadiusPx = maxOrbitPx * (0.58f + normalizedRadius * 0.42f)
                val phaseFraction = placement.phaseSec / placement.durationSec.toFloat()
                val angleRad = ((orbitProgress + phaseFraction) * 2.0 * Math.PI) - (Math.PI / 2.0)
                val px = centerX + (ringRadiusPx * cos(angleRad)).toFloat() - pinWidthPx / 2f
                val py = centerY + (ringRadiusPx * sin(angleRad)).toFloat() - pinHeightPx / 2f

                Column(
                    modifier = Modifier
                        .offset { IntOffset(px.roundToInt(), py.roundToInt()) }
                        .width(104.dp)
                        .clip(RoundedCornerShape(16.dp))
                        .clickable { onTapPin(pin) }
                        .padding(vertical = 2.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    val badgeColor = when {
                        pin.isPaired && pin.isOnline -> palette.accent
                        !pin.isPaired -> palette.loading
                        else -> palette.surface
                    }
                    val iconTint = when {
                        (pin.isPaired && pin.isOnline) || !pin.isPaired -> palette.onAccent
                        else -> palette.textSecondary
                    }
                    Surface(
                        modifier = Modifier.size(44.dp),
                        shape = CircleShape,
                        color = badgeColor,
                        shadowElevation = 4.dp,
                    ) {
                        Box(contentAlignment = Alignment.Center) {
                            DeviceRoleIcon(role = pin.role, color = iconTint, size = 20.dp)
                        }
                    }
                    Surface(
                        shape = RoundedCornerShape(999.dp),
                        color = palette.surface,
                        shadowElevation = 2.dp,
                    ) {
                        Text(
                            text = "${pin.displayName} · ${pin.statusLabel}",
                            fontSize = 10.sp,
                            fontWeight = FontWeight.SemiBold,
                            color = palette.textPrimary,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                            modifier = Modifier.padding(horizontal = 7.dp, vertical = 2.dp),
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun DevicesInRangeSheet(
    pairedDevices: List<PairedDeviceUi>,
    nearbyPeers: List<DiscoveredPeerUi>,
    visibleToPeers: Boolean,
    expanded: Boolean,
    onToggleExpand: () -> Unit,
    onOpenDevice: (String) -> Unit,
    onSendToDevice: (String) -> Unit,
    onPairPeer: (String) -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val totalCount = pairedDevices.size + nearbyPeers.size
    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(topStart = 28.dp, topEnd = 28.dp),
        color = palette.surface,
        shadowElevation = 10.dp,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = KeynearSpacing.Lg, vertical = 10.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(12.dp))
                    .clickable(onClick = onToggleExpand)
                    .padding(vertical = 4.dp),
            ) {
                Column(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Box(
                        modifier = Modifier
                            .width(36.dp)
                            .height(4.dp)
                            .clip(RoundedCornerShape(999.dp))
                            .background(palette.sheetHandle),
                    )
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            text = "$totalCount device${if (totalCount == 1) "" else "s"} in range",
                            fontSize = 15.sp,
                            fontWeight = FontWeight.Bold,
                            color = palette.textPrimary,
                        )
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(4.dp),
                        ) {
                            Text(
                                text = if (expanded) "Collapse" else "Show all",
                                fontSize = 12.sp,
                                fontWeight = FontWeight.SemiBold,
                                color = palette.accent,
                            )
                            if (expanded) {
                                IconChevronDown(color = palette.accent, size = 16.dp)
                            } else {
                                IconChevronUp(color = palette.accent, size = 16.dp)
                            }
                        }
                    }
                }
            }

            if (totalCount == 0) {
                Text(
                    text = if (visibleToPeers) {
                        "No nearby devices found yet. Paired laptops and phones appear automatically."
                    } else {
                        "Visibility is hidden. Switch to Paired only or Everyone to scan."
                    },
                    fontSize = 13.sp,
                    color = palette.textMuted,
                    modifier = Modifier.padding(bottom = 8.dp),
                )
            } else {
                val maxPairedShown = if (expanded) pairedDevices.size else minOf(2, pairedDevices.size)
                val maxNearbyShown = if (expanded) nearbyPeers.size else if (pairedDevices.isEmpty()) minOf(2, nearbyPeers.size) else 0
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .heightIn(max = if (expanded) 280.dp else 160.dp)
                        .verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    pairedDevices.take(maxPairedShown).forEach { dev ->
                        Surface(
                            onClick = { onOpenDevice(dev.deviceId) },
                            modifier = Modifier.fillMaxWidth(),
                            shape = RoundedCornerShape(16.dp),
                            color = palette.surfaceTonal,
                        ) {
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .defaultMinSize(minHeight = 56.dp)
                                    .padding(horizontal = 12.dp, vertical = 8.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(10.dp),
                            ) {
                                Surface(
                                    modifier = Modifier.size(38.dp),
                                    shape = RoundedCornerShape(12.dp),
                                    color = palette.surface,
                                ) {
                                    Box(contentAlignment = Alignment.Center) {
                                        DeviceRoleIcon(role = dev.role, color = palette.accent, size = 19.dp)
                                    }
                                }
                                Column(modifier = Modifier.weight(1f)) {
                                    Row(
                                        verticalAlignment = Alignment.CenterVertically,
                                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                                    ) {
                                        Text(
                                            text = dev.displayName,
                                            fontSize = 14.sp,
                                            fontWeight = FontWeight.SemiBold,
                                            color = palette.textPrimary,
                                            maxLines = 1,
                                            overflow = TextOverflow.Ellipsis,
                                        )
                                        if (dev.isOnline) {
                                            StatusChip("READY", palette.loadingTonal, palette.onLoadingTonal)
                                        }
                                    }
                                    Text(
                                        text = buildString {
                                            append(if (dev.isOnline) "Wi-Fi ready" else "On standby")
                                            if (dev.autoAcceptFiles) append(" · auto-accept")
                                        },
                                        fontSize = 12.sp,
                                        color = palette.textMuted,
                                    )
                                }
                                Button(
                                    onClick = { onSendToDevice(dev.deviceId) },
                                    modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                                    shape = RoundedCornerShape(999.dp),
                                    colors = ButtonDefaults.buttonColors(
                                        containerColor = palette.surface,
                                        contentColor = palette.accent,
                                    ),
                                    contentPadding = PaddingValues(horizontal = 14.dp),
                                ) {
                                    Text("Send", fontSize = 13.sp, fontWeight = FontWeight.Bold)
                                }
                            }
                        }
                    }

                    nearbyPeers.take(maxNearbyShown).forEach { peer ->
                        Surface(
                            modifier = Modifier.fillMaxWidth(),
                            shape = RoundedCornerShape(16.dp),
                            color = palette.surfaceTonal,
                        ) {
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .defaultMinSize(minHeight = 56.dp)
                                    .padding(horizontal = 12.dp, vertical = 8.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(10.dp),
                            ) {
                                Surface(
                                    modifier = Modifier.size(38.dp),
                                    shape = RoundedCornerShape(12.dp),
                                    color = palette.surface,
                                ) {
                                    Box(contentAlignment = Alignment.Center) {
                                        DeviceRoleIcon(role = peer.role, color = palette.loading, size = 19.dp)
                                    }
                                }
                                Column(modifier = Modifier.weight(1f)) {
                                    Row(
                                        verticalAlignment = Alignment.CenterVertically,
                                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                                    ) {
                                        Text(
                                            text = peer.displayName,
                                            fontSize = 14.sp,
                                            fontWeight = FontWeight.SemiBold,
                                            color = palette.textPrimary,
                                            maxLines = 1,
                                            overflow = TextOverflow.Ellipsis,
                                        )
                                        StatusChip("NEARBY", palette.surface, palette.textSecondary)
                                    }
                                    Text(
                                        text = "${peer.address} · ${peer.rssi} dBm",
                                        fontSize = 12.sp,
                                        color = palette.textMuted,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                    )
                                }
                                Button(
                                    onClick = { onPairPeer(peer.address) },
                                    modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                                    shape = RoundedCornerShape(999.dp),
                                    colors = ButtonDefaults.buttonColors(
                                        containerColor = palette.accent,
                                        contentColor = palette.onAccent,
                                    ),
                                    contentPadding = PaddingValues(horizontal = 16.dp),
                                ) {
                                    Text("Pair", fontSize = 13.sp, fontWeight = FontWeight.Bold)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun SendScreen(
    snapshot: CoreSnapshot,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current
    var textComposerOpen by rememberSaveable { mutableStateOf(false) }
    var textDraft by rememberSaveable { mutableStateOf("") }
    val queue = snapshot.sendQueue
    val totalQueueBytes = queue.sumOf { it.sizeBytes }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = KeynearSpacing.Lg, vertical = KeynearSpacing.Sm),
        verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
    ) {
        Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                SendSelectionTile(
                    title = "File",
                    subtitle = "Pick files",
                    active = false,
                    onClick = actions.onPickFiles,
                    modifier = Modifier.weight(1f),
                    icon = { IconFileText(color = palette.accent) },
                )
                SendSelectionTile(
                    title = "Folder",
                    subtitle = "Pick folder",
                    active = false,
                    onClick = actions.onPickFolder,
                    modifier = Modifier.weight(1f),
                    icon = { IconFolder(color = palette.accent) },
                )
            }
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                SendSelectionTile(
                    title = "Clipboard",
                    subtitle = "Paste current",
                    active = false,
                    onClick = actions.onStageClipboard,
                    modifier = Modifier.weight(1f),
                    icon = { IconClipboard(color = palette.accent) },
                )
                SendSelectionTile(
                    title = "Text",
                    subtitle = "Write snippet",
                    active = textComposerOpen,
                    onClick = { textComposerOpen = !textComposerOpen },
                    modifier = Modifier.weight(1f),
                    icon = {
                        IconTextNote(
                            color = if (textComposerOpen) palette.onAccent else palette.accent,
                        )
                    },
                )
            }
        }

        AnimatedVisibility(visible = textComposerOpen) {
            TactileSurface(cornerRadius = 20.dp) {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(KeynearSpacing.Lg),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    Text(
                        text = "Write snippet",
                        fontSize = 14.sp,
                        fontWeight = FontWeight.Bold,
                        color = palette.textPrimary,
                    )
                    OutlinedTextField(
                        value = textDraft,
                        onValueChange = { textDraft = it },
                        modifier = Modifier.fillMaxWidth(),
                        placeholder = { Text("Paste a note, link, or command...") },
                        minLines = 3,
                        shape = RoundedCornerShape(14.dp),
                    )
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.End,
                    ) {
                        TextButton(
                            onClick = {
                                textComposerOpen = false
                                textDraft = ""
                            },
                            modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                        ) {
                            Text("Cancel", color = palette.textSecondary)
                        }
                        Spacer(modifier = Modifier.width(8.dp))
                        Button(
                            onClick = {
                                val clean = textDraft.trim()
                                if (clean.isNotEmpty()) {
                                    actions.onStageTextSnippet(clean)
                                    textDraft = ""
                                    textComposerOpen = false
                                }
                            },
                            enabled = textDraft.trim().isNotEmpty(),
                            modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                            shape = RoundedCornerShape(999.dp),
                            colors = ButtonDefaults.buttonColors(
                                containerColor = palette.accent,
                                contentColor = palette.onAccent,
                            ),
                        ) {
                            Text("Add to queue", fontWeight = FontWeight.Bold)
                        }
                    }
                }
            }
        }

        TactileSurface(cornerRadius = 20.dp) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(KeynearSpacing.Lg),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = if (queue.isEmpty()) {
                            "Queue empty"
                        } else {
                            "Queued (${queue.size}) · ${formatBytes(totalQueueBytes)}"
                        },
                        fontSize = 15.sp,
                        fontWeight = FontWeight.Bold,
                        color = palette.textPrimary,
                    )
                    if (queue.isNotEmpty()) {
                        TextButton(
                            onClick = actions.onClearQueue,
                            modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                        ) {
                            Text("Clear", color = palette.error, fontWeight = FontWeight.Bold)
                        }
                    }
                }
                if (queue.isEmpty()) {
                    Text(
                        text = "Pick files, folders, clipboard items, or write text above before choosing a target device.",
                        fontSize = 13.sp,
                        color = palette.textMuted,
                    )
                } else {
                    queue.forEach { item ->
                        SendQueueRow(
                            item = item,
                            onRemove = { actions.onRemoveQueueItem(item.id) },
                        )
                    }
                }
            }
        }

        SectionHeader("SEND TO")

        if (snapshot.pairedDevices.isEmpty()) {
            TactileSurface(cornerRadius = 20.dp) {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(KeynearSpacing.Lg),
                    verticalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    Text(
                        text = "No paired devices yet",
                        fontSize = 15.sp,
                        fontWeight = FontWeight.Bold,
                        color = palette.textPrimary,
                    )
                    Text(
                        text = "Open Receive, turn visibility to Everyone, and pair with a nearby laptop or phone.",
                        fontSize = 13.sp,
                        color = palette.textMuted,
                    )
                }
            }
        } else {
            snapshot.pairedDevices.forEach { dev ->
                TactileSurface(
                    cornerRadius = 18.dp,
                    onClick = { actions.onOpenDeviceDetail(dev.deviceId) },
                ) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .defaultMinSize(minHeight = 64.dp)
                            .padding(horizontal = 14.dp, vertical = 10.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                    ) {
                        Surface(
                            modifier = Modifier.size(44.dp),
                            shape = RoundedCornerShape(14.dp),
                            color = palette.surfaceTonal,
                        ) {
                            Box(contentAlignment = Alignment.Center) {
                                DeviceRoleIcon(
                                    role = dev.role,
                                    color = if (dev.isOnline) palette.accent else palette.textSecondary,
                                    size = 22.dp,
                                )
                            }
                        }
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = dev.displayName,
                                fontSize = 15.sp,
                                fontWeight = FontWeight.SemiBold,
                                color = palette.textPrimary,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                            Text(
                                text = if (dev.isOnline) "Online · Wi-Fi" else "Standby · BLE",
                                fontSize = 12.sp,
                                color = if (dev.isOnline) palette.loading else palette.textMuted,
                            )
                        }
                        Button(
                            onClick = { actions.onSendQueueToPeer(dev) },
                            enabled = queue.isNotEmpty(),
                            modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                            shape = RoundedCornerShape(999.dp),
                            colors = ButtonDefaults.buttonColors(
                                containerColor = palette.accent,
                                contentColor = palette.onAccent,
                            ),
                            contentPadding = PaddingValues(horizontal = 18.dp),
                        ) {
                            Text("Send", fontSize = 13.sp, fontWeight = FontWeight.Bold)
                        }
                    }
                }
            }
        }

        Spacer(modifier = Modifier.height(12.dp))
    }
}

@Composable
private fun SendSelectionTile(
    title: String,
    subtitle: String,
    active: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    icon: @Composable () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    TactileSurface(
        modifier = modifier.defaultMinSize(minHeight = 92.dp),
        cornerRadius = 18.dp,
        color = if (active) palette.accent else palette.surface,
        onClick = onClick,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(14.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Surface(
                modifier = Modifier.size(34.dp),
                shape = RoundedCornerShape(10.dp),
                color = if (active) palette.onAccent.copy(alpha = 0.2f) else palette.surfaceTonal,
            ) {
                Box(contentAlignment = Alignment.Center) {
                    icon()
                }
            }
            Column {
                Text(
                    text = title,
                    fontSize = 14.sp,
                    fontWeight = FontWeight.Bold,
                    color = if (active) palette.onAccent else palette.textPrimary,
                )
                Text(
                    text = subtitle,
                    fontSize = 12.sp,
                    color = if (active) palette.onAccent.copy(alpha = 0.85f) else palette.textMuted,
                )
            }
        }
    }
}

@Composable
private fun SendQueueRow(
    item: SendQueueItemUi,
    onRemove: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(14.dp),
        color = palette.surfaceTonal,
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(start = 12.dp, top = 6.dp, bottom = 6.dp, end = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            when (item.kind) {
                SendItemKind.FILE -> IconFileText(color = palette.accent, size = 18.dp)
                SendItemKind.FOLDER -> IconFolder(color = palette.accent, size = 18.dp)
                SendItemKind.CLIPBOARD -> IconClipboard(color = palette.accent, size = 18.dp)
                SendItemKind.TEXT -> IconTextNote(color = palette.accent, size = 18.dp)
            }
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = item.name,
                    fontSize = 13.sp,
                    fontWeight = FontWeight.SemiBold,
                    color = palette.textPrimary,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    text = "${item.mime} · ${formatBytes(item.sizeBytes)}",
                    fontSize = 11.sp,
                    color = palette.textMuted,
                )
            }
            Box(
                modifier = Modifier
                    .size(KeynearSpacing.MinTouchTarget)
                    .clip(CircleShape)
                    .clickable(onClick = onRemove),
                contentAlignment = Alignment.Center,
            ) {
                IconClose(color = palette.textMuted, size = 16.dp)
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun SettingsScreen(
    snapshot: CoreSnapshot,
    checklist: PermissionsChecklistState,
    actions: KeynearActions,
    onShowMessage: (String) -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val cfg = snapshot.daemonConfig
    var mimeExpanded by rememberSaveable { mutableStateOf(false) }
    var mimeDraft by rememberSaveable { mutableStateOf("") }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = KeynearSpacing.Lg, vertical = KeynearSpacing.Sm),
        verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
    ) {
        TactileSurface(
            cornerRadius = 20.dp,
            color = if (checklist.missingPermissionsCount > 0) palette.warningTonal else palette.surface,
            onClick = actions.onOpenPermissions,
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .defaultMinSize(minHeight = 60.dp)
                    .padding(horizontal = 16.dp, vertical = 12.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                IconShield(
                    color = if (checklist.missingPermissionsCount > 0) palette.onWarningTonal else palette.accent,
                    size = 20.dp,
                )
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = "Permissions",
                        fontSize = 15.sp,
                        fontWeight = FontWeight.Bold,
                        color = if (checklist.missingPermissionsCount > 0) palette.onWarningTonal else palette.textPrimary,
                    )
                    Text(
                        text = if (checklist.missingPermissionsCount == 0) {
                            "All 5 granted"
                        } else {
                            "${checklist.missingPermissionsCount} need attention"
                        },
                        fontSize = 12.sp,
                        color = if (checklist.missingPermissionsCount > 0) palette.onWarningTonal else palette.textMuted,
                    )
                }
                IconChevronRight(
                    color = if (checklist.missingPermissionsCount > 0) palette.onWarningTonal else palette.textMuted,
                    size = 18.dp,
                )
            }
        }

        SectionHeader("GENERAL & STORAGE")
        TactileSurface(cornerRadius = 20.dp) {
            Column(modifier = Modifier.fillMaxWidth()) {
                SettingsActionRow(
                    title = "Visibility mode",
                    subtitle = when (snapshot.visibility) {
                        VisibilityState.HIDDEN -> "Hidden from everyone"
                        VisibilityState.PAIRED_ONLY -> "Visible to paired devices"
                        VisibilityState.EVERYONE_NEARBY -> "Visible to everyone (${formatCountdown(snapshot.everyoneRemainingSecs)})"
                    },
                    valueLabel = when (snapshot.visibility) {
                        VisibilityState.HIDDEN -> "Hidden"
                        VisibilityState.PAIRED_ONLY -> "Paired"
                        VisibilityState.EVERYONE_NEARBY -> "Everyone"
                    },
                    onClick = {
                        val next = when (snapshot.visibility) {
                            VisibilityState.PAIRED_ONLY -> VisibilityState.EVERYONE_NEARBY
                            VisibilityState.EVERYONE_NEARBY -> VisibilityState.HIDDEN
                            VisibilityState.HIDDEN -> VisibilityState.PAIRED_ONLY
                        }
                        actions.onSetVisibility(next)
                    },
                )
                HorizontalDivider(color = palette.surfaceTonal, modifier = Modifier.padding(horizontal = 16.dp))
                SettingsActionRow(
                    title = "Save folder",
                    subtitle = cfg.incomingDirectory,
                    valueLabel = "Change",
                    onClick = actions.onPickSaveFolder,
                )
                HorizontalDivider(color = palette.surfaceTonal, modifier = Modifier.padding(horizontal = 16.dp))
                SettingsSwitchRow(
                    title = "Auto-accept from paired",
                    subtitle = "Save incoming transfers without asking",
                    checked = cfg.autoAcceptFromPaired,
                    onCheckedChange = { enabled ->
                        actions.onUpdateConfig { it.copy(autoAcceptFromPaired = enabled) }
                    },
                )
            }
        }

        SectionHeader("CLIPBOARD")
        TactileSurface(cornerRadius = 20.dp) {
            Column(modifier = Modifier.fillMaxWidth()) {
                SettingsSwitchRow(
                    title = "Clipboard sync",
                    subtitle = "Share text across paired devices",
                    checked = cfg.clipboardSyncEnabled,
                    onCheckedChange = { enabled ->
                        actions.onUpdateConfig { it.copy(clipboardSyncEnabled = enabled) }
                    },
                )
                HorizontalDivider(color = palette.surfaceTonal, modifier = Modifier.padding(horizontal = 16.dp))
                SettingsSwitchRow(
                    title = "Sync clipboard images",
                    subtitle = "Include screenshots and copied photos",
                    checked = cfg.clipboardImagesEnabled,
                    onCheckedChange = { enabled ->
                        actions.onUpdateConfig { it.copy(clipboardImagesEnabled = enabled) }
                    },
                )
                HorizontalDivider(color = palette.surfaceTonal, modifier = Modifier.padding(horizontal = 16.dp))
                SettingsSwitchRow(
                    title = "One-time code passthrough",
                    subtitle = "Forward SMS and login OTP notifications",
                    checked = cfg.otpPassthroughEnabled,
                    onCheckedChange = { enabled ->
                        actions.onUpdateConfig { it.copy(otpPassthroughEnabled = enabled) }
                    },
                )
                HorizontalDivider(color = palette.surfaceTonal, modifier = Modifier.padding(horizontal = 16.dp))
                SettingsActionRow(
                    title = "Ignored clipboard formats",
                    subtitle = "${cfg.ignoredMimeTypes.size} rules",
                    valueLabel = if (mimeExpanded) "Hide" else "Edit",
                    onClick = { mimeExpanded = !mimeExpanded },
                )
                AnimatedVisibility(visible = mimeExpanded) {
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(start = 16.dp, end = 16.dp, bottom = 14.dp),
                        verticalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        FlowRow(
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            cfg.ignoredMimeTypes.forEach { mime ->
                                Surface(
                                    onClick = {
                                        actions.onUpdateConfig { current ->
                                            current.copy(ignoredMimeTypes = current.ignoredMimeTypes - mime)
                                        }
                                        onShowMessage("Removed $mime")
                                    },
                                    shape = RoundedCornerShape(999.dp),
                                    color = palette.surfaceTonal,
                                ) {
                                    Row(
                                        modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
                                        verticalAlignment = Alignment.CenterVertically,
                                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                                    ) {
                                        Text(
                                            text = mime,
                                            fontSize = 12.sp,
                                            fontFamily = FontFamily.Monospace,
                                            color = palette.textPrimary,
                                        )
                                        IconClose(color = palette.textMuted, size = 13.dp)
                                    }
                                }
                            }
                        }
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            OutlinedTextField(
                                value = mimeDraft,
                                onValueChange = { mimeDraft = it },
                                modifier = Modifier.weight(1f),
                                placeholder = { Text("Add MIME rule...") },
                                singleLine = true,
                                shape = RoundedCornerShape(12.dp),
                            )
                            Button(
                                onClick = {
                                    val clean = mimeDraft.trim()
                                    if (clean.isNotEmpty()) {
                                        actions.onUpdateConfig { current ->
                                            if (current.ignoredMimeTypes.contains(clean)) {
                                                current
                                            } else {
                                                current.copy(ignoredMimeTypes = current.ignoredMimeTypes + clean)
                                            }
                                        }
                                        mimeDraft = ""
                                        onShowMessage("Added $clean")
                                    }
                                },
                                modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                                shape = RoundedCornerShape(999.dp),
                                colors = ButtonDefaults.buttonColors(
                                    containerColor = palette.accent,
                                    contentColor = palette.onAccent,
                                ),
                            ) {
                                Text("Add", fontWeight = FontWeight.Bold)
                            }
                        }
                    }
                }
            }
        }

        SectionHeader("AUDIO & NOTIFICATIONS")
        TactileSurface(cornerRadius = 20.dp) {
            Column(modifier = Modifier.fillMaxWidth()) {
                SettingsSwitchRow(
                    title = "Audio handoff",
                    subtitle = "Continue media playback on nearby devices",
                    checked = cfg.audioHandoffEnabled,
                    onCheckedChange = { enabled ->
                        actions.onUpdateConfig { it.copy(audioHandoffEnabled = enabled) }
                    },
                )
                HorizontalDivider(color = palette.surfaceTonal, modifier = Modifier.padding(horizontal = 16.dp))
                SettingsSwitchRow(
                    title = "Notify when auto-accepted",
                    subtitle = "Post silent notification after file saves",
                    checked = cfg.notifyAutoAcceptComplete,
                    onCheckedChange = { enabled ->
                        actions.onUpdateConfig { it.copy(notifyAutoAcceptComplete = enabled) }
                    },
                )
            }
        }

        SectionHeader("APPEARANCE")
        TactileSurface(cornerRadius = 20.dp) {
            SettingsSwitchRow(
                title = "Dark theme",
                subtitle = "Use dark canvas and ink surfaces",
                checked = cfg.darkTheme,
                onCheckedChange = { enabled ->
                    actions.onUpdateConfig { it.copy(darkTheme = enabled) }
                },
            )
        }

        if (snapshot.pairedDevices.isNotEmpty()) {
            SectionHeader("PAIRED DEVICES")
            TactileSurface(cornerRadius = 20.dp) {
                Column(modifier = Modifier.fillMaxWidth()) {
                    snapshot.pairedDevices.forEachIndexed { idx, dev ->
                        if (idx > 0) {
                            HorizontalDivider(
                                color = palette.surfaceTonal,
                                modifier = Modifier.padding(horizontal = 16.dp),
                            )
                        }
                        SettingsActionRow(
                            title = dev.displayName,
                            subtitle = buildString {
                                append(if (dev.isOnline) "Online · Wi-Fi" else "Standby")
                                if (dev.autoAcceptFiles) append(" · auto-accept")
                            },
                            valueLabel = "Details",
                            onClick = { actions.onOpenDeviceDetail(dev.deviceId) },
                        )
                    }
                }
            }
        }

        Spacer(modifier = Modifier.height(12.dp))
    }
}

@Composable
private fun PermissionsScreen(
    checklist: PermissionsChecklistState,
    actions: KeynearActions,
) {
    val rows = listOf(
        Triple(
            "Nearby Devices & Wi-Fi",
            "Required to advertise over BLE and connect on local Wi-Fi.",
            checklist.nearbyGranted to actions.onRequestNearbyOnboardingPermissions,
        ),
        Triple(
            "Local Network",
            "Required on Android 16+ for peer mDNS and QUIC LAN transfers.",
            checklist.localNetworkGranted to actions.onRequestLocalNetworkPermission,
        ),
        Triple(
            "Companion Device",
            "Recommended for reliable background wakeups when transfers start.",
            checklist.companionAssociated to actions.onOpenAppSettings,
        ),
        Triple(
            "Notifications",
            "Required to confirm incoming pairings and show active transfers.",
            checklist.notificationsGranted to actions.onRequestNotificationsPermission,
        ),
        Triple(
            "Unrestricted Background",
            "Recommended so Android does not pause multi-file transfers.",
            checklist.batteryOptimizationIgnored to actions.onRequestBatteryOptimization,
        ),
    )

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = KeynearSpacing.Lg, vertical = KeynearSpacing.Sm),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        rows.forEach { (title, desc, stateAction) ->
            val (granted, onGrant) = stateAction
            PermissionRowCard(
                title = title,
                description = desc,
                granted = granted,
                onAction = onGrant,
            )
        }
    }
}

@Composable
private fun PermissionRowCard(
    title: String,
    description: String,
    granted: Boolean,
    onAction: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    TactileSurface(cornerRadius = 20.dp) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Lg),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Column(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Text(
                        text = title,
                        fontSize = 15.sp,
                        fontWeight = FontWeight.Bold,
                        color = palette.textPrimary,
                    )
                    StatusChip(
                        label = if (granted) "Granted" else "Missing",
                        bg = if (granted) palette.loadingTonal else palette.warningTonal,
                        fg = if (granted) palette.onLoadingTonal else palette.onWarningTonal,
                    )
                }
                Text(
                    text = description,
                    fontSize = 12.sp,
                    color = palette.textMuted,
                )
            }
            if (!granted) {
                Button(
                    onClick = onAction,
                    modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                    shape = RoundedCornerShape(999.dp),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = palette.accent,
                        contentColor = palette.onAccent,
                    ),
                ) {
                    Text("Allow", fontSize = 13.sp, fontWeight = FontWeight.Bold)
                }
            }
        }
    }
}

@Composable
private fun DeviceDetailScreen(
    device: PairedDeviceUi,
    confirmUnpair: Boolean,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current
    var draftName by remember(device.deviceId, device.displayName) { mutableStateOf(device.displayName) }
    var renameError by remember { mutableStateOf<String?>(null) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = KeynearSpacing.Lg, vertical = KeynearSpacing.Sm),
        verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
    ) {
        TactileSurface(cornerRadius = 24.dp) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(20.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Surface(
                    modifier = Modifier.size(64.dp),
                    shape = CircleShape,
                    color = palette.surfaceTonal,
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        DeviceRoleIcon(
                            role = device.role,
                            color = if (device.isOnline) palette.accent else palette.textSecondary,
                            size = 30.dp,
                        )
                    }
                }
                Text(
                    text = device.displayName,
                    fontSize = 20.sp,
                    fontWeight = FontWeight.ExtraBold,
                    color = palette.textPrimary,
                )
                StatusChip(
                    label = if (device.isOnline) "Online · Wi-Fi" else "On standby",
                    bg = if (device.isOnline) palette.loadingTonal else palette.surfaceTonal,
                    fg = if (device.isOnline) palette.onLoadingTonal else palette.textSecondary,
                )
                Button(
                    onClick = actions.onJumpToSendTab,
                    modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                    shape = RoundedCornerShape(999.dp),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = palette.accent,
                        contentColor = palette.onAccent,
                    ),
                    contentPadding = PaddingValues(horizontal = 24.dp),
                ) {
                    Text("Send files", fontSize = 14.sp, fontWeight = FontWeight.Bold)
                }
            }
        }

        TactileSurface(cornerRadius = 20.dp) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(KeynearSpacing.Lg),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text(
                    text = "Display name",
                    fontSize = 13.sp,
                    fontWeight = FontWeight.Bold,
                    color = palette.textSecondary,
                )
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    OutlinedTextField(
                        value = draftName,
                        onValueChange = {
                            draftName = it
                            renameError = null
                        },
                        modifier = Modifier.weight(1f),
                        singleLine = true,
                        shape = RoundedCornerShape(14.dp),
                    )
                    Button(
                        onClick = {
                            val check = validateDisplayNameInput(draftName)
                            if (check.isSuccess) {
                                actions.onRenameDevice(device.deviceId, check.getOrThrow())
                                renameError = null
                            } else {
                                renameError = check.exceptionOrNull()?.message ?: "Invalid name"
                            }
                        },
                        modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                        shape = RoundedCornerShape(999.dp),
                        colors = ButtonDefaults.buttonColors(
                            containerColor = palette.accent,
                            contentColor = palette.onAccent,
                        ),
                    ) {
                        Text("Save", fontWeight = FontWeight.Bold)
                    }
                }
                renameError?.let { msg ->
                    Text(msg, fontSize = 12.sp, color = palette.error)
                }
                HorizontalDivider(color = palette.surfaceTonal)
                SettingsSwitchRow(
                    title = "Auto-accept files",
                    subtitle = "Save transfers from ${device.displayName} automatically",
                    checked = device.autoAcceptFiles,
                    onCheckedChange = { enabled ->
                        actions.onToggleAutoAccept(device.deviceId, enabled)
                    },
                    horizontalPadding = 0.dp,
                )
                HorizontalDivider(color = palette.surfaceTonal)
                MetadataRow("Public key fingerprint", device.publicKeyFingerprint)
                MetadataRow("Bluetooth address", device.lastBleAddress.ifBlank { "Unavailable" })
                MetadataRow("Local Wi-Fi endpoint", device.lastLanAddress.ifBlank { "Standby" })
                MetadataRow("Paired", device.pairedAt)
            }
        }

        OutlinedButton(
            onClick = { actions.onResetBleBond(device.deviceId) },
            modifier = Modifier
                .fillMaxWidth()
                .defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
            shape = RoundedCornerShape(999.dp),
        ) {
            Text("Reset Bluetooth bond", color = palette.textPrimary, fontWeight = FontWeight.Bold)
        }

        if (confirmUnpair) {
            Surface(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(20.dp),
                color = palette.errorTonal,
            ) {
                Column(
                    modifier = Modifier.padding(KeynearSpacing.Lg),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    Text(
                        text = "Remove ${device.displayName}? You will need to pair again before sharing.",
                        fontSize = 14.sp,
                        fontWeight = FontWeight.SemiBold,
                        color = palette.onDangerTonal,
                    )
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Button(
                            onClick = { actions.onConfirmUnpair(device.deviceId) },
                            modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                            shape = RoundedCornerShape(999.dp),
                            colors = ButtonDefaults.buttonColors(
                                containerColor = palette.error,
                                contentColor = palette.onAccent,
                            ),
                        ) {
                            Text("Confirm remove", fontWeight = FontWeight.Bold)
                        }
                        TextButton(
                            onClick = actions.onCancelUnpair,
                            modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                        ) {
                            Text("Cancel", color = palette.onDangerTonal)
                        }
                    }
                }
            }
        } else {
            Button(
                onClick = { actions.onRequestUnpair(device.deviceId) },
                modifier = Modifier
                    .fillMaxWidth()
                    .defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                shape = RoundedCornerShape(999.dp),
                colors = ButtonDefaults.buttonColors(
                    containerColor = palette.errorTonal,
                    contentColor = palette.onDangerTonal,
                ),
            ) {
                Text("Remove device", fontWeight = FontWeight.Bold)
            }
        }

        Spacer(modifier = Modifier.height(12.dp))
    }
}

@Composable
private fun BluetoothWarningCard(
    status: BluetoothStatus,
    message: String?,
    onRequestPermissions: () -> Unit,
    onOpenBluetoothSettings: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val resolved = message ?: when (status) {
        BluetoothStatus.MISSING_PERMISSION -> "Bluetooth permissions are required for discovery and pairing."
        BluetoothStatus.ADAPTER_DISABLED -> "Bluetooth is turned off. Enable Bluetooth to discover nearby peers."
        BluetoothStatus.UNAVAILABLE -> "Bluetooth Low Energy hardware is unavailable on this device."
        BluetoothStatus.READY -> ""
    }
    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.Xl),
        color = palette.warningTonal,
    ) {
        Column(
            modifier = Modifier.padding(KeynearSpacing.Lg),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(text = resolved, fontSize = 13.sp, color = palette.onWarningTonal)
            if (status == BluetoothStatus.MISSING_PERMISSION) {
                Button(
                    onClick = onRequestPermissions,
                    modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                    shape = RoundedCornerShape(999.dp),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = palette.accent,
                        contentColor = palette.onAccent,
                    ),
                ) {
                    Text("Allow Bluetooth")
                }
            } else if (status == BluetoothStatus.ADAPTER_DISABLED) {
                Button(
                    onClick = onOpenBluetoothSettings,
                    modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                    shape = RoundedCornerShape(999.dp),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = palette.accent,
                        contentColor = palette.onAccent,
                    ),
                ) {
                    Text("Enable Bluetooth")
                }
            }
        }
    }
}

@Composable
private fun LocalNetworkWarningCard(
    status: LocalNetworkStatus,
    message: String?,
    onRequestPermission: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val resolved = message ?: when (status) {
        LocalNetworkStatus.MISSING_PERMISSION -> "Allow local network permission to connect with paired peers on Wi-Fi."
        LocalNetworkStatus.BLOCKED_LNP -> "Allow local network access so Keynear can discover paired devices on Wi-Fi."
        LocalNetworkStatus.UNAVAILABLE -> "Local Wi-Fi network discovery is unavailable."
        LocalNetworkStatus.READY -> ""
    }
    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.Xl),
        color = palette.warningTonal,
    ) {
        Column(
            modifier = Modifier.padding(KeynearSpacing.Lg),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(text = resolved, fontSize = 13.sp, color = palette.onWarningTonal)
            if (status == LocalNetworkStatus.MISSING_PERMISSION || status == LocalNetworkStatus.BLOCKED_LNP) {
                Button(
                    onClick = onRequestPermission,
                    modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                    shape = RoundedCornerShape(999.dp),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = palette.accent,
                        contentColor = palette.onAccent,
                    ),
                ) {
                    Text("Allow local Wi-Fi")
                }
            }
        }
    }
}

@Composable
private fun PendingPairingCard(
    pending: PendingPairingUi,
    onApprove: () -> Unit,
    onReject: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    TactileSurface(cornerRadius = 20.dp) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Lg),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = "Pair with ${pending.peerName}?",
                        fontSize = 16.sp,
                        fontWeight = FontWeight.Bold,
                        color = palette.textPrimary,
                    )
                    Text(
                        text = "${pending.peerAddress} · ${pending.remainingSecs}s",
                        fontSize = 12.sp,
                        color = palette.textMuted,
                    )
                }
                Text(
                    text = PairingNotificationManager.formatCode(pending.verificationCode),
                    fontSize = 24.sp,
                    fontFamily = FontFamily.Monospace,
                    fontWeight = FontWeight.ExtraBold,
                    color = palette.accent,
                )
            }
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                Button(
                    onClick = onApprove,
                    modifier = Modifier
                        .weight(1f)
                        .defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                    shape = RoundedCornerShape(999.dp),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = palette.accent,
                        contentColor = palette.onAccent,
                    ),
                ) {
                    Text("Approve", fontWeight = FontWeight.Bold)
                }
                OutlinedButton(
                    onClick = onReject,
                    modifier = Modifier
                        .weight(1f)
                        .defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                    shape = RoundedCornerShape(999.dp),
                ) {
                    Text("Decline", color = palette.textPrimary, fontWeight = FontWeight.Bold)
                }
            }
        }
    }
}

@Composable
private fun ActiveTransfersCard(
    transfers: List<TransferUi>,
    onAccept: (String) -> Unit,
    onDecline: (String) -> Unit,
    onCancel: (String) -> Unit,
) {
    val palette = LocalKeynearPalette.current
    TactileSurface(cornerRadius = 20.dp) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Lg),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Text(
                text = "Transfers",
                fontSize = 15.sp,
                fontWeight = FontWeight.Bold,
                color = palette.textPrimary,
            )
            transfers.forEach { transfer ->
                Surface(
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(14.dp),
                    color = palette.surfaceTonal,
                ) {
                    Column(
                        modifier = Modifier.padding(12.dp),
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Column(modifier = Modifier.weight(1f)) {
                                Text(
                                    text = transfer.summaryName,
                                    fontSize = 14.sp,
                                    fontWeight = FontWeight.SemiBold,
                                    color = palette.textPrimary,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                )
                                Text(
                                    text = "${if (transfer.direction == TransferDirectionUi.SEND) "To" else "From"} ${transfer.peerDisplayName} · ${formatBytes(transfer.totalBytes)}",
                                    fontSize = 12.sp,
                                    color = palette.textMuted,
                                )
                            }
                            StatusChip(
                                label = transfer.state.label(),
                                bg = palette.loadingTonal,
                                fg = palette.onLoadingTonal,
                            )
                        }
                        if (transfer.totalBytes > 0L && transfer.state == TransferStateUi.ACTIVE) {
                            val fraction = (transfer.transferredBytes.toFloat() / transfer.totalBytes.toFloat())
                                .coerceIn(0f, 1f)
                            LinearProgressIndicator(
                                progress = { fraction },
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .height(6.dp)
                                    .clip(RoundedCornerShape(999.dp)),
                                color = palette.loading,
                                trackColor = palette.trackWell,
                            )
                        }
                        if (transfer.state == TransferStateUi.PENDING_CONSENT &&
                            transfer.direction == TransferDirectionUi.RECEIVE
                        ) {
                            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                Button(
                                    onClick = { onAccept(transfer.transferId) },
                                    modifier = Modifier
                                        .weight(1f)
                                        .defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                                    shape = RoundedCornerShape(999.dp),
                                    colors = ButtonDefaults.buttonColors(
                                        containerColor = palette.accent,
                                        contentColor = palette.onAccent,
                                    ),
                                ) {
                                    Text("Accept", fontWeight = FontWeight.Bold)
                                }
                                OutlinedButton(
                                    onClick = { onDecline(transfer.transferId) },
                                    modifier = Modifier
                                        .weight(1f)
                                        .defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                                    shape = RoundedCornerShape(999.dp),
                                ) {
                                    Text("Decline", color = palette.textPrimary)
                                }
                            }
                        } else if (!transfer.state.isTerminal()) {
                            OutlinedButton(
                                onClick = { onCancel(transfer.transferId) },
                                modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                                shape = RoundedCornerShape(999.dp),
                            ) {
                                Text("Cancel", color = palette.error)
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun TactileSurface(
    modifier: Modifier = Modifier,
    cornerRadius: Dp = 20.dp,
    color: Color? = null,
    onClick: (() -> Unit)? = null,
    content: @Composable () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val resolvedColor = color ?: palette.surface
    val shape = RoundedCornerShape(cornerRadius)
    val rimColor = if (palette.isDark) Color(0x16FFFFFE) else Color(0xCCFFFFFE)
    val elevation = if (palette.isDark) 4.dp else 3.dp
    if (onClick != null) {
        Surface(
            onClick = onClick,
            modifier = modifier,
            shape = shape,
            color = resolvedColor,
            border = BorderStroke(1.dp, rimColor),
            shadowElevation = elevation,
            content = content,
        )
    } else {
        Surface(
            modifier = modifier,
            shape = shape,
            color = resolvedColor,
            border = BorderStroke(1.dp, rimColor),
            shadowElevation = elevation,
            content = content,
        )
    }
}

@Composable
private fun SectionHeader(label: String) {
    val palette = LocalKeynearPalette.current
    Text(
        text = label,
        fontSize = 11.sp,
        fontWeight = FontWeight.Bold,
        letterSpacing = 0.8.sp,
        color = palette.textMuted,
        modifier = Modifier.padding(horizontal = 4.dp, vertical = 2.dp),
    )
}

@Composable
private fun SettingsActionRow(
    title: String,
    subtitle: String,
    valueLabel: String,
    onClick: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .defaultMinSize(minHeight = 60.dp)
            .clickable(onClick = onClick)
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = title,
                fontSize = 15.sp,
                fontWeight = FontWeight.SemiBold,
                color = palette.textPrimary,
            )
            Text(
                text = subtitle,
                fontSize = 12.sp,
                color = palette.textMuted,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            Text(
                text = valueLabel,
                fontSize = 13.sp,
                fontWeight = FontWeight.SemiBold,
                color = palette.accent,
            )
            IconChevronRight(color = palette.textMuted, size = 16.dp)
        }
    }
}

@Composable
private fun SettingsSwitchRow(
    title: String,
    subtitle: String,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    horizontalPadding: Dp = 16.dp,
) {
    val palette = LocalKeynearPalette.current
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .defaultMinSize(minHeight = 60.dp)
            .clickable(role = Role.Switch) { onCheckedChange(!checked) }
            .padding(horizontal = horizontalPadding, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = title,
                fontSize = 15.sp,
                fontWeight = FontWeight.SemiBold,
                color = palette.textPrimary,
            )
            Text(
                text = subtitle,
                fontSize = 12.sp,
                color = palette.textMuted,
            )
        }
        KeynearSwitch(checked = checked, onCheckedChange = onCheckedChange)
    }
}

@Composable
private fun KeynearSwitch(
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val thumbOffset by animateDpAsState(
        targetValue = if (checked) 24.dp else 4.dp,
        animationSpec = spring(stiffness = 500f),
        label = "switchThumbOffset",
    )
    Box(
        modifier = Modifier
            .defaultMinSize(minWidth = 52.dp, minHeight = KeynearSpacing.MinTouchTarget)
            .clickable(role = Role.Switch) { onCheckedChange(!checked) },
        contentAlignment = Alignment.Center,
    ) {
        Box(
            modifier = Modifier
                .width(52.dp)
                .height(32.dp)
                .clip(RoundedCornerShape(999.dp))
                .background(if (checked) palette.accent else palette.switchTrackOff),
        ) {
            Surface(
                modifier = Modifier
                    .offset(x = thumbOffset, y = 4.dp)
                    .size(24.dp),
                shape = CircleShape,
                color = if (palette.isDark) Color(0xFFF7F6F7) else Color(0xFFFFFFFE),
                shadowElevation = 2.dp,
            ) {}
        }
    }
}

@Composable
private fun MetadataRow(label: String, value: String) {
    val palette = LocalKeynearPalette.current
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = label,
            fontSize = 13.sp,
            color = palette.textMuted,
        )
        Text(
            text = value,
            fontSize = 13.sp,
            fontFamily = FontFamily.Monospace,
            color = palette.textPrimary,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.widthIn(max = 190.dp),
        )
    }
}

@Composable
private fun StatusChip(label: String, bg: Color, fg: Color) {
    Surface(
        shape = RoundedCornerShape(999.dp),
        color = bg,
    ) {
        Text(
            text = label,
            fontSize = 10.sp,
            fontWeight = FontWeight.Bold,
            color = fg,
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp),
        )
    }
}

@Composable
private fun SnackbarPill(message: String) {
    val palette = LocalKeynearPalette.current
    Surface(
        shape = RoundedCornerShape(999.dp),
        color = palette.snackbar,
        shadowElevation = 8.dp,
    ) {
        Text(
            text = message,
            fontSize = 13.sp,
            fontWeight = FontWeight.SemiBold,
            color = palette.onSnackbar,
            maxLines = 2,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.padding(horizontal = 18.dp, vertical = 12.dp),
        )
    }
}

@Composable
private fun LoadingView() {
    val palette = LocalKeynearPalette.current
    Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            CircularProgressIndicator(color = palette.accent)
            Text("Preparing local continuity...", color = palette.textMuted, fontSize = 14.sp)
        }
    }
}

@Composable
private fun ErrorStateCard(
    message: String,
    onRetry: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    Surface(
        modifier = modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.Xl),
        color = palette.errorTonal,
    ) {
        Column(
            modifier = Modifier.padding(KeynearSpacing.Lg),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Text(text = message, fontSize = 13.sp, color = palette.onDangerTonal)
            Button(
                onClick = onRetry,
                modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                shape = RoundedCornerShape(999.dp),
                colors = ButtonDefaults.buttonColors(
                    containerColor = palette.accent,
                    contentColor = palette.onAccent,
                ),
            ) {
                Text("Retry", fontWeight = FontWeight.Bold)
            }
        }
    }
}

@Composable
private fun EmptyDetailFallback(onBack: () -> Unit) {
    val palette = LocalKeynearPalette.current
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(KeynearSpacing.Lg),
        verticalArrangement = Arrangement.spacedBy(12.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text("Device no longer paired.", color = palette.textSecondary)
        Button(
            onClick = onBack,
            modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
            shape = RoundedCornerShape(999.dp),
        ) {
            Text("Back")
        }
    }
}

@Composable
private fun DeviceRoleIcon(role: DeviceRoleUi, color: Color, size: Dp = 20.dp) {
    when (role) {
        DeviceRoleUi.MOBILE -> IconSmartphone(color = color, size = size)
        DeviceRoleUi.DESKTOP -> IconLaptop(color = color, size = size)
    }
}

@Composable
private fun IconInbox(color: Color, size: Dp = 20.dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.1f
        val stroke = Stroke(width = sw, cap = StrokeCap.Round, join = StrokeJoin.Round)
        val boxPath = Path().apply {
            moveTo(w * 0.18f, h * 0.56f)
            lineTo(w * 0.18f, h * 0.82f)
            lineTo(w * 0.82f, h * 0.82f)
            lineTo(w * 0.82f, h * 0.56f)
        }
        drawPath(boxPath, color = color, style = stroke)
        drawLine(color, Offset(w * 0.50f, h * 0.16f), Offset(w * 0.50f, h * 0.60f), strokeWidth = sw, cap = StrokeCap.Round)
        drawLine(color, Offset(w * 0.34f, h * 0.44f), Offset(w * 0.50f, h * 0.60f), strokeWidth = sw, cap = StrokeCap.Round)
        drawLine(color, Offset(w * 0.66f, h * 0.44f), Offset(w * 0.50f, h * 0.60f), strokeWidth = sw, cap = StrokeCap.Round)
    }
}

@Composable
private fun IconSend(color: Color, size: Dp = 20.dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.1f
        val stroke = Stroke(width = sw, cap = StrokeCap.Round, join = StrokeJoin.Round)
        val paper = Path().apply {
            moveTo(w * 0.86f, h * 0.14f)
            lineTo(w * 0.14f, h * 0.44f)
            lineTo(w * 0.48f, h * 0.56f)
            lineTo(w * 0.60f, h * 0.86f)
            close()
        }
        drawPath(paper, color = color, style = stroke)
        drawLine(color, Offset(w * 0.86f, h * 0.14f), Offset(w * 0.48f, h * 0.56f), strokeWidth = sw, cap = StrokeCap.Round)
    }
}

@Composable
private fun IconSliders(color: Color, size: Dp = 20.dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.1f
        listOf(0.28f to 0.62f, 0.50f to 0.34f, 0.72f to 0.68f).forEach { (yFrac, knobX) ->
            drawLine(color, Offset(w * 0.16f, h * yFrac), Offset(w * 0.84f, h * yFrac), strokeWidth = sw, cap = StrokeCap.Round)
            drawCircle(color = color, radius = sw * 1.15f, center = Offset(w * knobX, h * yFrac))
        }
    }
}

@Composable
private fun IconArrowLeft(color: Color, size: Dp = 20.dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.1f
        drawLine(color, Offset(w * 0.80f, h * 0.50f), Offset(w * 0.22f, h * 0.50f), strokeWidth = sw, cap = StrokeCap.Round)
        drawLine(color, Offset(w * 0.46f, h * 0.24f), Offset(w * 0.22f, h * 0.50f), strokeWidth = sw, cap = StrokeCap.Round)
        drawLine(color, Offset(w * 0.46f, h * 0.76f), Offset(w * 0.22f, h * 0.50f), strokeWidth = sw, cap = StrokeCap.Round)
    }
}

@Composable
private fun IconChevronRight(color: Color, size: Dp = 18.dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.11f
        drawLine(color, Offset(w * 0.36f, h * 0.22f), Offset(w * 0.66f, h * 0.50f), strokeWidth = sw, cap = StrokeCap.Round)
        drawLine(color, Offset(w * 0.66f, h * 0.50f), Offset(w * 0.36f, h * 0.78f), strokeWidth = sw, cap = StrokeCap.Round)
    }
}

@Composable
private fun IconChevronDown(color: Color, size: Dp = 18.dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.11f
        drawLine(color, Offset(w * 0.22f, h * 0.38f), Offset(w * 0.50f, h * 0.66f), strokeWidth = sw, cap = StrokeCap.Round)
        drawLine(color, Offset(w * 0.50f, h * 0.66f), Offset(w * 0.78f, h * 0.38f), strokeWidth = sw, cap = StrokeCap.Round)
    }
}

@Composable
private fun IconChevronUp(color: Color, size: Dp = 18.dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.11f
        drawLine(color, Offset(w * 0.22f, h * 0.62f), Offset(w * 0.50f, h * 0.34f), strokeWidth = sw, cap = StrokeCap.Round)
        drawLine(color, Offset(w * 0.50f, h * 0.34f), Offset(w * 0.78f, h * 0.62f), strokeWidth = sw, cap = StrokeCap.Round)
    }
}

@Composable
private fun IconAlertTriangle(color: Color, size: Dp = 18.dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.1f
        val tri = Path().apply {
            moveTo(w * 0.50f, h * 0.14f)
            lineTo(w * 0.88f, h * 0.84f)
            lineTo(w * 0.12f, h * 0.84f)
            close()
        }
        drawPath(tri, color = color, style = Stroke(width = sw, cap = StrokeCap.Round, join = StrokeJoin.Round))
        drawLine(color, Offset(w * 0.50f, h * 0.40f), Offset(w * 0.50f, h * 0.60f), strokeWidth = sw, cap = StrokeCap.Round)
        drawCircle(color = color, radius = sw * 0.65f, center = Offset(w * 0.50f, h * 0.72f))
    }
}

@Composable
private fun IconEyeOff(color: Color, size: Dp = 24.dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.09f
        val stroke = Stroke(width = sw, cap = StrokeCap.Round, join = StrokeJoin.Round)
        drawOval(color, topLeft = Offset(w * 0.14f, h * 0.30f), size = Size(w * 0.72f, h * 0.40f), style = stroke)
        drawLine(color, Offset(w * 0.18f, h * 0.18f), Offset(w * 0.82f, h * 0.82f), strokeWidth = sw, cap = StrokeCap.Round)
    }
}

@Composable
private fun IconShield(color: Color, size: Dp = 20.dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.1f
        val path = Path().apply {
            moveTo(w * 0.50f, h * 0.12f)
            lineTo(w * 0.82f, h * 0.26f)
            lineTo(w * 0.82f, h * 0.54f)
            quadraticTo(w * 0.82f, h * 0.78f, w * 0.50f, h * 0.88f)
            quadraticTo(w * 0.18f, h * 0.78f, w * 0.18f, h * 0.54f)
            lineTo(w * 0.18f, h * 0.26f)
            close()
        }
        drawPath(path, color = color, style = Stroke(width = sw, cap = StrokeCap.Round, join = StrokeJoin.Round))
    }
}

@Composable
private fun IconRadio(color: Color, size: Dp = 26.dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.085f
        val c = Offset(w * 0.50f, h * 0.50f)
        drawCircle(color = color, radius = w * 0.11f, center = c)
        drawCircle(color = color, radius = w * 0.25f, center = c, style = Stroke(width = sw))
        drawCircle(color = color.copy(alpha = 0.75f), radius = w * 0.40f, center = c, style = Stroke(width = sw))
    }
}

@Composable
private fun IconSmartphone(color: Color, size: Dp = 20.dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.095f
        drawRoundRect(
            color = color,
            topLeft = Offset(w * 0.26f, h * 0.12f),
            size = Size(w * 0.48f, h * 0.76f),
            cornerRadius = CornerRadius(w * 0.08f, w * 0.08f),
            style = Stroke(width = sw),
        )
        drawCircle(color = color, radius = sw * 0.65f, center = Offset(w * 0.50f, h * 0.76f))
    }
}

@Composable
private fun IconLaptop(color: Color, size: Dp = 20.dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.095f
        drawRoundRect(
            color = color,
            topLeft = Offset(w * 0.18f, h * 0.22f),
            size = Size(w * 0.64f, h * 0.46f),
            cornerRadius = CornerRadius(w * 0.06f, w * 0.06f),
            style = Stroke(width = sw),
        )
        drawLine(color, Offset(w * 0.10f, h * 0.76f), Offset(w * 0.90f, h * 0.76f), strokeWidth = sw, cap = StrokeCap.Round)
    }
}

@Composable
private fun IconFileText(color: Color, size: Dp = 18.dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.1f
        val doc = Path().apply {
            moveTo(w * 0.24f, h * 0.12f)
            lineTo(w * 0.62f, h * 0.12f)
            lineTo(w * 0.78f, h * 0.28f)
            lineTo(w * 0.78f, h * 0.88f)
            lineTo(w * 0.24f, h * 0.88f)
            close()
        }
        drawPath(doc, color = color, style = Stroke(width = sw, cap = StrokeCap.Round, join = StrokeJoin.Round))
        drawLine(color, Offset(w * 0.36f, h * 0.50f), Offset(w * 0.66f, h * 0.50f), strokeWidth = sw, cap = StrokeCap.Round)
        drawLine(color, Offset(w * 0.36f, h * 0.68f), Offset(w * 0.66f, h * 0.68f), strokeWidth = sw, cap = StrokeCap.Round)
    }
}

@Composable
private fun IconFolder(color: Color, size: Dp = 18.dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.1f
        val path = Path().apply {
            moveTo(w * 0.14f, h * 0.24f)
            lineTo(w * 0.42f, h * 0.24f)
            lineTo(w * 0.50f, h * 0.34f)
            lineTo(w * 0.86f, h * 0.34f)
            lineTo(w * 0.86f, h * 0.80f)
            lineTo(w * 0.14f, h * 0.80f)
            close()
        }
        drawPath(path, color = color, style = Stroke(width = sw, cap = StrokeCap.Round, join = StrokeJoin.Round))
    }
}

@Composable
private fun IconClipboard(color: Color, size: Dp = 18.dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.1f
        drawRoundRect(
            color = color,
            topLeft = Offset(w * 0.22f, h * 0.22f),
            size = Size(w * 0.56f, h * 0.66f),
            cornerRadius = CornerRadius(w * 0.06f, w * 0.06f),
            style = Stroke(width = sw),
        )
        drawRoundRect(
            color = color,
            topLeft = Offset(w * 0.36f, h * 0.12f),
            size = Size(w * 0.28f, h * 0.16f),
            cornerRadius = CornerRadius(w * 0.04f, w * 0.04f),
            style = Stroke(width = sw),
        )
    }
}

@Composable
private fun IconTextNote(color: Color, size: Dp = 18.dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.1f
        drawLine(color, Offset(w * 0.22f, h * 0.26f), Offset(w * 0.78f, h * 0.26f), strokeWidth = sw, cap = StrokeCap.Round)
        drawLine(color, Offset(w * 0.50f, h * 0.26f), Offset(w * 0.50f, h * 0.80f), strokeWidth = sw, cap = StrokeCap.Round)
    }
}

@Composable
private fun IconClose(color: Color, size: Dp = 16.dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.12f
        drawLine(color, Offset(w * 0.24f, h * 0.24f), Offset(w * 0.76f, h * 0.76f), strokeWidth = sw, cap = StrokeCap.Round)
        drawLine(color, Offset(w * 0.76f, h * 0.24f), Offset(w * 0.24f, h * 0.76f), strokeWidth = sw, cap = StrokeCap.Round)
    }
}
