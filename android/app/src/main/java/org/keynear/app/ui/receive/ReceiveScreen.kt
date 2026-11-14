package org.keynear.app.ui.receive

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.FastOutSlowInEasing
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
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
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
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.core.CoreSnapshot
import org.keynear.app.core.DeviceRoleUi
import org.keynear.app.core.VisibilityState
import org.keynear.app.core.orbitFor
import org.keynear.app.ui.components.ActiveTransfersStrip
import org.keynear.app.ui.components.PendingPairingBanner
import org.keynear.app.ui.components.TactileSurface
import org.keynear.app.ui.components.TactileTone
import org.keynear.app.ui.components.tactileCushion
import org.keynear.app.ui.icons.IconChevronRight
import org.keynear.app.ui.icons.IconEyeOff
import org.keynear.app.ui.icons.IconLaptop
import org.keynear.app.ui.icons.IconRadio
import org.keynear.app.ui.icons.IconShieldCheck
import org.keynear.app.ui.icons.IconSmartphone
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.app.ui.navigation.OrbitPinUi
import org.keynear.app.ui.theme.KeynearRadii
import org.keynear.app.ui.theme.KeynearShapes
import org.keynear.app.ui.theme.LocalKeynearPalette
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin

@Composable
fun ReceiveScreen(
    snapshot: CoreSnapshot,
    localDeviceName: String,
    sheetExpanded: Boolean,
    onSetSheetExpanded: (Boolean) -> Unit,
    actions: KeynearActions,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    val pins = buildMapPins(snapshot)
    val radarActive = snapshot.visibility != VisibilityState.HIDDEN

    Column(
        modifier = modifier
            .fillMaxSize()
            .background(
                brush = Brush.verticalGradient(
                    0.0f to palette.mapBlue,
                    0.65f to palette.mapBlue,
                    1.0f to palette.canvas,
                ),
            ),
    ) {
        Box(
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth(),
        ) {
            RadarCanvasLayer(radarActive = radarActive)

            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(top = 16.dp),
                contentAlignment = Alignment.Center,
            ) {
                CenterSelfNode(
                    localDeviceName = localDeviceName,
                    visibility = snapshot.visibility,
                    remainingSecs = snapshot.everyoneRemainingSecs,
                )
                OrbitPinsLayer(
                    pins = pins,
                    actions = actions,
                )
            }

            Column(
                modifier = Modifier
                    .align(Alignment.TopCenter)
                    .fillMaxWidth()
                    .padding(start = 12.dp, end = 12.dp, top = 12.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                VisibilitySegmentedControl(
                    current = snapshot.visibility,
                    onSelect = actions.onSetVisibility,
                )

                snapshot.pendingPairing?.let { prompt ->
                    PendingPairingBanner(
                        prompt = prompt,
                        onApprove = actions.onApprovePairing,
                        onReject = actions.onRejectPairing,
                    )
                }

                if (snapshot.activeTransfers.isNotEmpty()) {
                    ActiveTransfersStrip(
                        transfers = snapshot.activeTransfers,
                        onAccept = actions.onAcceptTransfer,
                        onDecline = actions.onDeclineTransfer,
                        onCancel = actions.onCancelTransfer,
                    )
                }
            }
        }

        DevicesBottomSheet(
            pins = pins,
            expanded = sheetExpanded,
            onSetExpanded = onSetSheetExpanded,
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
    val selectedIndex = items.indexOfFirst { it.first == current }.coerceAtLeast(0)
    val wellColor = if (palette.isDark) palette.trackWell else Color(0xFFD2DFF4)

    Box(
        modifier = Modifier
            .fillMaxWidth()
            .clip(KeynearShapes.Card)
            .background(wellColor)
            .tactileCushion(
                tone = TactileTone.INSET,
                isDark = palette.isDark,
                cornerRadius = KeynearRadii.TwoXl,
            )
            .padding(6.dp),
    ) {
        BoxWithConstraints(modifier = Modifier.fillMaxWidth()) {
            val slotWidth = maxWidth / 3f
            val indicatorOffset by animateDpAsState(
                targetValue = slotWidth * selectedIndex,
                label = "visibilityPill",
            )

            TactileSurface(
                modifier = Modifier
                    .offset(x = indicatorOffset)
                    .width(slotWidth)
                    .height(56.dp),
                shape = KeynearShapes.Xl,
                containerColor = palette.loading,
                tone = TactileTone.ACCENT,
                cornerRadius = KeynearRadii.Xl,
            ) {
                Box(modifier = Modifier.width(slotWidth).height(56.dp))
            }

            Row(modifier = Modifier.fillMaxWidth()) {
                items.forEach { (mode, label, kind) ->
                    val selected = current == mode
                    val fg by animateColorAsState(
                        targetValue = if (selected) Color(0xFFFFFFFE) else palette.textSecondary,
                        label = "visibilityFg",
                    )
                    Row(
                        modifier = Modifier
                            .weight(1f)
                            .heightIn(min = 56.dp)
                            .clip(KeynearShapes.Xl)
                            .clickable { onSelect(mode) }
                            .padding(horizontal = 10.dp),
                        horizontalArrangement = Arrangement.Center,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        when (kind) {
                            0 -> IconEyeOff(modifier = Modifier.size(18.dp), tint = fg)
                            1 -> IconShieldCheck(modifier = Modifier.size(18.dp), tint = fg)
                            else -> IconRadio(modifier = Modifier.size(18.dp), tint = fg)
                        }
                        Spacer(modifier = Modifier.width(8.dp))
                        Text(
                            text = label,
                            color = fg,
                            fontSize = 14.sp,
                            fontWeight = FontWeight.Bold,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun RadarCanvasLayer(radarActive: Boolean) {
    val palette = LocalKeynearPalette.current
    val transition = rememberInfiniteTransition(label = "radarPulse")
    val beaconPulse by transition.animateFloat(
        initialValue = 0f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 6000, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "beacon",
    )
    val pulse by transition.animateFloat(
        initialValue = 0f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 4800, easing = LinearEasing),
            repeatMode = RepeatMode.Restart,
        ),
        label = "pulse",
    )

    Canvas(modifier = Modifier.fillMaxSize()) {
        val c = Offset(size.width / 2f, size.height / 2f + 12.dp.toPx())
        val loading = palette.loading
        val baseRingColor = if (palette.isDark) Color(0x1EFFFFFF) else Color(0x16051131)
        val midRingColor = if (palette.isDark) Color(0x14FFFFFF) else Color(0x10051131)
        val outerRingColor = palette.mapRing
        val glowColor = if (palette.isDark) Color(0x220053E2) else Color(0x99FFFFFE)

        drawCircle(
            brush = Brush.radialGradient(
                colors = listOf(glowColor, Color.Transparent),
                center = c,
                radius = 190.dp.toPx(),
            ),
            radius = 190.dp.toPx(),
            center = c,
        )

        val beaconAlpha = if (radarActive) 0.78f - beaconPulse * 0.42f else 0f
        val beaconScale = if (radarActive) 1f + beaconPulse * 0.08f else 1f
        val beaconRadius = 185.dp.toPx() * beaconScale

        if (beaconAlpha > 0f) {
            drawCircle(
                brush = Brush.radialGradient(
                    colors = listOf(
                        loading.copy(alpha = 0.28f * beaconAlpha),
                        loading.copy(alpha = 0.09f * beaconAlpha),
                        Color.Transparent,
                    ),
                    center = c,
                    radius = beaconRadius,
                ),
                radius = beaconRadius,
                center = c,
            )
        }

        drawCircle(
            color = baseRingColor,
            radius = 78.dp.toPx(),
            center = c,
            style = Stroke(width = 1.5.dp.toPx()),
        )
        drawCircle(
            color = midRingColor,
            radius = 136.dp.toPx(),
            center = c,
            style = Stroke(width = 1.25.dp.toPx()),
        )
        drawCircle(
            color = outerRingColor,
            radius = 196.dp.toPx(),
            center = c,
            style = Stroke(width = 1.dp.toPx()),
        )

        if (radarActive) {
            listOf(pulse, (pulse + 0.5f) % 1f).forEach { phase ->
                val radius = (0.28f + phase * 1.12f) * 180.dp.toPx()
                val alpha = 0.26f * (1f - phase) * 0.65f
                drawCircle(
                    color = loading.copy(alpha = alpha),
                    radius = radius,
                    center = c,
                    style = Stroke(width = 20.dp.toPx()),
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
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        TactileSurface(
            modifier = Modifier.size(82.dp),
            shape = CircleShape,
            containerColor = if (active) palette.loading else palette.surface,
            tone = if (active) TactileTone.ACCENT else TactileTone.SURFACE,
            cornerRadius = 41.dp,
            isCircle = true,
        ) {
            Box(modifier = Modifier.size(82.dp), contentAlignment = Alignment.Center) {
                IconSmartphone(
                    modifier = Modifier.size(36.dp),
                    tint = if (active) Color(0xFFFFFFFE) else palette.textMuted,
                )
            }
        }

        TactileSurface(
            shape = KeynearShapes.Xl,
            containerColor = palette.surface,
            tone = TactileTone.SURFACE,
            cornerRadius = KeynearRadii.Xl,
        ) {
            Row(
                modifier = Modifier.padding(horizontal = 18.dp, vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Text(
                    text = localDeviceName,
                    color = palette.textPrimary,
                    fontSize = 14.sp,
                    fontWeight = FontWeight.Bold,
                )
                if (visibility == VisibilityState.EVERYONE_NEARBY) {
                    val mins = (remainingSecs.coerceAtLeast(0L) / 60L)
                    val secs = (remainingSecs.coerceAtLeast(0L) % 60L)
                    Text(
                        text = "$mins:${secs.toString().padStart(2, '0')}",
                        color = palette.loading,
                        fontSize = 14.sp,
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
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            TactileSurface(
                modifier = Modifier.size(64.dp),
                shape = CircleShape,
                containerColor = palette.surface,
                tone = TactileTone.SURFACE,
                cornerRadius = 32.dp,
                isCircle = true,
            ) {
                Box(modifier = Modifier.size(64.dp), contentAlignment = Alignment.Center) {
                    if (pin.role == DeviceRoleUi.DESKTOP) {
                        IconLaptop(modifier = Modifier.size(28.dp), tint = palette.loading)
                    } else {
                        IconSmartphone(modifier = Modifier.size(28.dp), tint = palette.loading)
                    }
                }
            }

            TactileSurface(
                shape = KeynearShapes.Xl,
                containerColor = palette.surface,
                tone = TactileTone.SURFACE,
                cornerRadius = KeynearRadii.Xl,
            ) {
                Column(
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 7.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    Text(
                        text = pin.displayName,
                        color = palette.textPrimary,
                        fontSize = 13.sp,
                        fontWeight = FontWeight.Bold,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Text(
                        text = pin.statusLabel,
                        color = palette.textMuted,
                        fontSize = 12.sp,
                    )
                    Text(
                        text = "${pin.rssi} dBm",
                        color = palette.textMuted,
                        fontSize = 12.sp,
                    )
                }
            }
        }
    }
}

@Composable
private fun DevicesBottomSheet(
    pins: List<OrbitPinUi>,
    expanded: Boolean,
    onSetExpanded: (Boolean) -> Unit,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current
    val density = LocalDensity.current
    val collapsedHeight = if (pins.isEmpty()) 0.dp else 132.dp
    val expandedHeight = if (pins.isEmpty()) 156.dp else 252.dp
    val targetHeight = if (expanded) expandedHeight else collapsedHeight
    val animatedHeight by animateDpAsState(
        targetValue = targetHeight,
        label = "sheetHeight",
    )
    var dragHeight by remember { mutableStateOf<Dp?>(null) }
    val currentHeight = dragHeight ?: animatedHeight

    TactileSurface(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp)
            .pointerInput(pins.isEmpty(), expanded) {
                detectVerticalDragGestures(
                    onDragStart = {
                        dragHeight = currentHeight
                    },
                    onVerticalDrag = { _, dragAmount ->
                        val base = dragHeight ?: currentHeight
                        val deltaDp = with(density) { (-dragAmount).toDp() }
                        dragHeight = (base + deltaDp).coerceIn(0.dp, expandedHeight)
                    },
                    onDragEnd = {
                        val finalHeight = dragHeight ?: currentHeight
                        val midpoint = (collapsedHeight + expandedHeight) * 0.5f
                        onSetExpanded(finalHeight >= midpoint)
                        dragHeight = null
                    },
                    onDragCancel = {
                        dragHeight = null
                    },
                )
            },
        shape = KeynearShapes.Sheet,
        containerColor = palette.surface,
        tone = TactileTone.SHEET,
        cornerRadius = KeynearRadii.ThreeXl,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(start = 16.dp, end = 16.dp, top = 10.dp, bottom = 12.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .heightIn(min = 44.dp)
                    .clickable { onSetExpanded(!expanded) }
                    .padding(bottom = 6.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Box(
                    modifier = Modifier
                        .width(48.dp)
                        .height(6.dp)
                        .clip(CircleShape)
                        .background(palette.sheetHandle),
                )
                Spacer(modifier = Modifier.height(10.dp))
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Text(
                        text = "${pins.size} computer(s) in range",
                        color = palette.textPrimary,
                        fontSize = 14.sp,
                        fontWeight = FontWeight.Bold,
                    )
                    Text(
                        text = if (expanded) "Collapse" else "All devices",
                        color = palette.loading,
                        fontSize = 14.sp,
                        fontWeight = FontWeight.Bold,
                    )
                }
            }

            if (currentHeight > 0.dp) {
                if (pins.isEmpty()) {
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(currentHeight)
                            .clip(KeynearShapes.Xl)
                            .background(palette.surfaceTonal)
                            .padding(16.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        Column(
                            horizontalAlignment = Alignment.CenterHorizontally,
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            IconRadio(
                                modifier = Modifier.size(22.dp),
                                tint = palette.textMuted,
                            )
                            Text(
                                text = "No computers in range",
                                color = palette.textMuted,
                                fontSize = 14.sp,
                                fontWeight = FontWeight.SemiBold,
                            )
                        }
                    }
                } else {
                    LazyColumn(
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(currentHeight),
                        verticalArrangement = Arrangement.spacedBy(6.dp),
                        contentPadding = PaddingValues(bottom = 4.dp),
                    ) {
                    items(pins, key = { it.id }) { pin ->
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .heightIn(min = 62.dp)
                                .clip(KeynearShapes.Xl)
                                .clickable {
                                    if (pin.isPaired) {
                                        actions.onOpenDeviceDetail(pin.id)
                                    } else {
                                        actions.onInitiatePairing(pin.peerAddress)
                                    }
                                }
                                .padding(horizontal = 14.dp, vertical = 10.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(14.dp),
                        ) {
                            TactileSurface(
                                modifier = Modifier.size(44.dp),
                                shape = KeynearShapes.Xl,
                                containerColor = palette.loadingTonal,
                                tone = TactileTone.TONAL,
                                cornerRadius = KeynearRadii.Xl,
                            ) {
                                Box(modifier = Modifier.size(44.dp), contentAlignment = Alignment.Center) {
                                    if (pin.role == DeviceRoleUi.DESKTOP) {
                                        IconLaptop(modifier = Modifier.size(20.dp), tint = palette.onLoadingTonal)
                                    } else {
                                        IconSmartphone(modifier = Modifier.size(20.dp), tint = palette.onLoadingTonal)
                                    }
                                }
                            }
                            Column(modifier = Modifier.weight(1f)) {
                                Row(
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                                ) {
                                    Text(
                                        text = pin.displayName,
                                        color = palette.textPrimary,
                                        fontSize = 16.sp,
                                        fontWeight = FontWeight.Bold,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                    )
                                    if (!pin.isPaired) {
                                        TactileSurface(
                                            shape = KeynearShapes.Lg,
                                            containerColor = palette.loadingTonal,
                                            tone = TactileTone.TONAL,
                                            cornerRadius = KeynearRadii.Lg,
                                        ) {
                                            Text(
                                                text = "Pair",
                                                color = palette.onLoadingTonal,
                                                fontSize = 12.sp,
                                                fontWeight = FontWeight.Bold,
                                                modifier = Modifier.padding(horizontal = 10.dp, vertical = 2.dp),
                                            )
                                        }
                                    }
                                }
                                Text(
                                    text = "${pin.statusLabel} ${pin.rssi} dBm",
                                    color = palette.textMuted,
                                    fontSize = 12.sp,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                )
                            }
                            IconChevronRight(modifier = Modifier.size(20.dp), tint = palette.textMuted)
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
