package org.keynear.app

import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
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
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
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
import org.keynear.app.core.TransferDirectionUi
import org.keynear.app.core.TransferStateUi
import org.keynear.app.core.TransferUi
import org.keynear.app.core.VisibilityState
import org.keynear.app.core.formatByteCount
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
    val onOpenAppSettings: () -> Unit,
    val onInitiatePairing: (String) -> Unit,
    val onApprovePairing: () -> Unit,
    val onRejectPairing: () -> Unit,
    val onToggleAutoAccept: (String, Boolean) -> Unit,
    val onAcceptTransfer: (String) -> Unit,
    val onDeclineTransfer: (String) -> Unit,
    val onCancelTransfer: (String) -> Unit,
    val onRequestUnpair: (String) -> Unit,
    val onCancelUnpair: () -> Unit,
    val onConfirmUnpair: (String) -> Unit,
)

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val bridge = CoreClientProvider.init(applicationContext)
        setContent {
            KeynearTheme {
                KeynearFoundationScreen(bridge = bridge)
            }
        }
    }
}

private fun snapshotFromState(state: CoreState): CoreSnapshot? = when (state) {
    is CoreState.Empty -> state.snapshot
    is CoreState.Ready -> state.snapshot
    is CoreState.Error -> state.previous
    CoreState.Loading -> null
}

@Composable
private fun KeynearFoundationScreen(bridge: CoreBridge) {
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

    val syncBleAndLoad: suspend () -> Unit = {
        val loaded = withContext(Dispatchers.IO) { bridge.loadState() }
        val vis = snapshotFromState(loaded)?.visibility ?: VisibilityState.PAIRED_ONLY
        withContext(Dispatchers.IO) {
            KeynearBleManager.startOrSync(context, vis)
            KeynearForegroundService.syncIfPermitted(context)
        }
        uiState = withContext(Dispatchers.IO) { bridge.loadState() }
    }

    val permissionLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.RequestMultiplePermissions(),
    ) {
        scope.launch { syncBleAndLoad() }
    }

    LaunchedEffect(bridge) {
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
            context.startActivity(Intent(Settings.ACTION_BLUETOOTH_SETTINGS))
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
        onOpenAppSettings = {
            val intent = Intent(
                Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                Uri.fromParts("package", context.packageName, null),
            )
            context.startActivity(intent)
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
    confirmUnpairDeviceId: String?,
    actions: SnapshotActions,
) {
    val palette = LocalKeynearPalette.current
    val dark = isSystemInDarkTheme()
    val warningBg = if (dark) Color(0xFF381C00) else Color(0xFFFFF6D3)
    val warningFg = if (dark) Color(0xFFFFEAA5) else Color(0xFF863300)

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
private fun PairedDeviceRow(
    device: PairedDeviceUi,
    confirmingUnpair: Boolean,
    warningBg: Color,
    warningFg: Color,
    onToggleAutoAccept: (Boolean) -> Unit,
    onRequestUnpair: () -> Unit,
    onCancelUnpair: () -> Unit,
    onConfirmUnpair: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
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
                Text(
                    text = "${device.role.label()} · ${device.publicKeyFingerprint}",
                    style = MaterialTheme.typography.labelMedium,
                    color = palette.textSecondary,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            Spacer(modifier = Modifier.width(KeynearSpacing.Sm))
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
