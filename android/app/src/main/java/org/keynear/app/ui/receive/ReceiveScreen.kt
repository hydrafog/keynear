package org.keynear.app.ui.receive

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.core.CoreSnapshot
import org.keynear.app.core.DeviceRoleUi
import org.keynear.app.core.PermissionsChecklistState
import org.keynear.app.core.VisibilityState
import org.keynear.app.core.orbitFor
import org.keynear.app.ui.components.ActionButton
import org.keynear.app.ui.components.ActiveTransfersStrip
import org.keynear.app.ui.components.PendingPairingBanner
import org.keynear.app.ui.components.TransportWarningBanner
import org.keynear.app.ui.icons.IconChevronRight
import org.keynear.app.ui.icons.IconEyeOff
import org.keynear.app.ui.icons.IconGlobe
import org.keynear.app.ui.icons.IconLaptop
import org.keynear.app.ui.icons.IconShield
import org.keynear.app.ui.icons.IconSmartphone
import org.keynear.app.ui.icons.IconUsers
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.app.ui.navigation.OrbitPinUi
import org.keynear.app.ui.theme.LocalKeynearPalette
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin

@Composable
fun ReceiveScreen(
    snapshot: CoreSnapshot,
    permissions: PermissionsChecklistState,
    localDeviceName: String,
    sheetExpanded: Boolean,
    onToggleSheetExpanded: () -> Unit,
    actions: KeynearActions,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    val pins = buildMapPins(snapshot)
    val radarActive = snapshot.visibility != VisibilityState.HIDDEN

    Column(
        modifier = modifier
            .fillMaxSize()
            .background(palette.mapGround),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            if (permissions.missingPermissionsCount > 0) {
                Surface(
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(14.dp),
                    color = palette.errorTonal,
                    onClick = actions.onOpenPermissions,
                ) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 14.dp, vertical = 10.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        IconShield(modifier = Modifier.size(18.dp), tint = palette.onDangerTonal)
                        Text(
                            text = "${permissions.missingPermissionsCount} Android permission(s) required",
                            color = palette.onDangerTonal,
                            fontSize = 13.sp,
                            fontWeight = FontWeight.SemiBold,
                            modifier = Modifier.weight(1f),
                        )
                        Text(
                            text = "Grant",
                            color = palette.onDangerTonal,
                            fontSize = 13.sp,
                            fontWeight = FontWeight.Bold,
                        )
                    }
                }
            }

            TransportWarningBanner(snapshot = snapshot, actions = actions)

            VisibilitySegmentedControl(
                current = snapshot.visibility,
                onSelect = actions.onSetVisibility,
            )
        }

        Box(
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth(),
            contentAlignment = Alignment.Center,
        ) {
            RadarCanvasLayer(radarActive = radarActive)
            OrbitPinsLayer(pins = pins, actions = actions)
            CenterSelfNode(
                localDeviceName = localDeviceName,
                visibility = snapshot.visibility,
                remainingSecs = snapshot.everyoneRemainingSecs,
            )
        }

        snapshot.pendingPairing?.let { prompt ->
            PendingPairingBanner(
                prompt = prompt,
                onApprove = actions.onApprovePairing,
                onReject = actions.onRejectPairing,
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 6.dp),
            )
        }

        val activeTransfers = snapshot.activeTransfers.filter { !it.state.isTerminal() }
        if (activeTransfers.isNotEmpty()) {
            ActiveTransfersStrip(
                transfers = activeTransfers,
                onAccept = actions.onAcceptTransfer,
                onDecline = actions.onDeclineTransfer,
                onCancel = actions.onCancelTransfer,
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 6.dp),
            )
        }

        DevicesBottomSheet(
            snapshot = snapshot,
            pins = pins,
            expanded = sheetExpanded,
            onToggleExpanded = onToggleSheetExpanded,
            actions = actions,
        )
    }
}

@Composable
private fun VisibilitySegmentedControl(
    current: VisibilityState,
    onSelect: (VisibilityState) -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val items = listOf(
        Triple(VisibilityState.HIDDEN, "Hidden", 0),
        Triple(VisibilityState.PAIRED_ONLY, "Paired", 1),
        Triple(VisibilityState.EVERYONE_NEARBY, "Everyone", 2),
    )

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp))
            .background(palette.trackWell)
            .padding(4.dp),
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        items.forEach { (mode, label, kind) ->
            val selected = current == mode
            val bg = if (selected) palette.loading else Color.Transparent
            val fg = if (selected) Color(0xFFFFFFFE) else palette.textSecondary
            Row(
                modifier = Modifier
                    .weight(1f)
                    .heightIn(min = 44.dp)
                    .clip(RoundedCornerShape(12.dp))
                    .background(bg)
                    .clickable { onSelect(mode) }
                    .padding(horizontal = 8.dp, vertical = 10.dp),
                horizontalArrangement = Arrangement.Center,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                when (kind) {
                    0 -> IconEyeOff(tint = fg)
                    1 -> IconUsers(tint = fg)
                    else -> IconGlobe(tint = fg)
                }
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    text = label,
                    color = fg,
                    fontSize = 13.sp,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

@Composable
private fun RadarCanvasLayer(radarActive: Boolean) {
    val palette = LocalKeynearPalette.current
    val transition = rememberInfiniteTransition(label = "radarPulse")
    val pulse by transition.animateFloat(
        initialValue = 0f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 2600, easing = LinearEasing),
            repeatMode = RepeatMode.Restart,
        ),
        label = "pulse",
    )

    Canvas(modifier = Modifier.fillMaxSize()) {
        val c = Offset(size.width / 2f, size.height / 2f)
        val loading = palette.loading
        val ringColor = palette.mapRing

        drawCircle(
            brush = Brush.radialGradient(
                colors = listOf(
                    loading.copy(alpha = if (radarActive) 0.22f else 0.04f),
                    Color.Transparent,
                ),
                center = c,
                radius = 150.dp.toPx(),
            ),
            radius = 150.dp.toPx(),
            center = c,
        )

        drawCircle(
            color = ringColor,
            radius = 80.dp.toPx(),
            center = c,
            style = Stroke(width = 1.dp.toPx()),
        )
        drawCircle(
            color = ringColor,
            radius = 180.dp.toPx(),
            center = c,
            style = Stroke(width = 1.dp.toPx()),
        )

        if (radarActive) {
            listOf(pulse, (pulse + 0.5f) % 1f).forEach { phase ->
                val radius = (32f + phase * 128f).dp.toPx()
                val alpha = (1f - phase) * 0.34f
                drawCircle(
                    color = loading.copy(alpha = alpha),
                    radius = radius,
                    center = c,
                    style = Stroke(width = 1.6.dp.toPx()),
                )
            }
        }
    }
}

@Composable
private fun CenterSelfNode(
    localDeviceName: String,
    visibility: VisibilityState,
    remainingSecs: Long,
) {
    val palette = LocalKeynearPalette.current
    val active = visibility != VisibilityState.HIDDEN
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Surface(
            modifier = Modifier.size(64.dp),
            shape = CircleShape,
            color = if (active) palette.loading else palette.surface,
            shadowElevation = if (active) 8.dp else 3.dp,
        ) {
            Box(contentAlignment = Alignment.Center) {
                IconSmartphone(
                    modifier = Modifier.size(28.dp),
                    tint = if (active) Color(0xFFFFFFFE) else palette.textMuted,
                )
            }
        }

        Surface(
            shape = RoundedCornerShape(12.dp),
            color = palette.surface,
            shadowElevation = 3.dp,
        ) {
            Row(
                modifier = Modifier.padding(horizontal = 12.dp, vertical = 5.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                Text(
                    text = localDeviceName,
                    color = palette.textPrimary,
                    fontSize = 12.sp,
                    fontWeight = FontWeight.SemiBold,
                )
                if (visibility == VisibilityState.EVERYONE_NEARBY) {
                    val mins = (remainingSecs.coerceAtLeast(0L) / 60L)
                    val secs = (remainingSecs.coerceAtLeast(0L) % 60L)
                    Text(
                        text = "$mins:${secs.toString().padStart(2, '0')}",
                        color = palette.loading,
                        fontSize = 12.sp,
                        fontWeight = FontWeight.Bold,
                    )
                }
            }
        }
    }
}

@Composable
private fun OrbitPinsLayer(
    pins: List<OrbitPinUi>,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current
    val transition = rememberInfiniteTransition(label = "orbit")
    val orbitProgress by transition.animateFloat(
        initialValue = 0f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 40_000, easing = LinearEasing),
            repeatMode = RepeatMode.Restart,
        ),
        label = "orbitPhase",
    )

    pins.forEachIndexed { idx, pin ->
        val orbit = orbitFor(pin.rssi, idx, pins.size)
        val fraction = (orbit.phaseSec / orbit.durationSec.toFloat()) + orbitProgress
        val angle = (fraction * 2f * PI.toFloat()) - (PI.toFloat() / 2f)
        val radiusDp = orbit.radiusPx * 0.72f
        val xOffset = (cos(angle) * radiusDp).dp
        val yOffset = (sin(angle) * radiusDp).dp

        Column(
            modifier = Modifier
                .offset(x = xOffset, y = yOffset)
                .clickable {
                    if (pin.isPaired) {
                        actions.onOpenDeviceDetail(pin.id)
                    } else {
                        actions.onInitiatePairing(pin.peerAddress)
                    }
                },
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            Surface(
                modifier = Modifier.size(56.dp),
                shape = CircleShape,
                color = palette.surface,
                shadowElevation = 5.dp,
            ) {
                Box(contentAlignment = Alignment.Center) {
                    if (pin.role == DeviceRoleUi.DESKTOP) {
                        IconLaptop(modifier = Modifier.size(24.dp), tint = palette.loading)
                    } else {
                        IconSmartphone(modifier = Modifier.size(24.dp), tint = palette.loading)
                    }
                }
            }

            Surface(
                shape = RoundedCornerShape(10.dp),
                color = palette.surface,
                shadowElevation = 3.dp,
            ) {
                Column(
                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    Text(
                        text = pin.displayName,
                        color = palette.textPrimary,
                        fontSize = 11.sp,
                        fontWeight = FontWeight.Bold,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Text(
                        text = pin.statusLabel,
                        color = when {
                            !pin.isPaired -> palette.onWarningTonal
                            pin.isOnline -> palette.loading
                            else -> palette.textMuted
                        },
                        fontSize = 10.sp,
                        fontWeight = FontWeight.SemiBold,
                    )
                    Text(
                        text = "${pin.rssi} dBm",
                        color = palette.textMuted,
                        fontSize = 10.sp,
                    )
                }
            }
        }
    }
}

@Composable
private fun DevicesBottomSheet(
    snapshot: CoreSnapshot,
    pins: List<OrbitPinUi>,
    expanded: Boolean,
    onToggleExpanded: () -> Unit,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current
    val sheetListHeight by animateDpAsState(
        targetValue = if (expanded) 248.dp else 116.dp,
        label = "sheetHeight",
    )

    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp),
        shape = RoundedCornerShape(topStart = 24.dp, topEnd = 24.dp),
        color = palette.surface,
        shadowElevation = 6.dp,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 10.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Box(
                modifier = Modifier
                    .align(Alignment.CenterHorizontally)
                    .width(36.dp)
                    .height(4.dp)
                    .clip(CircleShape)
                    .background(palette.sheetHandle)
                    .clickable(onClick = onToggleExpanded),
            )

            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = "Devices in range",
                        color = palette.textPrimary,
                        fontSize = 16.sp,
                        fontWeight = FontWeight.Bold,
                    )
                    Text(
                        text = "${pins.size} device(s) in range",
                        color = palette.textMuted,
                        fontSize = 12.sp,
                    )
                }
                Text(
                    text = if (expanded) "Collapse" else "All devices",
                    color = palette.loading,
                    fontSize = 13.sp,
                    fontWeight = FontWeight.SemiBold,
                    modifier = Modifier
                        .clip(RoundedCornerShape(8.dp))
                        .clickable(onClick = onToggleExpanded)
                        .padding(horizontal = 8.dp, vertical = 6.dp),
                )
            }

            if (pins.isEmpty()) {
                Text(
                    text = "Discovery is hidden. Switch to Paired or Everyone to see nearby devices.",
                    color = palette.textMuted,
                    fontSize = 13.sp,
                    modifier = Modifier.padding(vertical = 8.dp),
                )
            } else {
                LazyColumn(
                    modifier = Modifier
                        .fillMaxWidth()
                        .heightIn(max = sheetListHeight),
                    verticalArrangement = Arrangement.spacedBy(6.dp),
                    contentPadding = PaddingValues(bottom = 4.dp),
                ) {
                    items(pins, key = { it.id }) { pin ->
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .clip(RoundedCornerShape(12.dp))
                                .clickable {
                                    if (pin.isPaired) {
                                        actions.onOpenDeviceDetail(pin.id)
                                    } else {
                                        actions.onInitiatePairing(pin.peerAddress)
                                    }
                                }
                                .padding(horizontal = 4.dp, vertical = 6.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(12.dp),
                        ) {
                            Box(
                                modifier = Modifier
                                    .size(44.dp)
                                    .clip(RoundedCornerShape(12.dp))
                                    .background(if (pin.isPaired && pin.isOnline) palette.loadingTonal else palette.surfaceTonal),
                                contentAlignment = Alignment.Center,
                            ) {
                                val iconTint = if (pin.isPaired && pin.isOnline) palette.onLoadingTonal else palette.textMuted
                                if (pin.role == DeviceRoleUi.DESKTOP) {
                                    IconLaptop(modifier = Modifier.size(20.dp), tint = iconTint)
                                } else {
                                    IconSmartphone(modifier = Modifier.size(20.dp), tint = iconTint)
                                }
                            }
                            Column(modifier = Modifier.weight(1f)) {
                                Text(
                                    text = pin.displayName,
                                    color = palette.textPrimary,
                                    fontSize = 14.sp,
                                    fontWeight = FontWeight.SemiBold,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                )
                                val pairedDev = snapshot.pairedDevices.firstOrNull { it.deviceId == pin.id }
                                val sub = if (pairedDev != null) {
                                    "${pairedDev.role.label()} · Paired ${pairedDev.pairedAt}"
                                } else {
                                    "Nearby peer · ${pin.rssi} dBm"
                                }
                                Text(
                                    text = sub,
                                    color = palette.textMuted,
                                    fontSize = 12.sp,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                )
                            }
                            if (pin.isPaired) {
                                IconChevronRight(tint = palette.textMuted)
                            } else {
                                ActionButton(
                                    label = "Pair",
                                    onClick = { actions.onInitiatePairing(pin.peerAddress) },
                                    primary = true,
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

private fun buildMapPins(snapshot: CoreSnapshot): List<OrbitPinUi> {
    val result = mutableListOf<OrbitPinUi>()
    snapshot.pairedDevices.forEach { dev ->
        result.add(
            OrbitPinUi(
                id = dev.deviceId,
                displayName = dev.displayName,
                statusLabel = if (dev.isOnline) "Wi-Fi" else "Standby",
                role = dev.role,
                rssi = dev.rssi,
                isPaired = true,
                isOnline = dev.isOnline,
                peerAddress = dev.lastBleAddress,
            ),
        )
    }
    if (snapshot.visibility != VisibilityState.HIDDEN) {
        snapshot.nearbyPeers.filter { !it.isPaired }.forEach { peer ->
            result.add(
                OrbitPinUi(
                    id = peer.address,
                    displayName = peer.displayName,
                    statusLabel = "Tap to pair",
                    role = peer.role,
                    rssi = peer.rssi,
                    isPaired = false,
                    isOnline = true,
                    peerAddress = peer.address,
                ),
            )
        }
    }
    return result
}
