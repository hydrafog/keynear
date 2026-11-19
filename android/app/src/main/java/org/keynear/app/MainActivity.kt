package org.keynear.app

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.graphics.Color as AndroidColor
import android.os.Build
import android.os.Bundle
import android.provider.DocumentsContract
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
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
import org.keynear.app.ble.BleScanner
import org.keynear.app.ble.GattSession
import org.keynear.app.bridge.CoreBridge
import org.keynear.app.bridge.CoreClientProvider
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.CoreState
import org.keynear.app.ui.model.VisibilityState
import org.keynear.app.ui.model.evaluatePermissionsChecklist
import org.keynear.app.data.LanManager
import org.keynear.app.otp.OtpConsentVerifier
import org.keynear.app.otp.SmsUserConsentManager
import org.keynear.app.service.KeynearForegroundService
import org.keynear.designsystem.ActionButton
import org.keynear.designsystem.TactileSurface
import org.keynear.app.ui.devices.DeviceDetailScreen
import org.keynear.app.ui.navigation.buildKeynearActions
import org.keynear.app.ui.navigation.enumerateTreeChildUris
import org.keynear.app.ui.navigation.snapshotFromState
import org.keynear.app.ui.navigation.KeynearBottomNav
import org.keynear.app.ui.navigation.KeynearSnackbarHost
import org.keynear.app.ui.navigation.KeynearSubScreen
import org.keynear.app.ui.navigation.KeynearTab
import org.keynear.app.ui.navigation.KeynearTopAppBar
import org.keynear.app.ui.receive.ReceiveScreen
import org.keynear.app.ui.send.SendScreen
import org.keynear.app.ui.settings.PermissionsScreen
import org.keynear.app.ui.settings.SettingsScreen
import org.keynear.designsystem.KeynearTheme
import org.keynear.designsystem.LocalKeynearPalette

class MainActivity : ComponentActivity() {
    private var resumeTick by mutableIntStateOf(0)
    private var pendingConsentIntent by mutableStateOf<Intent?>(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge(
            statusBarStyle = SystemBarStyle.auto(AndroidColor.TRANSPARENT, AndroidColor.TRANSPARENT),
            navigationBarStyle = SystemBarStyle.auto(AndroidColor.TRANSPARENT, AndroidColor.TRANSPARENT),
        )
        extractPendingConsentIntent(intent)?.let { pendingConsentIntent = it }
        val bridge = CoreClientProvider.init(applicationContext)
        setContent {
            KeynearAppRoot(
                bridge = bridge,
                resumeTick = resumeTick,
                pendingConsentIntent = pendingConsentIntent,
                onConsentConsumed = { pendingConsentIntent = null },
            )
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        extractPendingConsentIntent(intent)?.let { pendingConsentIntent = it }
    }

    override fun onResume() {
        super.onResume()
        resumeTick += 1
    }

    private fun extractPendingConsentIntent(source: Intent?): Intent? {
        if (source?.action != SmsUserConsentManager.ACTION_HANDLE_SMS_CONSENT) return null
        @Suppress("DEPRECATION")
        val consent = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            source.getParcelableExtra(SmsUserConsentManager.EXTRA_PENDING_CONSENT_INTENT, Intent::class.java)
        } else {
            source.getParcelableExtra(SmsUserConsentManager.EXTRA_PENDING_CONSENT_INTENT)
        } ?: return null
        val pkg = consent.resolveActivity(packageManager)?.packageName
        return consent.takeIf { OtpConsentVerifier.isConsentIntentTrusted(pkg) }
    }
}

@Composable
private fun KeynearAppRoot(
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

    val consentLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.StartActivityForResult(),
    ) { result ->
        if (SmsUserConsentManager.handleConsentResult(context, result.resultCode, result.data)) {
            showSnackbar(context.getString(R.string.otp_verified_copied_snackbar))
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
            val prevVis = snapshotFromState(uiState)?.visibility
            val nextState = withContext(Dispatchers.IO) { bridge.tick() }
            val nextVis = snapshotFromState(nextState)?.visibility
            if (prevVis == VisibilityState.EVERYONE_NEARBY && nextVis != null && nextVis != prevVis) {
                withContext(Dispatchers.IO) {
                    BleScanner.startOrSync(context, nextVis)
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
                    missingPermissionsCount = permissionsChecklist.missingPermissionsCount,
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
                                localDeviceName = localName,
                                sheetExpanded = sheetExpanded,
                                onSetSheetExpanded = { sheetExpanded = it },
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
