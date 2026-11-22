package org.keynear.designsystem

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp

@Composable
fun KeynearSwitch(
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    val trackColor by animateColorAsState(
        targetValue = if (checked) palette.loading else palette.switchTrackOff,
        animationSpec = tween(durationMillis = SWITCH_GLIDE_DURATION_MS, easing = KeynearSpringEasing),
        label = "switchTrack",
    )
    val thumbOffset by animateDpAsState(
        targetValue = if (checked) KeynearSpacing.Xl else 0.dp,
        animationSpec = tween(durationMillis = SWITCH_GLIDE_DURATION_MS, easing = KeynearSpringEasing),
        label = "switchThumb",
    )
    val trackTone = if (checked) TactileTone.ACCENT else TactileTone.INSET

    Box(
        modifier = modifier
            .defaultMinSize(minWidth = KeynearLayout.SwitchMinWidth, minHeight = KeynearLayout.ControlMinHeight)
            .pointerInput(checked, onCheckedChange) {
                var dragOffset = 0f
                detectHorizontalDragGestures(
                    onDragStart = { dragOffset = 0f },
                    onHorizontalDrag = { change, dragAmount ->
                        change.consume()
                        dragOffset += dragAmount
                        val threshold = KeynearLayout.SwitchDragThreshold.toPx()
                        when {
                            dragOffset > threshold && !checked -> {
                                onCheckedChange(true)
                                dragOffset = 0f
                            }
                            dragOffset < -threshold && checked -> {
                                onCheckedChange(false)
                                dragOffset = 0f
                            }
                        }
                    },
                    onDragEnd = { dragOffset = 0f },
                    onDragCancel = { dragOffset = 0f },
                )
            }
            .clickable(role = Role.Switch) { onCheckedChange(!checked) },
        contentAlignment = Alignment.CenterEnd,
    ) {
        Box(
            modifier = Modifier
                .width(KeynearLayout.SwitchTrackWidth)
                .height(KeynearLayout.SwitchTrackHeight)
                .tactileShadow(
                    tone = trackTone,
                    isDark = palette.isDark,
                    cornerRadius = KeynearLayout.SwitchTrackRadius,
                )
                .clip(CircleShape)
                .background(trackColor)
                .tactileCushion(tone = trackTone, isDark = palette.isDark, cornerRadius = KeynearLayout.SwitchTrackRadius)
                .padding(KeynearSpacing.Xs),
            contentAlignment = Alignment.CenterStart,
        ) {
            Box(
                modifier = Modifier
                    .offset(x = thumbOffset)
                    .size(KeynearLayout.SwitchThumbSize)
                    .tactileShadow(
                        tone = TactileTone.THUMB,
                        isDark = palette.isDark,
                        cornerRadius = KeynearLayout.SwitchThumbRadius,
                        isCircle = true,
                    )
                    .clip(CircleShape)
                    .background(palette.switchThumb)
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
    LinearProgressIndicator(
        progress = { progress.coerceIn(0f, 1f) },
        modifier = modifier
            .fillMaxWidth()
            .height(KeynearLayout.CompactGap)
            .clip(CircleShape),
        color = palette.loading,
        trackColor = palette.switchTrackOff,
    )
}

