package org.keynear.app.ui.receive

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.LocalIndication
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableFloatState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.disabled
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import org.keynear.app.ui.model.DeviceRoleUi
import org.keynear.app.ui.model.OrbitPlacement
import org.keynear.app.ui.model.RadarLayout
import org.keynear.app.ui.model.orbitPlacementsFor
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.app.ui.navigation.OrbitPinUi
import org.keynear.designsystem.IconLaptop
import org.keynear.designsystem.IconSmartphone
import org.keynear.designsystem.KeynearDimensions
import org.keynear.designsystem.KeynearMotion
import org.keynear.designsystem.KeynearPressScale
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.designsystem.tactileCushion
import org.keynear.designsystem.tactilePressScale
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin

private const val FOG_SURFACE_ALPHA = 0.84f
private const val MAX_FRAME_DELTA_MS = 64f
private const val NANOS_PER_MS = 1_000_000f
private const val PIN_ENTER_BASE_SCALE = 0.92f
private const val PIN_VISIBLE_BASE_SCALE = 0.75f
private const val SKELETON_DRIFT_HALF_MS = 4_000
private const val SKELETON_PULSE_MIN = 0.45f
private const val SKELETON_PULSE_MAX = 0.85f
private const val SKELETON_PULSE_IDLE = 0.55f
private const val SKELETON_SCALE_MIN = 0.97f
private const val SKELETON_SCALE_MAX = 1.03f
private const val SKELETON_ICON_ALPHA = 0.55f
private const val SKELETON_TITLE_ALPHA = 0.32f
private const val SKELETON_SUB_ALPHA = 0.22f
private val PinLabelMaxWidth = 164.dp
private val SkeletonBarHeight = 6.dp
private val SkeletonLabelWidth = 56.dp
private val SkeletonSubLabelWidth = 36.dp
private val SkeletonIds = listOf("skeleton-inner", "skeleton-middle", "skeleton-outer")

private fun nextOrbitRate(
    startRate: Float,
    targetRate: Float,
    elapsedMs: Float,
): Float {
    if (elapsedMs >= KeynearMotion.RADAR_RAMP_MS) return targetRate
    val inv = 1f - elapsedMs / KeynearMotion.RADAR_RAMP_MS.toFloat()
    return startRate + (targetRate - startRate) * (1f - inv * inv * inv)
}

@Composable
internal fun OrbitPinsLayer(
    pins: List<OrbitPinUi>,
    radarActive: Boolean,
    layout: RadarLayout,
    actions: KeynearActions,
    orbitPhase: MutableFloatState,
) {
    var orbitRate by remember { mutableFloatStateOf(if (radarActive) 1f else 0f) }
    val currentActive by rememberUpdatedState(radarActive)

    LaunchedEffect(radarActive) {
        val startRate = orbitRate
        val targetRate = if (currentActive) 1f else 0f
        var elapsedMs = 0f
        var lastNanos = 0L
        while (currentActive || orbitRate > 0f) {
            withFrameNanos { frameNanos ->
                if (lastNanos == 0L) lastNanos = frameNanos
                val deltaMs = ((frameNanos - lastNanos) / NANOS_PER_MS).coerceIn(0f, MAX_FRAME_DELTA_MS)
                lastNanos = frameNanos
                elapsedMs = minOf(KeynearMotion.RADAR_RAMP_MS.toFloat(), elapsedMs + deltaMs)
                orbitRate = nextOrbitRate(startRate, targetRate, elapsedMs)
                orbitPhase.value = (orbitPhase.value + (deltaMs / KeynearMotion.ORBIT_CYCLE_MS.toFloat()) * orbitRate) % 1f
            }
        }
    }

    if (pins.isEmpty()) {
        SkeletonPins(radarActive = radarActive, layout = layout, orbitPhase = orbitPhase.value)
        return
    }
    val placements = orbitPlacementsFor(pins.map { it.id }, layout)
    pins.forEachIndexed { idx, pin ->
        OrbitPeerPin(pin, idx, placements[idx], orbitPhase.value, actions)
    }
}

private fun orbitCoordinatesPx(
    orbit: OrbitPlacement,
    orbitPhase: Float,
    densityPxPerDp: Float,
): Pair<Float, Float> {
    val angle = (((orbit.phaseSec / orbit.durationSec.toFloat()) + orbitPhase) * 2f * PI.toFloat()) - (PI.toFloat() / 2f)
    val radiusPx = orbit.radiusPx.toFloat() * densityPxPerDp
    return (cos(angle) * radiusPx) to (sin(angle) * radiusPx)
}

@Composable
private fun OrbitPeerPin(
    pin: OrbitPinUi,
    index: Int,
    orbit: OrbitPlacement,
    orbitPhase: Float,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current
    val density = LocalDensity.current
    val enter = remember(pin.id) { Animatable(0f) }
    LaunchedEffect(pin.id) {
        delay((index * KeynearMotion.PIN_STAGGER_MS).toLong())
        enter.animateTo(1f, tween(KeynearMotion.PIN_ENTER_DURATION_MS, easing = KeynearMotion.SpringEasing))
    }
    val visible by animateFloatAsState(
        targetValue = if (pin.isVisible) 1f else 0f,
        animationSpec = tween(KeynearMotion.DURATION_500_MS, easing = KeynearMotion.SpringEasing),
        label = "pinVisibility",
    )
    val interactionSource = remember { MutableInteractionSource() }
    val (xPx, yPx) = orbitCoordinatesPx(orbit, orbitPhase, density.density)
    val enterSlidePx = with(density) { KeynearDimensions.Dp4.toPx() }

    Column(
        modifier = Modifier
            .graphicsLayer {
                val e = enter.value
                val scale = (PIN_ENTER_BASE_SCALE + (1f - PIN_ENTER_BASE_SCALE) * e) *
                    (PIN_VISIBLE_BASE_SCALE + (1f - PIN_VISIBLE_BASE_SCALE) * visible)
                translationX = xPx
                translationY = yPx + enterSlidePx * (1f - e)
                scaleX = scale
                scaleY = scale
                alpha = e * visible
            }
            .tactilePressScale(interactionSource, KeynearPressScale.PRESS_95)
            .semantics { if (!pin.isVisible) disabled() }
            .clickable(enabled = pin.isVisible, interactionSource = interactionSource, indication = LocalIndication.current) {
                if (pin.isPaired) actions.onOpenDeviceDetail(pin.id) else actions.onInitiatePairing(pin.peerAddress)
            },
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
    ) {
        TactileSurface(
            modifier = Modifier.size(KeynearDimensions.Dp58),
            shape = CircleShape,
            containerColor = palette.surface.copy(alpha = FOG_SURFACE_ALPHA),
            tone = TactileTone.SURFACE,
            cornerRadius = KeynearDimensions.Dp58 / 2,
            isCircle = true,
            contentAlignment = Alignment.Center,
        ) {
            if (pin.role == DeviceRoleUi.DESKTOP) {
                IconLaptop(modifier = Modifier.size(KeynearDimensions.Dp26), tint = palette.loadingInk)
            } else {
                IconSmartphone(modifier = Modifier.size(KeynearDimensions.Dp26), tint = palette.loadingInk)
            }
        }
        TactileSurface(
            modifier = Modifier.widthIn(max = PinLabelMaxWidth),
            shape = KeynearShapes.Xl,
            containerColor = palette.surface.copy(alpha = FOG_SURFACE_ALPHA),
            tone = TactileTone.SURFACE,
            cornerRadius = KeynearRadii.Xl,
        ) {
            Column(
                modifier = Modifier.padding(horizontal = KeynearDimensions.Dp14, vertical = KeynearDimensions.Dp6),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text(pin.displayName, color = palette.textPrimary, fontSize = KeynearTypeSizes.Caption, fontWeight = FontWeight.Bold,
                    maxLines = 1, overflow = TextOverflow.Ellipsis)
                Text("${pin.statusLabel} · ${pin.rssi} dBm", color = palette.textMuted, fontSize = KeynearTypeSizes.Caption,
                    maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
    }
}

@Composable
private fun SkeletonPins(radarActive: Boolean, layout: RadarLayout, orbitPhase: Float) {
    val palette = LocalKeynearPalette.current
    val pulses = if (radarActive) {
        val transition = rememberInfiniteTransition(label = "skeletonDrift")
        val spec = infiniteRepeatable<Float>(
            animation = tween(durationMillis = SKELETON_DRIFT_HALF_MS, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse,
        )
        listOf(
            transition.animateFloat(SKELETON_PULSE_MIN, SKELETON_PULSE_MAX, spec, label = "s0").value,
            transition.animateFloat(SKELETON_PULSE_MAX, SKELETON_PULSE_MIN, spec, label = "s1").value,
            transition.animateFloat(SKELETON_PULSE_MIN, SKELETON_PULSE_MAX, spec, label = "s2").value,
        )
    } else {
        listOf(SKELETON_PULSE_IDLE, SKELETON_PULSE_IDLE, SKELETON_PULSE_IDLE)
    }
    val density = LocalDensity.current.density
    val placements = orbitPlacementsFor(SkeletonIds, layout)
    SkeletonIds.forEachIndexed { idx, _ ->
        val pulse = pulses[idx]
        val (xPx, yPx) = orbitCoordinatesPx(placements[idx], orbitPhase, density)
        val scale = SKELETON_SCALE_MIN +
            ((pulse - SKELETON_PULSE_MIN) / (SKELETON_PULSE_MAX - SKELETON_PULSE_MIN)) * (SKELETON_SCALE_MAX - SKELETON_SCALE_MIN)
        Column(
            modifier = Modifier.graphicsLayer { translationX = xPx; translationY = yPx; scaleX = scale; scaleY = scale },
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
        ) {
            Box(
                modifier = Modifier
                    .size(KeynearDimensions.Dp58)
                    .clip(CircleShape)
                    .background(palette.surfaceTonal.copy(alpha = pulse))
                    .tactileCushion(TactileTone.INSET, palette.isDark, KeynearDimensions.Dp58 / 2, isCircle = true),
                contentAlignment = Alignment.Center,
            ) {
                IconLaptop(
                    modifier = Modifier.size(KeynearDimensions.Dp26),
                    tint = palette.textMuted.copy(alpha = SKELETON_ICON_ALPHA * pulse),
                )
            }
            Column(
                modifier = Modifier
                    .clip(KeynearShapes.Xl)
                    .background(palette.surfaceTonal.copy(alpha = pulse))
                    .padding(horizontal = KeynearDimensions.Dp14, vertical = KeynearDimensions.Dp6),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Xs),
            ) {
                Box(Modifier.size(SkeletonLabelWidth, SkeletonBarHeight).clip(CircleShape)
                    .background(palette.textMuted.copy(alpha = SKELETON_TITLE_ALPHA * pulse)))
                Box(Modifier.size(SkeletonSubLabelWidth, SkeletonBarHeight).clip(CircleShape)
                    .background(palette.textMuted.copy(alpha = SKELETON_SUB_ALPHA * pulse)))
            }
        }
    }
}
