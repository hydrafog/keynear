package org.keynear.app

import android.content.Intent
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
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
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
import org.keynear.app.core.PairedDeviceUi
import org.keynear.app.core.PendingPairingUi
import org.keynear.app.core.VisibilityState
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

                val onCycle: () -> Unit = {
                    scope.launch {
                        val cycled = withContext(Dispatchers.IO) { bridge.cycleVisibility() }
                        val vis = snapshotFromState(cycled)?.visibility ?: VisibilityState.PAIRED_ONLY
                        withContext(Dispatchers.IO) {
                            KeynearBleManager.startOrSync(context, vis)
                            KeynearForegroundService.syncIfPermitted(context)
                        }
                        uiState = withContext(Dispatchers.IO) { bridge.loadState() }
                    }
                }

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
                                    onCycleVisibility = onCycle,
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
                            }
                        }
                    }
                    ScreenCategory.SNAPSHOT -> {
                        val snapshot = snapshotFromState(uiState)
                        if (snapshot != null) {
                            SnapshotContent(
                                snapshot = snapshot,
                                confirmUnpairDeviceId = confirmUnpairDeviceId,
                                onCycleVisibility = onCycle,
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
private fun SnapshotContent(
    snapshot: CoreSnapshot,
    confirmUnpairDeviceId: String?,
    onCycleVisibility: () -> Unit,
    onRequestBluetoothPermissions: () -> Unit,
    onOpenBluetoothSettings: () -> Unit,
    onInitiatePairing: (String) -> Unit,
    onApprovePairing: () -> Unit,
    onRejectPairing: () -> Unit,
    onRequestUnpair: (String) -> Unit,
    onCancelUnpair: () -> Unit,
    onConfirmUnpair: (String) -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val dark = isSystemInDarkTheme()
    val warningBg = if (dark) Color(0xFF381C00) else Color(0xFFFFF6D3)
    val warningFg = if (dark) Color(0xFFFFEAA5) else Color(0xFF863300)

    if (snapshot.bluetoothStatus != BluetoothStatus.READY || snapshot.warningMessage != null) {
        BluetoothWarningCard(
            status = snapshot.bluetoothStatus,
            message = snapshot.warningMessage,
            onRequestPermissions = onRequestBluetoothPermissions,
            onOpenBluetoothSettings = onOpenBluetoothSettings,
        )
    }

    snapshot.pendingPairing?.let { pending ->
        PendingPairingCard(
            pending = pending,
            onApprove = onApprovePairing,
            onReject = onRejectPairing,
        )
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
                onClick = onCycleVisibility,
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
                    NearbyPeerRow(peer = peer, onPair = { onInitiatePairing(peer.address) })
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
                        onRequestUnpair = { onRequestUnpair(device.deviceId) },
                        onCancelUnpair = onCancelUnpair,
                        onConfirmUnpair = { onConfirmUnpair(device.deviceId) },
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
