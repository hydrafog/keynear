package org.keynear.app

import android.Manifest
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
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
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat
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
import org.keynear.app.core.DiscoveredPeerUi
import org.keynear.app.core.LocalNetworkStatus
import org.keynear.app.core.PairedDeviceUi
import org.keynear.app.core.PendingPairingUi
import org.keynear.app.core.PermissionsChecklistState
import org.keynear.app.core.TransferDirectionUi
import org.keynear.app.core.TransferStateUi
import org.keynear.app.core.TransferUi
import org.keynear.app.core.VisibilityState
import org.keynear.app.core.evaluatePermissionsChecklist
import org.keynear.app.core.formatByteCount
import org.keynear.app.core.validateDisplayNameInput
import org.keynear.app.lan.LocalNetworkConstants
import org.keynear.app.lan.LocalNetworkManager
import org.keynear.app.service.KeynearForegroundService
import org.keynear.app.ui.theme.KeynearMotion
import org.keynear.app.ui.theme.KeynearRadii
import org.keynear.app.ui.theme.KeynearSpacing
import org.keynear.app.ui.theme.KeynearTheme
import org.keynear.app.ui.theme.LocalKeynearPalette

private enum class ScreenCategory {
    LOADING,
    SNAPSHOT,
    ERROR,
}

private data class SnapshotActions(
    val onCycleVisibility: () -> Unit,
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
    val onAcceptTransfer: (String) -> Unit,
    val onDeclineTransfer: (String) -> Unit,
    val onCancelTransfer: (String) -> Unit,
    val onRequestUnpair: (String) -> Unit,
    val onCancelUnpair: () -> Unit,
    val onConfirmUnpair: (String) -> Unit,
)

private fun launchFirstResolvableActivity(
    context: Context,
    candidates: List<Intent>,
): Boolean {
    val pm = context.packageManager
    for (intent in candidates) {
        if (intent.resolveActivity(pm) != null) {
            if (runCatching { context.startActivity(intent) }.isSuccess) {
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
            KeynearTheme {
                KeynearFoundationScreen(bridge = bridge, resumeTick = resumeTick)
            }
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

@Composable
private fun KeynearFoundationScreen(
    bridge: CoreBridge,
    resumeTick: Int = 0,
) {
    val palette = LocalKeynearPalette.current
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val reducedMotion = remember(context) {
        runCatching {
            Settings.Global.getFloat(
                context.contentResolver,
                Settings.Global.ANIMATOR_DURATION_SCALE,
                1f,
            ) == 0f
        }.getOrDefault(false)
    }

    var uiState by remember { mutableStateOf<CoreState>(CoreState.Loading) }
    var confirmUnpairDeviceId by remember { mutableStateOf<String?>(null) }
    var permissionsChecklist by remember {
        mutableStateOf(evaluatePermissionsChecklist(context))
    }

    val syncBleAndLoad: suspend () -> Unit = {
        val loaded = withContext(Dispatchers.IO) { bridge.loadState() }
        val vis = snapshotFromState(loaded)?.visibility ?: VisibilityState.PAIRED_ONLY
        withContext(Dispatchers.IO) {
            KeynearBleManager.startOrSync(context, vis)
            KeynearForegroundService.syncIfPermitted(context)
        }
        permissionsChecklist = evaluatePermissionsChecklist(context)
        uiState = withContext(Dispatchers.IO) { bridge.loadState() }
    }

    val permissionLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.RequestMultiplePermissions(),
    ) {
        scope.launch { syncBleAndLoad() }
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

    val actions = SnapshotActions(
        onCycleVisibility = {
            scope.launch {
                val cycled = withContext(Dispatchers.IO) { bridge.cycleVisibility() }
                val vis = snapshotFromState(cycled)?.visibility ?: VisibilityState.PAIRED_ONLY
                withContext(Dispatchers.IO) {
                    KeynearBleManager.startOrSync(context, vis)
                    KeynearForegroundService.syncIfPermitted(context)
                }
                uiState = withContext(Dispatchers.IO) { bridge.loadState() }
            }
        },
        onRequestBluetoothPermissions = {
            val missing = KeynearBleConstants.missingPermissions(
                context,
                Build.VERSION.SDK_INT,
            )
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
            val missing = KeynearBleConstants.missingPermissions(
                context,
                Build.VERSION.SDK_INT,
            ).toMutableList()
            if (Build.VERSION.SDK_INT >= 33 &&
                ContextCompat.checkSelfPermission(context, Manifest.permission.NEARBY_WIFI_DEVICES) !=
                PackageManager.PERMISSION_GRANTED
            ) {
                missing.add(Manifest.permission.NEARBY_WIFI_DEVICES)
            }
            if (LocalNetworkConstants.requiresLocalNetworkPermission(Build.VERSION.SDK_INT)) {
                missing.add(LocalNetworkConstants.PERMISSION_ACCESS_LOCAL_NETWORK)
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
            if (Build.VERSION.SDK_INT >= 33 &&
                ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) !=
                PackageManager.PERMISSION_GRANTED
            ) {
                permissionLauncher.launch(arrayOf(Manifest.permission.POST_NOTIFICATIONS))
            } else {
                launchFirstResolvableActivity(
                    context,
                    listOf(
                        Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS).putExtra(
                            Settings.EXTRA_APP_PACKAGE,
                            context.packageName,
                        ),
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
            permissionsChecklist = evaluatePermissionsChecklist(context)
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
                withContext(Dispatchers.IO) {
                    KeynearBleManager.initiatePairing(context, addr)
                }
                uiState = withContext(Dispatchers.IO) { bridge.loadState() }
            }
        },
        onApprovePairing = {
            scope.launch {
                withContext(Dispatchers.IO) {
                    KeynearBleManager.approveActivePairing(context)
                }
                uiState = withContext(Dispatchers.IO) { bridge.loadState() }
            }
        },
        onRejectPairing = {
            scope.launch {
                withContext(Dispatchers.IO) {
                    KeynearBleManager.rejectActivePairing(context)
                }
                uiState = withContext(Dispatchers.IO) { bridge.loadState() }
            }
        },
        onToggleAutoAccept = { deviceId, enabled ->
            scope.launch {
                uiState = withContext(Dispatchers.IO) {
                    bridge.setAutoAcceptFiles(deviceId, enabled)
                }
            }
        },
        onRenameDevice = { deviceId, newDisplayName ->
            scope.launch {
                uiState = withContext(Dispatchers.IO) {
                    bridge.renamePairedDevice(deviceId, newDisplayName)
                }
            }
        },
        onAcceptTransfer = { transferId ->
            scope.launch {
                uiState = withContext(Dispatchers.IO) {
                    bridge.acceptTransfer(transferId)
                }
            }
        },
        onDeclineTransfer = { transferId ->
            scope.launch {
                uiState = withContext(Dispatchers.IO) {
                    bridge.declineTransfer(transferId)
                }
            }
        },
        onCancelTransfer = { transferId ->
            scope.launch {
                uiState = withContext(Dispatchers.IO) {
                    bridge.cancelTransfer(transferId)
                }
            }
        },
        onRequestUnpair = { deviceId -> confirmUnpairDeviceId = deviceId },
        onCancelUnpair = { confirmUnpairDeviceId = null },
        onConfirmUnpair = { deviceId ->
            confirmUnpairDeviceId = null
            scope.launch {
                uiState = withContext(Dispatchers.IO) {
                    bridge.unpairDevice(deviceId)
                }
            }
        },
    )

    val category = when (uiState) {
        CoreState.Loading -> ScreenCategory.LOADING
        is CoreState.Error -> ScreenCategory.ERROR
        is CoreState.Empty, is CoreState.Ready -> ScreenCategory.SNAPSHOT
    }

    Surface(
        modifier = Modifier.fillMaxSize(),
        color = palette.canvas,
    ) {
        Crossfade(
            targetState = category,
            animationSpec = tween(durationMillis = if (reducedMotion) 0 else KeynearMotion.SHORT_DURATION_MS),
            label = "core_state_crossfade",
        ) { targetCategory ->
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(KeynearSpacing.Lg),
                verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
            ) {
                Text(
                    text = stringResource(R.string.app_name),
                    style = MaterialTheme.typography.headlineSmall,
                    color = palette.textPrimary,
                )
                Text(
                    text = "Local continuity for nearby devices",
                    style = MaterialTheme.typography.bodyMedium,
                    color = palette.textSecondary,
                )

                when (targetCategory) {
                    ScreenCategory.LOADING -> LoadingCard()
                    ScreenCategory.ERROR -> {
                        val err = uiState as? CoreState.Error
                        if (err != null) {
                            ErrorBanner(
                                message = err.message,
                                onRetry = { scope.launch { syncBleAndLoad() } },
                            )
                            err.previous?.let { previous ->
                                SnapshotContent(
                                    snapshot = previous,
                                    checklist = permissionsChecklist,
                                    confirmUnpairDeviceId = confirmUnpairDeviceId,
                                    actions = actions,
                                )
                            }
                        }
                    }
                    ScreenCategory.SNAPSHOT -> {
                        val snapshot = snapshotFromState(uiState)
                        if (snapshot != null) {
                            SnapshotContent(
                                snapshot = snapshot,
                                checklist = permissionsChecklist,
                                confirmUnpairDeviceId = confirmUnpairDeviceId,
                                actions = actions,
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun LoadingCard() {
    val palette = LocalKeynearPalette.current
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.TwoXl),
        colors = CardDefaults.cardColors(containerColor = palette.surface),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Lg),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            CircularProgressIndicator(color = palette.accent)
            Spacer(modifier = Modifier.width(KeynearSpacing.Md))
            Text(
                text = stringResource(R.string.status_loading),
                style = MaterialTheme.typography.bodyLarge,
                color = palette.textPrimary,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f),
            )
        }
    }
}

@Composable
private fun ErrorBanner(
    message: String,
    onRetry: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.Xl),
        colors = CardDefaults.cardColors(containerColor = palette.errorTonal),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Lg),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
        ) {
            Text(
                text = message,
                style = MaterialTheme.typography.bodyMedium,
                color = palette.onDangerTonal,
                maxLines = 3,
                overflow = TextOverflow.Ellipsis,
            )
            Button(
                onClick = onRetry,
                modifier = Modifier.defaultMinSize(
                    minWidth = KeynearSpacing.MinTouchTarget,
                    minHeight = KeynearSpacing.MinTouchTarget,
                ),
                shape = RoundedCornerShape(KeynearRadii.Xl),
                colors = ButtonDefaults.buttonColors(
                    containerColor = palette.accent,
                    contentColor = palette.onAccent,
                ),
            ) {
                Text(text = stringResource(R.string.action_retry))
            }
        }
    }
}

@Composable
private fun BluetoothWarningCard(
    status: BluetoothStatus,
    message: String?,
    onRequestPermissions: () -> Unit,
    onOpenBluetoothSettings: () -> Unit,
) {
    val dark = isSystemInDarkTheme()
    val warningBg = if (dark) Color(0xFF381C00) else Color(0xFFFFF6D3)
    val warningFg = if (dark) Color(0xFFFFEAA5) else Color(0xFF863300)
    val palette = LocalKeynearPalette.current

    val resolvedText = message ?: when (status) {
        BluetoothStatus.MISSING_PERMISSION -> "Bluetooth permissions are required for discovery and pairing."
        BluetoothStatus.ADAPTER_DISABLED -> "Bluetooth is turned off. Enable Bluetooth to discover nearby peers."
        BluetoothStatus.UNAVAILABLE -> "Bluetooth Low Energy hardware is unavailable on this device."
        BluetoothStatus.READY -> ""
    }

    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.Xl),
        colors = CardDefaults.cardColors(containerColor = warningBg),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Lg),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
        ) {
            Text(
                text = resolvedText,
                style = MaterialTheme.typography.bodyMedium,
                color = warningFg,
                maxLines = 3,
                overflow = TextOverflow.Ellipsis,
            )
            if (status == BluetoothStatus.MISSING_PERMISSION) {
                Button(
                    onClick = onRequestPermissions,
                    modifier = Modifier.defaultMinSize(
                        minWidth = KeynearSpacing.MinTouchTarget,
                        minHeight = KeynearSpacing.MinTouchTarget,
                    ),
                    shape = RoundedCornerShape(KeynearRadii.Xl),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = palette.accent,
                        contentColor = palette.onAccent,
                    ),
                ) {
                    Text(text = stringResource(R.string.action_allow_bluetooth))
                }
            } else if (status == BluetoothStatus.ADAPTER_DISABLED) {
                Button(
                    onClick = onOpenBluetoothSettings,
                    modifier = Modifier.defaultMinSize(
                        minWidth = KeynearSpacing.MinTouchTarget,
                        minHeight = KeynearSpacing.MinTouchTarget,
                    ),
                    shape = RoundedCornerShape(KeynearRadii.Xl),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = palette.accent,
                        contentColor = palette.onAccent,
                    ),
                ) {
                    Text(text = stringResource(R.string.action_enable_bluetooth))
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
    onOpenSettings: () -> Unit,
) {
    val dark = isSystemInDarkTheme()
    val warningBg = if (dark) Color(0xFF381C00) else Color(0xFFFFF6D3)
    val warningFg = if (dark) Color(0xFFFFEAA5) else Color(0xFF863300)
    val palette = LocalKeynearPalette.current

    val resolvedText = message ?: when (status) {
        LocalNetworkStatus.MISSING_PERMISSION -> stringResource(R.string.lnp_missing_permission_body)
        LocalNetworkStatus.BLOCKED_LNP -> stringResource(R.string.lnp_blocked_body)
        LocalNetworkStatus.UNAVAILABLE -> "Local Wi-Fi network discovery is unavailable."
        LocalNetworkStatus.READY -> ""
    }

    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.Xl),
        colors = CardDefaults.cardColors(containerColor = warningBg),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Lg),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
        ) {
            Text(
                text = stringResource(R.string.lnp_blocked_title),
                style = MaterialTheme.typography.titleMedium,
                color = warningFg,
            )
            Text(
                text = resolvedText,
                style = MaterialTheme.typography.bodyMedium,
                color = warningFg,
                maxLines = 3,
                overflow = TextOverflow.Ellipsis,
            )
            Row(horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm)) {
                if (status == LocalNetworkStatus.MISSING_PERMISSION) {
                    Button(
                        onClick = onRequestPermission,
                        modifier = Modifier.defaultMinSize(
                            minWidth = KeynearSpacing.MinTouchTarget,
                            minHeight = KeynearSpacing.MinTouchTarget,
                        ),
                        shape = RoundedCornerShape(KeynearRadii.Xl),
                        colors = ButtonDefaults.buttonColors(
                            containerColor = palette.accent,
                            contentColor = palette.onAccent,
                        ),
                    ) {
                        Text(text = stringResource(R.string.action_allow_local_network))
                    }
                } else if (status == LocalNetworkStatus.BLOCKED_LNP) {
                    Button(
                        onClick = onOpenSettings,
                        modifier = Modifier.defaultMinSize(
                            minWidth = KeynearSpacing.MinTouchTarget,
                            minHeight = KeynearSpacing.MinTouchTarget,
                        ),
                        shape = RoundedCornerShape(KeynearRadii.Xl),
                        colors = ButtonDefaults.buttonColors(
                            containerColor = palette.accent,
                            contentColor = palette.onAccent,
                        ),
                    ) {
                        Text(text = stringResource(R.string.action_open_settings))
                    }
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
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.TwoXl),
        colors = CardDefaults.cardColors(containerColor = palette.surfaceTonal),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Lg),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
        ) {
            Text(
                text = stringResource(R.string.pairing_prompt_title),
                style = MaterialTheme.typography.titleMedium,
                color = palette.textPrimary,
            )
            Text(
                text = "Peer: ${pending.peerName} (${pending.remainingSecs}s remaining)",
                style = MaterialTheme.typography.bodyMedium,
                color = palette.textSecondary,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                text = PairingNotificationManager.formatCode(pending.verificationCode),
                style = TextStyle(
                    fontFamily = FontFamily.Monospace,
                    fontWeight = FontWeight.Bold,
                    fontSize = 28.sp,
                    letterSpacing = 2.sp,
                    fontFeatureSettings = "tnum",
                ),
                color = palette.textPrimary,
            )
            Row(
                horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
            ) {
                Button(
                    onClick = onApprove,
                    modifier = Modifier.defaultMinSize(
                        minWidth = KeynearSpacing.MinTouchTarget,
                        minHeight = KeynearSpacing.MinTouchTarget,
                    ),
                    shape = RoundedCornerShape(KeynearRadii.Xl),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = palette.accent,
                        contentColor = palette.onAccent,
                    ),
                ) {
                    Text(text = stringResource(R.string.action_confirm))
                }
                OutlinedButton(
                    onClick = onReject,
                    modifier = Modifier.defaultMinSize(
                        minWidth = KeynearSpacing.MinTouchTarget,
                        minHeight = KeynearSpacing.MinTouchTarget,
                    ),
                    shape = RoundedCornerShape(KeynearRadii.Xl),
                ) {
                    Text(
                        text = stringResource(R.string.action_cancel),
                        color = palette.textPrimary,
                    )
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
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.TwoXl),
        colors = CardDefaults.cardColors(containerColor = palette.surface),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Lg),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
        ) {
            Text(
                text = stringResource(R.string.active_transfers_title),
                style = MaterialTheme.typography.titleMedium,
                color = palette.textPrimary,
            )
            if (transfers.isEmpty()) {
                Text(
                    text = stringResource(R.string.no_active_transfers),
                    style = MaterialTheme.typography.bodyMedium,
                    color = palette.textSecondary,
                )
            } else {
                transfers.forEach { transfer ->
                    TransferRow(
                        transfer = transfer,
                        onAccept = { onAccept(transfer.transferId) },
                        onDecline = { onDecline(transfer.transferId) },
                        onCancel = { onCancel(transfer.transferId) },
                    )
                }
            }
        }
    }
}

@Composable
private fun TransferRow(
    transfer: TransferUi,
    onAccept: () -> Unit,
    onDecline: () -> Unit,
    onCancel: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val dirLabel = if (transfer.direction == TransferDirectionUi.RECEIVE) "From" else "To"
    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.Xl),
        color = palette.surfaceTonal,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Md),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Xs),
        ) {
            Text(
                text = transfer.summaryName,
                style = MaterialTheme.typography.bodyLarge,
                color = palette.textPrimary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                text = "$dirLabel ${transfer.peerDisplayName} · ${transfer.state.label()}",
                style = MaterialTheme.typography.bodyMedium,
                color = palette.textSecondary,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                text = "${formatByteCount(transfer.transferredBytes)} / ${formatByteCount(transfer.totalBytes)}",
                style = MaterialTheme.typography.labelMedium.copy(fontFeatureSettings = "tnum"),
                color = palette.textMuted,
            )
            if (transfer.state == TransferStateUi.ACTIVE) {
                if (transfer.totalBytes > 0L) {
                    val fraction = (transfer.transferredBytes.toFloat() / transfer.totalBytes.toFloat()).coerceIn(0f, 1f)
                    LinearProgressIndicator(
                        progress = { fraction },
                        modifier = Modifier.fillMaxWidth(),
                        color = palette.accent,
                        trackColor = palette.surface,
                    )
                } else {
                    LinearProgressIndicator(
                        modifier = Modifier.fillMaxWidth(),
                        color = palette.accent,
                        trackColor = palette.surface,
                    )
                }
            }
            if (transfer.state == TransferStateUi.PENDING_CONSENT &&
                transfer.direction == TransferDirectionUi.RECEIVE
            ) {
                Row(horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm)) {
                    Button(
                        onClick = onAccept,
                        modifier = Modifier.defaultMinSize(
                            minWidth = KeynearSpacing.MinTouchTarget,
                            minHeight = KeynearSpacing.MinTouchTarget,
                        ),
                        shape = RoundedCornerShape(KeynearRadii.Xl),
                        colors = ButtonDefaults.buttonColors(
                            containerColor = palette.accent,
                            contentColor = palette.onAccent,
                        ),
                    ) {
                        Text(text = stringResource(R.string.action_accept))
                    }
                    OutlinedButton(
                        onClick = onDecline,
                        modifier = Modifier.defaultMinSize(
                            minWidth = KeynearSpacing.MinTouchTarget,
                            minHeight = KeynearSpacing.MinTouchTarget,
                        ),
                        shape = RoundedCornerShape(KeynearRadii.Xl),
                    ) {
                        Text(
                            text = stringResource(R.string.action_decline),
                            color = palette.textPrimary,
                        )
                    }
                }
            } else if (transfer.state == TransferStateUi.ACTIVE) {
                OutlinedButton(
                    onClick = onCancel,
                    modifier = Modifier.defaultMinSize(
                        minWidth = KeynearSpacing.MinTouchTarget,
                        minHeight = KeynearSpacing.MinTouchTarget,
                    ),
                    shape = RoundedCornerShape(KeynearRadii.Xl),
                ) {
                    Text(
                        text = stringResource(R.string.action_cancel),
                        color = palette.textPrimary,
                    )
                }
            }
        }
    }
}

@Composable
private fun SnapshotContent(
    snapshot: CoreSnapshot,
    checklist: PermissionsChecklistState,
    confirmUnpairDeviceId: String?,
    actions: SnapshotActions,
) {
    val palette = LocalKeynearPalette.current
    val dark = isSystemInDarkTheme()
    val warningBg = if (dark) Color(0xFF381C00) else Color(0xFFFFF6D3)
    val warningFg = if (dark) Color(0xFFFFEAA5) else Color(0xFF863300)

    PermissionsOnboardingCard(
        checklist = checklist,
        onGrantNearby = actions.onRequestNearbyOnboardingPermissions,
        onGrantNotifications = actions.onRequestNotificationsPermission,
        onAllowBackground = actions.onRequestBatteryOptimization,
    )

    if (snapshot.bluetoothStatus != BluetoothStatus.READY || snapshot.warningMessage != null) {
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
            onOpenSettings = actions.onOpenAppSettings,
        )
    }

    snapshot.pendingPairing?.let { pending ->
        PendingPairingCard(
            pending = pending,
            onApprove = actions.onApprovePairing,
            onReject = actions.onRejectPairing,
        )
    }

    ActiveTransfersCard(
        transfers = snapshot.activeTransfers,
        onAccept = actions.onAcceptTransfer,
        onDecline = actions.onDeclineTransfer,
        onCancel = actions.onCancelTransfer,
    )

    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.TwoXl),
        colors = CardDefaults.cardColors(containerColor = palette.surface),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Lg),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
        ) {
            Text(
                text = "Discovery visibility",
                style = MaterialTheme.typography.titleMedium,
                color = palette.textPrimary,
            )
            Text(
                text = snapshot.visibility.formattedStatus(snapshot.everyoneRemainingSecs),
                style = MaterialTheme.typography.bodyLarge,
                color = palette.textSecondary,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            Spacer(modifier = Modifier.height(KeynearSpacing.Xs))
            Button(
                onClick = actions.onCycleVisibility,
                modifier = Modifier.defaultMinSize(
                    minWidth = KeynearSpacing.MinTouchTarget,
                    minHeight = KeynearSpacing.MinTouchTarget,
                ),
                shape = RoundedCornerShape(KeynearRadii.Xl),
                colors = ButtonDefaults.buttonColors(
                    containerColor = palette.accent,
                    contentColor = palette.onAccent,
                ),
            ) {
                Text(text = stringResource(R.string.action_cycle_visibility))
            }
        }
    }

    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.TwoXl),
        colors = CardDefaults.cardColors(containerColor = palette.surface),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Lg),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
        ) {
            Text(
                text = stringResource(R.string.nearby_peers_title),
                style = MaterialTheme.typography.titleMedium,
                color = palette.textPrimary,
            )
            if (snapshot.nearbyPeers.isEmpty()) {
                Text(
                    text = stringResource(R.string.no_nearby_peers),
                    style = MaterialTheme.typography.bodyMedium,
                    color = palette.textSecondary,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
            } else {
                snapshot.nearbyPeers.forEach { peer ->
                    NearbyPeerRow(peer = peer, onPair = { actions.onInitiatePairing(peer.address) })
                }
            }
        }
    }

    if (snapshot.pairedDevices.isEmpty()) {
        Card(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(KeynearRadii.TwoXl),
            colors = CardDefaults.cardColors(containerColor = palette.surfaceTonal),
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(KeynearSpacing.Lg),
                verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Xs),
            ) {
                Text(
                    text = stringResource(R.string.empty_devices_title),
                    style = MaterialTheme.typography.titleMedium,
                    color = palette.textPrimary,
                )
                Text(
                    text = stringResource(R.string.empty_devices_body),
                    style = MaterialTheme.typography.bodyMedium,
                    color = palette.textSecondary,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    } else {
        Card(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(KeynearRadii.TwoXl),
            colors = CardDefaults.cardColors(containerColor = palette.surface),
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(KeynearSpacing.Lg),
                verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
            ) {
                Text(
                    text = stringResource(R.string.paired_devices_title),
                    style = MaterialTheme.typography.titleMedium,
                    color = palette.textPrimary,
                )
                snapshot.pairedDevices.forEach { device ->
                    PairedDeviceRow(
                        device = device,
                        confirmingUnpair = confirmUnpairDeviceId == device.deviceId,
                        warningBg = warningBg,
                        warningFg = warningFg,
                        onToggleAutoAccept = { enabled ->
                            actions.onToggleAutoAccept(device.deviceId, enabled)
                        },
                        onRenameDevice = { newName ->
                            actions.onRenameDevice(device.deviceId, newName)
                        },
                        onRequestUnpair = { actions.onRequestUnpair(device.deviceId) },
                        onCancelUnpair = actions.onCancelUnpair,
                        onConfirmUnpair = { actions.onConfirmUnpair(device.deviceId) },
                    )
                }
            }
        }
    }

    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.TwoXl),
        colors = CardDefaults.cardColors(containerColor = palette.surface),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Lg),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Xs),
        ) {
            Text(
                text = "Core contract",
                style = MaterialTheme.typography.titleMedium,
                color = palette.textPrimary,
            )
            Text(
                text = "Version ${snapshot.version} (${snapshot.maxControlFrameBytes} bytes max frame)",
                style = MaterialTheme.typography.bodyMedium,
                color = palette.textSecondary,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                text = snapshot.configSummary,
                style = MaterialTheme.typography.labelMedium,
                color = palette.textMuted,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

@Composable
private fun NearbyPeerRow(
    peer: DiscoveredPeerUi,
    onPair: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = peer.displayName,
                style = MaterialTheme.typography.bodyLarge,
                color = palette.textPrimary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            val batterySuffix = peer.batteryLevel?.let { " · $it%" } ?: ""
            Text(
                text = "${peer.role.label()} · ${peer.rssi} dBm$batterySuffix",
                style = MaterialTheme.typography.labelMedium,
                color = palette.textSecondary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
        Spacer(modifier = Modifier.width(KeynearSpacing.Sm))
        if (peer.isPaired) {
            Text(
                text = "Paired",
                style = MaterialTheme.typography.labelMedium,
                color = palette.accent,
            )
        } else {
            Button(
                onClick = onPair,
                modifier = Modifier.defaultMinSize(
                    minWidth = KeynearSpacing.MinTouchTarget,
                    minHeight = KeynearSpacing.MinTouchTarget,
                ),
                shape = RoundedCornerShape(KeynearRadii.Xl),
                colors = ButtonDefaults.buttonColors(
                    containerColor = palette.accent,
                    contentColor = palette.onAccent,
                ),
            ) {
                Text(text = stringResource(R.string.action_pair))
            }
        }
    }
}

@Composable
private fun PermissionsOnboardingCard(
    checklist: PermissionsChecklistState,
    onGrantNearby: () -> Unit,
    onGrantNotifications: () -> Unit,
    onAllowBackground: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    var manualExpanded by remember { mutableStateOf<Boolean?>(null) }
    val expanded = manualExpanded ?: !checklist.allRequiredGranted

    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.TwoXl),
        colors = CardDefaults.cardColors(containerColor = palette.surfaceTonal),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Lg),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = stringResource(R.string.onboarding_title),
                        style = MaterialTheme.typography.titleMedium,
                        color = palette.textPrimary,
                    )
                    Text(
                        text = if (checklist.allRecommendedComplete) {
                            stringResource(R.string.onboarding_summary_ready)
                        } else {
                            stringResource(R.string.onboarding_summary_pending)
                        },
                        style = MaterialTheme.typography.bodyMedium,
                        color = palette.textSecondary,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                Spacer(modifier = Modifier.width(KeynearSpacing.Sm))
                Column(horizontalAlignment = Alignment.End) {
                    Text(
                        text = "${checklist.completedCount}/4",
                        style = MaterialTheme.typography.labelMedium.copy(
                            fontFeatureSettings = "tnum",
                        ),
                        color = palette.accent,
                    )
                    TextButton(
                        onClick = { manualExpanded = !expanded },
                        modifier = Modifier.defaultMinSize(
                            minWidth = KeynearSpacing.MinTouchTarget,
                            minHeight = KeynearSpacing.MinTouchTarget,
                        ),
                    ) {
                        Text(
                            text = if (expanded) {
                                stringResource(R.string.onboarding_hide)
                            } else {
                                stringResource(R.string.onboarding_show)
                            },
                            color = palette.accent,
                        )
                    }
                }
            }

            AnimatedVisibility(visible = expanded) {
                Column(
                    modifier = Modifier.fillMaxWidth(),
                    verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
                ) {
                    OnboardingChecklistRow(
                        label = stringResource(R.string.onboarding_nearby_label),
                        completed = checklist.nearbyGranted,
                        actionLabel = stringResource(R.string.onboarding_grant_action),
                        pendingLabel = stringResource(R.string.onboarding_status_needed),
                        onAction = onGrantNearby,
                    )
                    OnboardingChecklistRow(
                        label = stringResource(R.string.onboarding_notifications_label),
                        completed = checklist.notificationsGranted,
                        actionLabel = stringResource(R.string.onboarding_grant_action),
                        pendingLabel = stringResource(R.string.onboarding_status_needed),
                        onAction = onGrantNotifications,
                    )
                    OnboardingChecklistRow(
                        label = stringResource(R.string.onboarding_battery_label),
                        completed = checklist.batteryOptimizationIgnored,
                        actionLabel = stringResource(R.string.onboarding_battery_action),
                        pendingLabel = stringResource(R.string.onboarding_status_recommended),
                        onAction = onAllowBackground,
                    )
                    OnboardingChecklistRow(
                        label = stringResource(R.string.onboarding_companion_label),
                        completed = checklist.companionAssociated,
                        actionLabel = null,
                        pendingLabel = stringResource(R.string.onboarding_status_recommended),
                        onAction = null,
                    )
                }
            }
        }
    }
}

@Composable
private fun OnboardingChecklistRow(
    label: String,
    completed: Boolean,
    actionLabel: String?,
    pendingLabel: String,
    onAction: (() -> Unit)?,
) {
    val palette = LocalKeynearPalette.current
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = label,
                style = MaterialTheme.typography.bodyMedium,
                color = palette.textPrimary,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                text = if (completed) stringResource(R.string.onboarding_status_ready) else pendingLabel,
                style = MaterialTheme.typography.labelMedium,
                color = if (completed) palette.accent else palette.textMuted,
            )
        }
        if (!completed && actionLabel != null && onAction != null) {
            Spacer(modifier = Modifier.width(KeynearSpacing.Sm))
            OutlinedButton(
                onClick = onAction,
                modifier = Modifier.defaultMinSize(
                    minWidth = KeynearSpacing.MinTouchTarget,
                    minHeight = KeynearSpacing.MinTouchTarget,
                ),
                shape = RoundedCornerShape(KeynearRadii.Xl),
            ) {
                Text(
                    text = actionLabel,
                    color = palette.textPrimary,
                )
            }
        }
    }
}

@Composable
private fun PairedDeviceRow(
    device: PairedDeviceUi,
    confirmingUnpair: Boolean,
    warningBg: Color,
    warningFg: Color,
    onToggleAutoAccept: (Boolean) -> Unit,
    onRenameDevice: (String) -> Unit,
    onRequestUnpair: () -> Unit,
    onCancelUnpair: () -> Unit,
    onConfirmUnpair: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    var editingRename by remember(device.deviceId) { mutableStateOf(false) }
    var renameDraft by remember(device.deviceId, device.displayName) { mutableStateOf(device.displayName) }
    var renameError by remember(device.deviceId) { mutableStateOf<String?>(null) }

    Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = device.displayName,
                    style = MaterialTheme.typography.bodyLarge,
                    color = palette.textPrimary,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                val endpointSuffix = if (device.lastEndpointId.isNotBlank()) {
                    " · ${device.lastEndpointId.take(8)}"
                } else {
                    ""
                }
                Text(
                    text = "${device.role.label()} · ${device.publicKeyFingerprint}$endpointSuffix",
                    style = MaterialTheme.typography.labelMedium,
                    color = palette.textSecondary,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            Spacer(modifier = Modifier.width(KeynearSpacing.Sm))
            OutlinedButton(
                onClick = {
                    renameDraft = device.displayName
                    renameError = null
                    editingRename = !editingRename
                },
                modifier = Modifier.defaultMinSize(
                    minWidth = KeynearSpacing.MinTouchTarget,
                    minHeight = KeynearSpacing.MinTouchTarget,
                ),
                shape = RoundedCornerShape(KeynearRadii.Xl),
            ) {
                Text(
                    text = stringResource(R.string.action_rename),
                    color = palette.textPrimary,
                )
            }
            Spacer(modifier = Modifier.width(KeynearSpacing.Xs))
            OutlinedButton(
                onClick = onRequestUnpair,
                modifier = Modifier.defaultMinSize(
                    minWidth = KeynearSpacing.MinTouchTarget,
                    minHeight = KeynearSpacing.MinTouchTarget,
                ),
                shape = RoundedCornerShape(KeynearRadii.Xl),
            ) {
                Text(
                    text = stringResource(R.string.action_unpair),
                    color = palette.textPrimary,
                )
            }
        }

        if (editingRename) {
            Surface(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(KeynearRadii.Xl),
                color = palette.surfaceTonal,
            ) {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(KeynearSpacing.Md),
                    verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
                ) {
                    Text(
                        text = stringResource(R.string.rename_dialog_title),
                        style = MaterialTheme.typography.titleMedium,
                        color = palette.textPrimary,
                    )
                    OutlinedTextField(
                        value = renameDraft,
                        onValueChange = {
                            renameDraft = it
                            renameError = null
                        },
                        singleLine = true,
                        label = { Text(text = stringResource(R.string.rename_input_label)) },
                        isError = renameError != null,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    renameError?.let { err ->
                        Text(
                            text = err,
                            style = MaterialTheme.typography.bodyMedium,
                            color = palette.onDangerTonal,
                        )
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm)) {
                        Button(
                            onClick = {
                                validateDisplayNameInput(renameDraft)
                                    .onSuccess { clean ->
                                        editingRename = false
                                        renameError = null
                                        onRenameDevice(clean)
                                    }
                                    .onFailure { err ->
                                        renameError = err.message ?: "Invalid device name"
                                    }
                            },
                            modifier = Modifier.defaultMinSize(
                                minWidth = KeynearSpacing.MinTouchTarget,
                                minHeight = KeynearSpacing.MinTouchTarget,
                            ),
                            shape = RoundedCornerShape(KeynearRadii.Xl),
                            colors = ButtonDefaults.buttonColors(
                                containerColor = palette.accent,
                                contentColor = palette.onAccent,
                            ),
                        ) {
                            Text(text = stringResource(R.string.action_save))
                        }
                        OutlinedButton(
                            onClick = {
                                editingRename = false
                                renameDraft = device.displayName
                                renameError = null
                            },
                            modifier = Modifier.defaultMinSize(
                                minWidth = KeynearSpacing.MinTouchTarget,
                                minHeight = KeynearSpacing.MinTouchTarget,
                            ),
                            shape = RoundedCornerShape(KeynearRadii.Xl),
                        ) {
                            Text(
                                text = stringResource(R.string.action_cancel),
                                color = palette.textPrimary,
                            )
                        }
                    }
                }
            }
        }

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = stringResource(R.string.auto_accept_files_label),
                style = MaterialTheme.typography.bodyMedium,
                color = palette.textSecondary,
                modifier = Modifier.weight(1f),
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            Switch(
                checked = device.autoAcceptFiles,
                onCheckedChange = onToggleAutoAccept,
                modifier = Modifier.defaultMinSize(
                    minWidth = KeynearSpacing.MinTouchTarget,
                    minHeight = KeynearSpacing.MinTouchTarget,
                ),
                colors = SwitchDefaults.colors(
                    checkedThumbColor = palette.onAccent,
                    checkedTrackColor = palette.accent,
                    uncheckedTrackColor = palette.surfaceTonal,
                ),
            )
        }

        if (confirmingUnpair) {
            Surface(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(KeynearRadii.Xl),
                color = warningBg,
            ) {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(KeynearSpacing.Md),
                    verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
                ) {
                    Text(
                        text = "Remove ${device.displayName} and clear stored Noise keys?",
                        style = MaterialTheme.typography.bodyMedium,
                        color = warningFg,
                    )
                    Row(horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm)) {
                        Button(
                            onClick = onConfirmUnpair,
                            modifier = Modifier.defaultMinSize(
                                minWidth = KeynearSpacing.MinTouchTarget,
                                minHeight = KeynearSpacing.MinTouchTarget,
                            ),
                            shape = RoundedCornerShape(KeynearRadii.Xl),
                            colors = ButtonDefaults.buttonColors(
                                containerColor = palette.errorTonal,
                                contentColor = palette.onDangerTonal,
                            ),
                        ) {
                            Text(text = stringResource(R.string.action_confirm_unpair))
                        }
                        OutlinedButton(
                            onClick = onCancelUnpair,
                            modifier = Modifier.defaultMinSize(
                                minWidth = KeynearSpacing.MinTouchTarget,
                                minHeight = KeynearSpacing.MinTouchTarget,
                            ),
                            shape = RoundedCornerShape(KeynearRadii.Xl),
                        ) {
                            Text(
                                text = stringResource(R.string.action_cancel),
                                color = warningFg,
                            )
                        }
                    }
                }
            }
        }
    }
}
