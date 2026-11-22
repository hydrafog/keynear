package org.keynear.app.ui.receive

import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.ui.model.DeviceRoleUi
import org.keynear.app.ui.model.orbitFor
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.app.ui.navigation.OrbitPinUi
import org.keynear.designsystem.IconLaptop
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
import kotlin.math.sin

private val BadgeHorizontalPadding = 2.dp
private val ChevronSize = 20.dp
private val CompactGap = 6.dp
private val ContentGap = 10.dp
private val DeviceRowMinHeight = 62.dp
private val EchoStrokeWidth = 4.5.dp
private val EmptyIconSize = 22.dp
private val EmptySheetExpandedHeight = 156.dp
private const val ORBIT_CYCLE_MS = 40_000
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
private const val WAVE_CYCLE_MS = 120_000
private const val WAVE_MOTE_ALPHA = 0.5f
private const val SKELETON_DRIFT_HALF_MS = 4_000
private const val SKELETON_SCALE_MIN = 0.97f
private const val SKELETON_SCALE_MAX = 1.03f
private val SkeletonOffsets = listOf(
    (-96.dp to (-56.dp)),
    (112.dp to 28.dp),
    (-52.dp to 92.dp),
)
private val SkeletonLabelWidth = 56.dp
private val SkeletonSubLabelWidth = 36.dp
private val SkeletonBarHeight = 6.dp


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
            animation = tween(durationMillis = ORBIT_CYCLE_MS, easing = LinearEasing),
            repeatMode = RepeatMode.Restart,
        ),
        label = "orbitPhase",
    )

    if (pins.isEmpty()) {
        SkeletonPins()
    }

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
                containerColor = palette.surface.copy(alpha = 0.94f),
                tone = TactileTone.SURFACE,
                cornerRadius = KeynearSpacing.TwoXl,
                isCircle = true,
            ) {
                Box(modifier = Modifier.size(PinNodeSize), contentAlignment = Alignment.Center) {
                    if (pin.role == DeviceRoleUi.DESKTOP) {
                        IconLaptop(modifier = Modifier.size(PinIconSize), tint = palette.loadingInk)
                    } else {
                        IconSmartphone(modifier = Modifier.size(PinIconSize), tint = palette.loadingInk)
                    }
                }
            }

            TactileSurface(
                shape = KeynearShapes.Xl,
                containerColor = palette.surface.copy(alpha = 0.94f),
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
private fun SkeletonPins() {
    val palette = LocalKeynearPalette.current
    val transition = rememberInfiniteTransition(label = "skeletonDrift")
    val driftSpec = infiniteRepeatable<Float>(
        animation = tween(durationMillis = SKELETON_DRIFT_HALF_MS, easing = FastOutSlowInEasing),
        repeatMode = RepeatMode.Reverse,
    )
    val pulseFirst = transition.animateFloat(0.45f, 0.85f, driftSpec, label = "skeletonDrift0")
    val pulseSecond = transition.animateFloat(0.85f, 0.45f, driftSpec, label = "skeletonDrift1")
    val pulseThird = transition.animateFloat(0.45f, 0.85f, driftSpec, label = "skeletonDrift2")
    val pulses = listOf(pulseFirst, pulseSecond, pulseThird)

    SkeletonOffsets.forEachIndexed { idx, (xOffset, yOffset) ->
        val pulse = pulses[idx].value
        val scale = SKELETON_SCALE_MIN + (pulse - 0.45f) / 0.40f * (SKELETON_SCALE_MAX - SKELETON_SCALE_MIN)
        Column(
            modifier = Modifier
                .offset(x = xOffset, y = yOffset)
                .graphicsLayer {
                    scaleX = scale
                    scaleY = scale
                },
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
        ) {
            Box(
                modifier = Modifier
                    .size(PinNodeSize)
                    .clip(CircleShape)
                    .background(palette.surfaceTonal.copy(alpha = pulse))
                    .tactileCushion(
                        tone = TactileTone.INSET,
                        isDark = palette.isDark,
                        cornerRadius = KeynearSpacing.TwoXl,
                        isCircle = true,
                    ),
                contentAlignment = Alignment.Center,
            ) {
                IconLaptop(
                    modifier = Modifier.size(PinIconSize),
                    tint = palette.textMuted.copy(alpha = 0.55f * pulse),
                )
            }
            Column(
                modifier = Modifier
                    .clip(KeynearShapes.Xl)
                    .background(palette.surfaceTonal.copy(alpha = pulse))
                    .padding(horizontal = KeynearSpacing.Lg, vertical = PinLabelVerticalPadding),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Xs),
            ) {
                Box(
                    modifier = Modifier
                        .size(width = SkeletonLabelWidth, height = SkeletonBarHeight)
                        .clip(CircleShape)
                        .background(palette.textMuted.copy(alpha = 0.32f * pulse)),
                )
                Box(
                    modifier = Modifier
                        .size(width = SkeletonSubLabelWidth, height = SkeletonBarHeight)
                        .clip(CircleShape)
                        .background(palette.textMuted.copy(alpha = 0.22f * pulse)),
                )
            }
        }
    }
}
