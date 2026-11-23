package org.keynear.designsystem

import android.os.SystemClock
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.State
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.input.pointer.PointerInputScope
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.progressBarRangeInfo
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.toggleableState
import androidx.compose.ui.state.ToggleableState
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.IntOffset
import kotlin.math.abs
import kotlin.math.roundToInt

private suspend fun PointerInputScope.handleSwitchDrag(
    currentChecked: State<Boolean>,
    onFractionChange: (Float?) -> Unit,
    onDragFinished: (moved: Boolean, nextChecked: Boolean) -> Unit,
) {
    var dragOffset = 0f
    var moved = false
    var startFraction = 0f
    var latestFraction = 0f
    val travelPx = KeynearLayout.SwitchThumbTravel.toPx()
    val dragSlopPx = KeynearLayout.SwitchDragSlop.toPx()
    detectHorizontalDragGestures(
        onDragStart = {
            dragOffset = 0f
            moved = false
            startFraction = if (currentChecked.value) 1f else 0f
            latestFraction = startFraction
        },
        onHorizontalDrag = { change, dragAmount ->
            change.consume()
            dragOffset += dragAmount
            if (!moved && abs(dragOffset) >= dragSlopPx) {
                moved = true
            }
            if (moved) {
                latestFraction = (startFraction + dragOffset / travelPx).coerceIn(0f, 1f)
                onFractionChange(latestFraction)
            }
        },
        onDragEnd = {
            onDragFinished(moved, latestFraction >= KeynearFractions.HALF)
            onFractionChange(null)
            dragOffset = 0f
            moved = false
        },
        onDragCancel = {
            onFractionChange(null)
            dragOffset = 0f
            moved = false
        },
    )
}

@Composable
fun KeynearSwitch(
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    val currentChecked = rememberUpdatedState(checked)
    val latestOnCheckedChange by rememberUpdatedState(onCheckedChange)
    var dragFraction by remember { mutableStateOf<Float?>(null) }
    var lastDragEndUptimeMs by remember { mutableLongStateOf(0L) }
    val animatedFraction by animateFloatAsState(
        targetValue = if (checked) 1f else 0f,
        animationSpec = tween(durationMillis = KeynearMotion.SWITCH_DURATION_MS, easing = KeynearMotion.SpringEasing),
        label = "switchFraction",
    )
    val effectiveFraction = dragFraction ?: animatedFraction
    val visualChecked = dragFraction?.let { it >= KeynearFractions.HALF } ?: checked
    val trackColor by animateColorAsState(
        targetValue = if (visualChecked) palette.loading else palette.switchTrackOff,
        animationSpec = tween(durationMillis = KeynearMotion.SWITCH_DURATION_MS, easing = KeynearMotion.SpringEasing),
        label = "switchTrack",
    )
    val trackTone = if (visualChecked) TactileTone.ACCENT else TactileTone.INSET
    val trackRim = tactileSurfaceRimColor(trackTone, palette)

    Box(
        modifier = modifier
            .defaultMinSize(minWidth = KeynearLayout.SwitchMinWidth, minHeight = KeynearLayout.ControlMinHeight)
            .semantics { toggleableState = if (checked) ToggleableState.On else ToggleableState.Off }
            .pointerInput(Unit) {
                handleSwitchDrag(
                    currentChecked = currentChecked,
                    onFractionChange = { dragFraction = it },
                    onDragFinished = { moved, nextChecked ->
                        if (moved) {
                            lastDragEndUptimeMs = SystemClock.uptimeMillis()
                            if (nextChecked != currentChecked.value) latestOnCheckedChange(nextChecked)
                        }
                    },
                )
            }
            .clickable(role = Role.Switch) {
                val sinceDrag = SystemClock.uptimeMillis() - lastDragEndUptimeMs
                if (sinceDrag >= KeynearMotion.CLICK_SUPPRESS_MS) onCheckedChange(!checked)
            },
        contentAlignment = Alignment.CenterEnd,
    ) {
        Box(
            modifier = Modifier
                .width(KeynearLayout.SwitchTrackWidth)
                .height(KeynearLayout.SwitchTrackHeight)
                .tactileShadow(tone = trackTone, isDark = palette.isDark, cornerRadius = KeynearLayout.SwitchTrackRadius)
                .clip(CircleShape)
                .tactileSurfaceBackground(tone = trackTone, containerColor = trackColor, palette = palette)
                .let { m -> if (trackRim != null) m.border(KeynearDimensions.Dp1, trackRim, CircleShape) else m }
                .tactileCushion(tone = trackTone, isDark = palette.isDark, cornerRadius = KeynearLayout.SwitchTrackRadius)
                .padding(KeynearSpacing.Xs),
            contentAlignment = Alignment.CenterStart,
        ) {
            Box(
                modifier = Modifier
                    .offset { IntOffset((KeynearLayout.SwitchThumbTravel.toPx() * effectiveFraction).roundToInt(), 0) }
                    .size(KeynearLayout.SwitchThumbSize)
                    .tactileShadow(
                        tone = TactileTone.THUMB,
                        isDark = palette.isDark,
                        cornerRadius = KeynearLayout.SwitchThumbRadius,
                        isCircle = true,
                    )
                    .clip(CircleShape)
                    .tactileSurfaceBackground(tone = TactileTone.THUMB, containerColor = palette.switchThumb, palette = palette)
                    .tactileCushion(
                        tone = TactileTone.THUMB,
                        isDark = palette.isDark,
                        cornerRadius = KeynearLayout.SwitchThumbRadius,
                        isCircle = true,
                    ),
            )
        }
    }
}

@Composable
fun WarningBannerCard(
    message: String,
    ctaLabel: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    TactileSurface(
        modifier = modifier.fillMaxWidth(),
        shape = KeynearShapes.Card,
        containerColor = palette.warningTonal,
        tone = TactileTone.TONAL,
        cornerRadius = KeynearRadii.TwoXl,
        onClick = onClick,
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = KeynearSpacing.Lg, vertical = KeynearSpacing.Md),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(KeynearLayout.ContentGap),
        ) {
            Text(
                text = message,
                color = palette.onWarningTonal,
                fontSize = KeynearTypeSizes.Caption,
                fontWeight = FontWeight.Medium,
                modifier = Modifier.weight(1f),
            )
            Text(
                text = ctaLabel,
                color = palette.onWarningTonal,
                fontSize = KeynearTypeSizes.Caption,
                fontWeight = FontWeight.Bold,
            )
        }
    }
}

@Composable
fun KeynearProgressBar(
    progress: Float,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    val clamped = progress.coerceIn(0f, 1f)
    val animatedProgress by animateFloatAsState(
        targetValue = clamped,
        animationSpec = tween(
            durationMillis = KeynearMotion.MORPH_DURATION_MS,
            easing = KeynearMotion.SpringEasing,
        ),
        label = "progressFraction",
    )
    val accentRim = tactileSurfaceRimColor(TactileTone.ACCENT, palette)
    Box(
        modifier = modifier
            .fillMaxWidth()
            .height(KeynearLayout.ProgressBarHeight)
            .semantics { progressBarRangeInfo = ProgressBarRangeInfo(clamped, 0f..1f) }
            .clip(CircleShape)
            .background(palette.trackWell)
            .tactileCushion(
                tone = TactileTone.INSET,
                isDark = palette.isDark,
                cornerRadius = KeynearLayout.ProgressBarHeight,
            ),
    ) {
        if (animatedProgress > 0f) {
            Box(
                modifier = Modifier
                    .fillMaxWidth(fraction = animatedProgress)
                    .fillMaxHeight()
                    .clip(CircleShape)
                    .tactileSurfaceBackground(TactileTone.ACCENT, palette.loading, palette)
                    .let { m -> if (accentRim != null) m.border(KeynearDimensions.Dp1, accentRim, CircleShape) else m }
                    .tactileCushion(
                        tone = TactileTone.ACCENT,
                        isDark = palette.isDark,
                        cornerRadius = KeynearLayout.ProgressBarHeight,
                    ),
            )
        }
    }
}
