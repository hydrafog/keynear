package org.keynear.app

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.DocumentsContract
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.keynear.app.ble.BleScanner
import org.keynear.app.bridge.CoreBridge
import org.keynear.app.bridge.readDismissedTransferIds
import org.keynear.app.bridge.writeDismissedTransferIds
import org.keynear.app.ui.model.CoreState
import org.keynear.app.ui.model.VisibilityState
import org.keynear.app.ui.model.evaluatePermissionsChecklist
import org.keynear.app.otp.SmsUserConsentManager
import org.keynear.app.service.KeynearForegroundService
import org.keynear.app.data.AndroidFolderExporter
import org.keynear.app.data.SafFolderHelper
import org.keynear.app.ui.navigation.buildKeynearActions
import org.keynear.app.ui.navigation.snapshotFromState
import org.keynear.app.ui.navigation.KeynearSubScreen
import org.keynear.app.ui.navigation.KeynearTab
import org.keynear.designsystem.KeynearAccents
import org.keynear.designsystem.KeynearTheme

@Composable
internal fun KeynearAppRoot(
    bridge: CoreBridge,
    resumeTick: Int = 0,
    pendingConsentIntent: Intent? = null,
    onConsentConsumed: () -> Unit = {},
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var uiState by remember { mutableStateOf<CoreState>(CoreState.Loading) }
    var permissionsChecklist by remember { mutableStateOf(evaluatePermissionsChecklist(context)) }
    var confirmUnpairDeviceId by remember { mutableStateOf<String?>(null) }
    var activeTab by rememberSaveable { mutableStateOf(KeynearTab.RECEIVE) }
    var subScreen by remember { mutableStateOf<KeynearSubScreen?>(null) }
    var sheetExpanded by rememberSaveable { mutableStateOf(false) }
    var dismissedTransferIds by remember { mutableStateOf(readDismissedTransferIds(context)) }

    val snapshot = snapshotFromState(uiState)
    val darkTheme = snapshot?.daemonConfig?.darkTheme ?: isSystemInDarkTheme()
    val accentHex = snapshot?.daemonConfig?.accentColorHex ?: KeynearAccents.DEFAULT_HEX

    val showSnackbar: (String) -> Unit = {}
    val picker = PickerContext(context, bridge, scope, { state -> uiState = state }, showSnackbar)

    val syncBleAndLoad: suspend () -> Unit = {
        val loaded = withContext(Dispatchers.IO) { bridge.loadState() }
        val vis = snapshotFromState(loaded)?.visibility ?: VisibilityState.PAIRED_ONLY
        withContext(Dispatchers.IO) {
            BleScanner.startOrSync(context, vis)
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
        picker.stagePickedFiles(uris)
    }

    val pickFolderLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocumentTree(),
    ) { treeUri -> picker.stagePickedFolder(treeUri) }

    val pickSaveFolderLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocumentTree(),
    ) { treeUri -> picker.setSaveFolder(treeUri) }

    val otpVerifiedMessage = stringResource(R.string.otp_verified_copied_snackbar)
    val consentLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.StartActivityForResult(),
    ) { result ->
        if (SmsUserConsentManager.handleConsentResult(context, result.resultCode, result.data)) {
            showSnackbar(otpVerifiedMessage)
        }
    }

    LaunchedEffect(pendingConsentIntent) {
        val intent = pendingConsentIntent ?: return@LaunchedEffect
        onConsentConsumed()
        consentLauncher.launch(intent)
    }

    LaunchedEffect(bridge, resumeTick) {
        syncBleAndLoad()
    }

    DisposableEffect(bridge) {
        SmsUserConsentManager.setForegroundConsentLauncher { intent ->
            consentLauncher.launch(intent)
        }
        BleScanner.setUiRefreshListener {
            scope.launch {
                uiState = withContext(Dispatchers.IO) { bridge.loadState() }
            }
        }
        onDispose {
            SmsUserConsentManager.setForegroundConsentLauncher(null)
            BleScanner.setUiRefreshListener(null)
        }
    }

    LaunchedEffect(bridge) {
        while (true) {
            delay(1000L)
            uiState = tickAndSync(bridge, context, snapshotFromState(uiState)?.visibility)
        }
    }

    BackHandler(enabled = subScreen != null) {
        subScreen = null
        confirmUnpairDeviceId = null
    }

    val actions = remember(context, bridge) {
        buildKeynearActions(
            context = context,
            scope = scope,
            bridge = bridge,
            onStateUpdated = { uiState = it },
            onShowSnackbar = showSnackbar,
            onLaunchPermissions = { permissionLauncher.launch(it) },
            onPickFiles = { pickFilesLauncher.launch(arrayOf("*/*")) },
            onPickFolder = { pickFolderLauncher.launch(null) },
            onPickSaveFolder = { pickSaveFolderLauncher.launch(null) },
            onRequestUnpair = { confirmUnpairDeviceId = it },
            onCancelUnpair = { confirmUnpairDeviceId = null },
            onOpenPermissions = { subScreen = KeynearSubScreen.Permissions },
            onOpenDeviceDetail = {
                confirmUnpairDeviceId = null
                subScreen = KeynearSubScreen.DeviceDetail(it)
            },
            onNavigateTab = { activeTab = it },
            onCloseSubScreen = { subScreen = null },
        )
    }

    fun clearSubScreen() {
        subScreen = null
        confirmUnpairDeviceId = null
    }

    KeynearTheme(darkTheme = darkTheme, accentHex = accentHex) {
        KeynearAppContent(
            snapshot = snapshot,
            uiState = uiState,
            activeTab = activeTab,
            subScreen = subScreen,
            confirmUnpairDeviceId = confirmUnpairDeviceId,
            permissionsChecklist = permissionsChecklist,
            actions = actions,
            sheetExpanded = sheetExpanded,
            onSetSheetExpanded = { sheetExpanded = it },
            dismissedTransferIds = dismissedTransferIds,
            onDismissTransfers = { ids ->
                val updated = dismissedTransferIds + ids
                dismissedTransferIds = updated
                writeDismissedTransferIds(context, updated)
            },
            onBack = ::clearSubScreen,
            onSelectTab = { tab ->
                clearSubScreen()
                activeTab = tab
            },
            scope = scope,
            syncBleAndLoad = syncBleAndLoad,
        )
    }
}

private data class PickerContext(
    val context: Context,
    val bridge: CoreBridge,
    val scope: CoroutineScope,
    val onUiState: (CoreState) -> Unit,
    val onSnackbar: (String) -> Unit,
)

private fun PickerContext.stagePickedFiles(uris: List<Uri>) {
    if (uris.isEmpty()) return
    uris.forEach { uri ->
        runCatching {
            context.contentResolver.takePersistableUriPermission(
                uri,
                Intent.FLAG_GRANT_READ_URI_PERMISSION,
            )
        }
    }
    scope.launch {
        onUiState(withContext(Dispatchers.IO) { bridge.stageFileUris(uris.map { it.toString() }) })
        onSnackbar(filesStagedMessage(uris.size))
    }
}

private fun PickerContext.stagePickedFolder(treeUri: Uri?) {
    if (treeUri == null) return
    runCatching {
        context.contentResolver.takePersistableUriPermission(
            treeUri,
            Intent.FLAG_GRANT_READ_URI_PERMISSION,
        )
    }
    scope.launch {
        val summary = withContext(Dispatchers.IO) {
            SafFolderHelper.inspectFolderTree(context, treeUri)
        }
        onUiState(withContext(Dispatchers.IO) {
            bridge.stageFolderUri(treeUri.toString(), summary.displayName, summary.totalBytes)
        })
        onSnackbar(folderStagedMessage(summary.displayName, summary.fileCount))
    }
}

private fun PickerContext.setSaveFolder(treeUri: Uri?) {
    if (treeUri == null) return
    runCatching {
        context.contentResolver.takePersistableUriPermission(
            treeUri,
            Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
        )
    }
    AndroidFolderExporter.saveFolderTreeUriPref(context, treeUri)
    val displayPath = displayPathForTree(treeUri)
    scope.launch {
        onUiState(withContext(Dispatchers.IO) {
            bridge.updateDaemonConfig { it.copy(incomingDirectory = displayPath) }
        })
        onSnackbar("Save folder updated to $displayPath")
    }
}

private fun filesStagedMessage(count: Int): String =
    "Added $count file${if (count == 1) "" else "s"} to queue"

private fun folderStagedMessage(folderName: String, fileCount: Int): String =
    if (fileCount == 0) "Staged folder $folderName" else "Added folder $folderName ($fileCount files)"

private fun displayPathForTree(treeUri: Uri): String {
    val treeId = runCatching { DocumentsContract.getTreeDocumentId(treeUri) }.getOrNull()
    return treeId?.substringAfter(':')?.takeIf { it.isNotBlank() }
        ?.let { "~/$it" } ?: treeUri.toString()
}

private suspend fun tickAndSync(
    bridge: CoreBridge,
    context: Context,
    currentVisibility: VisibilityState?,
): CoreState {
    val nextState = withContext(Dispatchers.IO) { bridge.tick() }
    val nextVis = snapshotFromState(nextState)?.visibility
    if (currentVisibility == VisibilityState.EVERYONE_NEARBY && nextVis != null && nextVis != currentVisibility) {
        withContext(Dispatchers.IO) {
            BleScanner.startOrSync(context, nextVis)
            KeynearForegroundService.syncIfPermitted(context)
        }
    }
    return nextState
}
