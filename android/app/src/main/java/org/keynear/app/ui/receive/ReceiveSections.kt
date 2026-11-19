package org.keynear.app.ui.receive

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.AnimationVector1D
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import org.keynear.app.ui.model.DeviceRoleUi
import org.keynear.app.ui.model.VisibilityState
import org.keynear.app.ui.model.orbitFor
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.app.ui.navigation.OrbitPinUi
import org.keynear.designsystem.IconChevronRight
import org.keynear.designsystem.IconEyeOff
import org.keynear.designsystem.IconLaptop
import org.keynear.designsystem.IconRadio
import org.keynear.designsystem.IconShieldCheck
import org.keynear.designsystem.IconSmartphone
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.designsystem.tactileCushion
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.roundToInt
import kotlin.math.sin

private val BadgeHorizontalPadding = 2.dp
private val ChevronSize = 20.dp
private val CompactGap = 6.dp
private val ContentGap = 10.dp
private val DeviceRowMinHeight = 62.dp
private val EchoStrokeWidth = 4.5.dp
private val EmptyIconSize = 22.dp
private val EmptySheetExpandedHeight = 156.dp
private val OrbitCycleMs = 40_000
private val PinIconSize = 28.dp
private val PinLabelTextSize = 13.sp
private val PinLabelVerticalPadding = 7.dp
private val PinNodeSize = 64.dp
private val RowHorizontalPadding = 14.dp
private val RowIconSize = 44.dp
private val SelfIconSize = 36.dp
private val SelfNodeRadius = 41.dp
private val SelfNodeSize = 82.dp
private val SheetCollapsedHeight = 132.dp
private val SheetExpandedHeight = 252.dp
private val SpokeStrokeWidth = 1.dp
private val VisibilityControlHeight = 56.dp
private val VisibilityIconSize = 18.dp
private val WaveCycleMs = 120_000
private val WaveMoteAlpha = 0.5f

private object WaveMap {
    val InnerRadius = 78.dp
    val MiddleRadius = 136.dp
    val OuterRadius = 196.dp
    val GuideRadii = listOf(InnerRadius, MiddleRadius, OuterRadius)
    const val Spokes = 12
    const val RingPoints = 96
    const val BaseLobes = 6
    const val LobeStep = 2
    const val HiddenAmplitude = 0.5f
    const val BaseAmplitude = 2.2f
    const val AmplitudeStep = 0.9f
    const val SpokeRate = 0.00015f
    const val WaveRate = 0.0028f
    const val SecondaryWaveFactor = 0.7f
    const val EchoAmplitudeFactor = 1.15f
    const val EchoPhaseFactor = 1.1f
    const val SecondaryHarmonic = 3f
    const val SecondaryAmplitudeFactor = 0.5f
    const val MoteRate = 0.0004f
    const val MoteWaveRate = 0.002f
    const val MoteCount = 8
    val MoteDrift = 6.dp
    val MoteRadius = 1.5.dp
}

@Composable
internal fun VisibilitySegmentedControl(
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
    val wellColor = palette.visibilityWell
    val density = LocalDensity.current

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
            .padding(CompactGap),
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
                    .height(VisibilityControlHeight),
                shape = KeynearShapes.Xl,
                containerColor = palette.loading,
                tone = TactileTone.ACCENT,
                cornerRadius = KeynearRadii.Xl,
            ) {
                Box(modifier = Modifier.width(slotWidth).height(VisibilityControlHeight))
            }

            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .pointerInput(current, onSelect) {
                        var dragAccum = 0f
                        var lastIndex = selectedIndex
                        val slotPx = with(density) { slotWidth.toPx() }
                        detectHorizontalDragGestures(
                            onDragStart = {
                                dragAccum = 0f
                                lastIndex = selectedIndex
                            },
                            onHorizontalDrag = { change, dragAmount ->
                                change.consume()
                                dragAccum += dragAmount
                                val next = (selectedIndex + dragAccum / slotPx)
                                    .roundToInt()
                                    .coerceIn(items.indices)
                                if (next != lastIndex) {
                                    lastIndex = next
                                    onSelect(items[next].first)
                                }
                            },
                            onDragEnd = {},
                            onDragCancel = {},
                        )
                    },
            ) {
                Row(modifier = Modifier.fillMaxWidth()) {
                    items.forEach { (mode, label, kind) ->
                        val selected = current == mode
                        val fg by animateColorAsState(
                            targetValue = if (selected) palette.onAccent else palette.textSecondary,
                            label = "visibilityFg",
                        )
                        Row(
                            modifier = Modifier
                                .weight(1f)
                                .heightIn(min = VisibilityControlHeight)
                                .clip(KeynearShapes.Xl)
                                .clickable { onSelect(mode) }
                                .padding(horizontal = ContentGap),
                            horizontalArrangement = Arrangement.Center,
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            when (kind) {
                                0 -> IconEyeOff(modifier = Modifier.size(VisibilityIconSize), tint = fg)
                                1 -> IconShieldCheck(modifier = Modifier.size(VisibilityIconSize), tint = fg)
                                else -> IconRadio(modifier = Modifier.size(VisibilityIconSize), tint = fg)
                            }
                            Spacer(modifier = Modifier.width(KeynearSpacing.Sm))
                            Text(
                                text = label,
                                color = fg,
                                fontSize = KeynearTypeSizes.Description,
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
}

@Composable
internal fun RadarCanvasLayer(radarActive: Boolean) {
    val palette = LocalKeynearPalette.current
    val cycleMs = WaveCycleMs
    val transition = rememberInfiniteTransition(label = "waveMesh")
    val timestamp by transition.animateFloat(
        initialValue = 0f,
        targetValue = cycleMs.toFloat(),
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = cycleMs, easing = LinearEasing),
            repeatMode = RepeatMode.Restart,
        ),
        label = "waveTime",
    )

    Canvas(modifier = Modifier.fillMaxSize()) {
        val cx = size.width / 2f
        val cy = size.height / 2f
        val innerHugRadius = WaveMap.InnerRadius.toPx()
        val maxRadius = WaveMap.OuterRadius.toPx()
        val guideRadii = WaveMap.GuideRadii.map { it.toPx() }
        val speedMult = 1f

        for (s in 0 until WaveMap.Spokes) {
            val a = (s / WaveMap.Spokes.toFloat()) * 2f * PI.toFloat() +
                timestamp * WaveMap.SpokeRate
            drawLine(
                color = palette.waveSpoke,
                start = Offset(cx + cos(a) * innerHugRadius, cy + sin(a) * innerHugRadius),
                end = Offset(cx + cos(a) * maxRadius, cy + sin(a) * maxRadius),
                strokeWidth = SpokeStrokeWidth.toPx(),
            )
        }

        guideRadii.forEachIndexed { idx, baseR ->
            val points = WaveMap.RingPoints
            val lobeCount = WaveMap.BaseLobes + idx * WaveMap.LobeStep
            val amp = if (radarActive) {
                WaveMap.BaseAmplitude + idx * WaveMap.AmplitudeStep
            } else {
                WaveMap.HiddenAmplitude
            }
            val ampPx = amp.dp.toPx()
            val phase = timestamp * WaveMap.WaveRate * speedMult *
                (if (idx % 2 == 0) 1f else -1f) + idx

            val ringPath = Path()
            for (p in 0..points) {
                val theta = (p / points.toFloat()) * 2f * PI.toFloat()
                val wave = sin(theta * lobeCount.toFloat() + phase) * ampPx +
                    cos(theta * WaveMap.SecondaryHarmonic - phase * WaveMap.SecondaryWaveFactor) *
                    (ampPx * WaveMap.SecondaryAmplitudeFactor)
                val r = baseR + wave
                val x = cx + cos(theta) * r
                val y = cy + sin(theta) * r
                if (p == 0) ringPath.moveTo(x, y) else ringPath.lineTo(x, y)
            }
            ringPath.close()
            drawPath(ringPath, palette.waveRing, style = Stroke(width = BadgeHorizontalPadding.toPx()))

            if (radarActive) {
                val echoPath = Path()
                for (p in 0..points) {
                    val theta = (p / points.toFloat()) * 2f * PI.toFloat()
                    val wave = cos(
                        theta * lobeCount.toFloat() - phase * WaveMap.EchoPhaseFactor,
                    ) * (ampPx * WaveMap.EchoAmplitudeFactor)
                    val r = baseR + wave
                    val x = cx + cos(theta) * r
                    val y = cy + sin(theta) * r
                    if (p == 0) echoPath.moveTo(x, y) else echoPath.lineTo(x, y)
                }
                echoPath.close()
                drawPath(echoPath, palette.waveEcho, style = Stroke(width = EchoStrokeWidth.toPx()))
            }
        }

        guideRadii.forEachIndexed { idx, baseR ->
            for (s in 0 until WaveMap.MoteCount) {
                val theta = (s / WaveMap.MoteCount.toFloat()) * 2f * PI.toFloat() +
                    timestamp * WaveMap.MoteRate + idx
                val r = baseR + sin(timestamp * WaveMap.MoteWaveRate + idx + s) * WaveMap.MoteDrift.toPx()
                drawCircle(
                    color = palette.waveEcho.copy(alpha = WaveMoteAlpha),
                    radius = WaveMap.MoteRadius.toPx(),
                    center = Offset(cx + cos(theta) * r, cy + sin(theta) * r),
                )
            }
        }
    }
}

@Composable
internal fun CenterSelfNode(
    localDeviceName: String,
    visibility: VisibilityState,
    remainingSecs: Long,
) {
    val palette = LocalKeynearPalette.current
    val active = visibility != VisibilityState.HIDDEN
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
    ) {
        TactileSurface(
            modifier = Modifier.size(SelfNodeSize),
            shape = CircleShape,
            containerColor = if (active) palette.loading else palette.surface,
            tone = if (active) TactileTone.ACCENT else TactileTone.SURFACE,
            cornerRadius = SelfNodeRadius,
            isCircle = true,
        ) {
            Box(modifier = Modifier.size(SelfNodeSize), contentAlignment = Alignment.Center) {
                IconSmartphone(
                    modifier = Modifier.size(SelfIconSize),
                    tint = if (active) palette.onAccent else palette.textMuted,
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
                modifier = Modifier.padding(horizontal = VisibilityIconSize, vertical = KeynearSpacing.Sm),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
            ) {
                Text(
                    text = localDeviceName,
                    color = palette.textPrimary,
                    fontSize = KeynearTypeSizes.Description,
                    fontWeight = FontWeight.Bold,
                )
                if (visibility == VisibilityState.EVERYONE_NEARBY) {
                    val mins = remainingSecs.coerceAtLeast(0L) / 60L
                    val secs = remainingSecs.coerceAtLeast(0L) % 60L
                    Text(
                        text = "$mins:${secs.toString().padStart(2, '0')}",
                        color = palette.loading,
                        fontSize = KeynearTypeSizes.Description,
                        fontWeight = FontWeight.Bold,
                    )
                }
            }
        }
    }
}

@Composable
internal fun OrbitPinsLayer(
    pins: List<OrbitPinUi>,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current
    val transition = rememberInfiniteTransition(label = "orbit")
    val orbitProgress by transition.animateFloat(
        initialValue = 0f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = OrbitCycleMs, easing = LinearEasing),
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
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
        ) {
            TactileSurface(
                modifier = Modifier.size(PinNodeSize),
                shape = CircleShape,
                containerColor = palette.surface,
                tone = TactileTone.SURFACE,
                cornerRadius = KeynearSpacing.TwoXl,
                isCircle = true,
            ) {
                Box(modifier = Modifier.size(PinNodeSize), contentAlignment = Alignment.Center) {
                    if (pin.role == DeviceRoleUi.DESKTOP) {
                        IconLaptop(modifier = Modifier.size(PinIconSize), tint = palette.loading)
                    } else {
                        IconSmartphone(modifier = Modifier.size(PinIconSize), tint = palette.loading)
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
                    modifier = Modifier.padding(horizontal = KeynearSpacing.Lg, vertical = PinLabelVerticalPadding),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    Text(
                        text = pin.displayName,
                        color = palette.textPrimary,
                        fontSize = PinLabelTextSize,
                        fontWeight = FontWeight.Bold,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Text(
                        text = pin.statusLabel,
                        color = palette.textMuted,
                        fontSize = KeynearTypeSizes.Caption,
                    )
                    Text(
                        text = "${pin.rssi} dBm",
                        color = palette.textMuted,
                        fontSize = KeynearTypeSizes.Caption,
                    )
                }
            }
        }
    }
}

@Composable
internal fun DevicesBottomSheet(
    pins: List<OrbitPinUi>,
    expanded: Boolean,
    onSetExpanded: (Boolean) -> Unit,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current
    val density = LocalDensity.current
    val collapsedHeight = if (pins.isEmpty()) 0.dp else SheetCollapsedHeight
    val expandedHeight = if (pins.isEmpty()) EmptySheetExpandedHeight else SheetExpandedHeight
    val targetHeight = if (expanded) expandedHeight else collapsedHeight
    val height = remember {
        Animatable<Dp, AnimationVector1D>(targetHeight, Dp.VectorConverter)
    }
    val scope = rememberCoroutineScope()
    val currentHeight = height.value

    LaunchedEffect(targetHeight) {
        height.animateTo(targetHeight, spring(stiffness = Spring.StiffnessMediumLow))
    }

    val dragModifier = Modifier.pointerInput(collapsedHeight, expandedHeight) {
        var dragHeight = 0.dp
        var snapJob: Job? = null
        detectVerticalDragGestures(
            onDragStart = { dragHeight = height.value },
            onVerticalDrag = { change, dragAmount ->
                change.consume()
                dragHeight = (dragHeight + with(density) { (-dragAmount).toDp() })
                    .coerceIn(0.dp, expandedHeight)
                snapJob?.cancel()
                snapJob = scope.launch { height.snapTo(dragHeight) }
            },
            onDragEnd = {
                snapJob?.cancel()
                val midpoint = (collapsedHeight + expandedHeight) / 2f
                val shouldExpand = dragHeight >= midpoint
                onSetExpanded(shouldExpand)
                scope.launch {
                    height.animateTo(
                        targetValue = if (shouldExpand) expandedHeight else collapsedHeight,
                        animationSpec = spring(stiffness = Spring.StiffnessMediumLow),
                    )
                }
            },
            onDragCancel = {
                snapJob?.cancel()
                scope.launch {
                    height.animateTo(targetHeight, spring(stiffness = Spring.StiffnessMediumLow))
                }
            },
        )
    }

    TactileSurface(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = KeynearSpacing.Lg),
        shape = KeynearShapes.Sheet,
        containerColor = palette.surface,
        tone = TactileTone.SHEET,
        cornerRadius = KeynearRadii.ThreeXl,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(start = KeynearSpacing.Lg, end = KeynearSpacing.Lg, top = ContentGap, bottom = KeynearSpacing.Md),
            verticalArrangement = Arrangement.spacedBy(CompactGap),
        ) {
            SheetHeaderBar(count = pins.size, dragModifier = dragModifier)
            if (currentHeight > 0.dp) {
                if (pins.isEmpty()) {
                    EmptyDevicesSheetContent(currentHeight = currentHeight, dragModifier = dragModifier)
                } else {
                    LazyColumn(
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(currentHeight),
                        verticalArrangement = Arrangement.spacedBy(CompactGap),
                        contentPadding = PaddingValues(bottom = KeynearSpacing.Xs),
                    ) {
                        items(pins, key = { it.id }) { pin ->
                            DeviceSheetRow(pin = pin, actions = actions)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun SheetHeaderBar(
    count: Int,
    dragModifier: Modifier,
) {
    val palette = LocalKeynearPalette.current
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = RowIconSize)
            .padding(bottom = CompactGap)
            .then(dragModifier),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(
            modifier = Modifier
                .width(KeynearSpacing.MinTouchTarget)
                .height(CompactGap)
                .clip(CircleShape)
                .background(palette.sheetHandle),
        )
        Spacer(modifier = Modifier.height(ContentGap))
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.Start,
        ) {
            Text(
                text = "$count computer(s) in range",
                color = palette.textPrimary,
                fontSize = KeynearTypeSizes.Description,
                fontWeight = FontWeight.Bold,
            )
        }
    }
}

@Composable
private fun EmptyDevicesSheetContent(
    currentHeight: Dp,
    dragModifier: Modifier,
) {
    val palette = LocalKeynearPalette.current
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .height(currentHeight)
            .then(dragModifier)
            .clip(KeynearShapes.Xl)
            .background(palette.trackWell)
            .tactileCushion(
                tone = TactileTone.INSET,
                isDark = palette.isDark,
                cornerRadius = KeynearRadii.Xl,
            )
            .padding(KeynearSpacing.Lg),
        contentAlignment = Alignment.Center,
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
        ) {
            IconRadio(modifier = Modifier.size(EmptyIconSize), tint = palette.textMuted)
            Text(
                text = "No computers in range",
                color = palette.textMuted,
                fontSize = KeynearTypeSizes.Description,
                fontWeight = FontWeight.SemiBold,
            )
        }
    }
}

@Composable
private fun DeviceSheetRow(
    pin: OrbitPinUi,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = DeviceRowMinHeight)
            .clip(KeynearShapes.Xl)
            .clickable {
                if (pin.isPaired) {
                    actions.onOpenDeviceDetail(pin.id)
                } else {
                    actions.onInitiatePairing(pin.peerAddress)
                }
            }
            .padding(horizontal = RowHorizontalPadding, vertical = ContentGap),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(RowHorizontalPadding),
    ) {
        TactileSurface(
            modifier = Modifier.size(RowIconSize),
            shape = KeynearShapes.Xl,
            containerColor = palette.loadingTonal,
            tone = TactileTone.TONAL,
            cornerRadius = KeynearRadii.Xl,
        ) {
            Box(modifier = Modifier.size(RowIconSize), contentAlignment = Alignment.Center) {
                if (pin.role == DeviceRoleUi.DESKTOP) {
                    IconLaptop(modifier = Modifier.size(ChevronSize), tint = palette.onLoadingTonal)
                } else {
                    IconSmartphone(modifier = Modifier.size(ChevronSize), tint = palette.onLoadingTonal)
                }
            }
        }
        Column(modifier = Modifier.weight(1f)) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
            ) {
                Text(
                    text = pin.displayName,
                    color = palette.textPrimary,
                    fontSize = KeynearTypeSizes.Body,
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
                            fontSize = KeynearTypeSizes.Caption,
                            fontWeight = FontWeight.Bold,
                            modifier = Modifier.padding(horizontal = ContentGap, vertical = BadgeHorizontalPadding),
                        )
                    }
                }
            }
            Text(
                text = "${pin.statusLabel} ${pin.rssi} dBm",
                color = palette.textMuted,
                fontSize = KeynearTypeSizes.Caption,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
        IconChevronRight(modifier = Modifier.size(ChevronSize), tint = palette.textMuted)
    }
}

