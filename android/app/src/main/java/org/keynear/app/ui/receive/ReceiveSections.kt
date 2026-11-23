package org.keynear.app.ui.receive

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandHorizontally
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkHorizontally
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
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
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerInputScope
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import org.keynear.app.ui.model.VisibilityState
import org.keynear.designsystem.IconEyeOff
import org.keynear.designsystem.IconRadio
import org.keynear.designsystem.IconShieldCheck
import org.keynear.designsystem.IconSmartphone
import org.keynear.designsystem.KeynearDimensions
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearMotion
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.designsystem.tactileCushion
import kotlin.math.abs
import kotlin.math.roundToInt

private const val FOG_SURFACE_ALPHA = 0.84f
private const val SECONDS_PER_MINUTE = 60L
private val SelfIconSize = 28.dp
private val SelfNodeRadius = 32.dp
private val SelfNodeSize = 64.dp
private val VisibilityOptions = listOf(
    VisibilityState.HIDDEN to "Hidden",
    VisibilityState.PAIRED_ONLY to "Paired",
    VisibilityState.EVERYONE_NEARBY to "Everyone",
)

private data class VisibilityDragConfig(
    val slotPx: Float,
    val dragSlopPx: Float,
)

private suspend fun PointerInputScope.handleVisibilityDrag(
    config: VisibilityDragConfig,
    currentIndex: State<Int>,
    onDragChange: (Float?) -> Unit,
    onSelect: (VisibilityState) -> Unit,
) {
    var dragAccum = 0f
    var moved = false
    var baseIndex = 0f
    var lastSnapped = 0
    detectHorizontalDragGestures(
        onDragStart = {
            dragAccum = 0f
            moved = false
            baseIndex = currentIndex.value.toFloat()
            lastSnapped = currentIndex.value
        },
        onHorizontalDrag = { change, dragAmount ->
            change.consume()
            dragAccum += dragAmount
            if (!moved && abs(dragAccum) >= config.dragSlopPx) moved = true
            if (moved && config.slotPx > 0f) {
                val raw = (baseIndex + dragAccum / config.slotPx).coerceIn(0f, (VisibilityOptions.size - 1).toFloat())
                onDragChange(raw)
                val snapped = raw.roundToInt()
                if (snapped != lastSnapped) {
                    lastSnapped = snapped
                    onSelect(VisibilityOptions[snapped].first)
                }
            }
        },
        onDragEnd = { onDragChange(null) },
        onDragCancel = { onDragChange(null) },
    )
}

@Composable
internal fun VisibilitySegmentedControl(
    current: VisibilityState,
    onSelect: (VisibilityState) -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val selectedIndex = VisibilityOptions.indexOfFirst { it.first == current }.coerceAtLeast(0)
    val currentIndex = rememberUpdatedState(selectedIndex)
    val latestOnSelect by rememberUpdatedState(onSelect)
    var dragPillIdx by remember { mutableStateOf<Float?>(null) }
    val density = LocalDensity.current

    Box(
        modifier = Modifier
            .fillMaxWidth()
            .clip(KeynearShapes.Card)
            .background(palette.trackWell)
            .tactileCushion(TactileTone.INSET, palette.isDark, KeynearRadii.TwoXl)
            .padding(KeynearLayout.CompactGap),
    ) {
        BoxWithConstraints(modifier = Modifier.fillMaxWidth()) {
            val slotWidth = maxWidth / VisibilityOptions.size.toFloat()
            val slotPx = with(density) { slotWidth.toPx() }
            val dragSlopPx = with(density) { KeynearLayout.SwitchDragSlop.toPx() }
            val dragConfig = VisibilityDragConfig(slotPx, dragSlopPx)
            val animatedIdx by animateFloatAsState(
                targetValue = selectedIndex.toFloat(),
                animationSpec = tween(KeynearMotion.MORPH_DURATION_MS, easing = KeynearMotion.SpringEasing),
                label = "visibilityPill",
            )
            val effectiveIdx = dragPillIdx ?: animatedIdx
            TactileSurface(
                modifier = Modifier
                    .offset { IntOffset((slotPx * effectiveIdx).roundToInt(), 0) }
                    .width(slotWidth)
                    .height(KeynearSpacing.MinTouchTarget),
                shape = KeynearShapes.Xl,
                containerColor = palette.loading,
                tone = TactileTone.ACCENT,
                cornerRadius = KeynearRadii.Xl,
            ) { Box(Modifier.width(slotWidth).height(KeynearSpacing.MinTouchTarget)) }

            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .pointerInput(dragConfig) {
                        handleVisibilityDrag(dragConfig, currentIndex, { dragPillIdx = it }, latestOnSelect)
                    },
            ) {
                VisibilityOptions.forEachIndexed { idx, (mode, label) ->
                    VisibilityOption(current == mode, label, idx, { onSelect(mode) }, Modifier.weight(1f))
                }
            }
        }
    }
}

@Composable
private fun VisibilityOption(
    selected: Boolean,
    label: String,
    kind: Int,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    val fg by animateColorAsState(
        targetValue = if (selected) palette.onAccent else palette.textSecondary,
        animationSpec = tween(KeynearMotion.MORPH_DURATION_MS, easing = KeynearMotion.SpringEasing),
        label = "visibilityFg",
    )
    Row(
        modifier = modifier
            .heightIn(min = KeynearSpacing.MinTouchTarget)
            .clip(KeynearShapes.Xl)
            .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null, onClick = onClick)
            .padding(horizontal = KeynearLayout.ContentGap),
        horizontalArrangement = Arrangement.Center,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        when (kind) {
            0 -> IconEyeOff(modifier = Modifier.size(KeynearSpacing.Lg), tint = fg)
            1 -> IconShieldCheck(modifier = Modifier.size(KeynearSpacing.Lg), tint = fg)
            else -> IconRadio(modifier = Modifier.size(KeynearSpacing.Lg), tint = fg)
        }
        Spacer(modifier = Modifier.width(KeynearSpacing.Sm))
        Text(label, color = fg, fontSize = KeynearTypeSizes.Description, fontWeight = FontWeight.Bold,
            maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}

@Composable
internal fun CenterSelfNode(localDeviceName: String, visibility: VisibilityState, remainingSecs: Long) {
    val palette = LocalKeynearPalette.current
    val active = visibility != VisibilityState.HIDDEN
    val morphFloat = tween<Float>(KeynearMotion.MORPH_DURATION_MS, easing = KeynearMotion.SpringEasing)
    val morphSize = tween<IntSize>(KeynearMotion.MORPH_DURATION_MS, easing = KeynearMotion.SpringEasing)
    val accentAlpha by animateFloatAsState(if (active) 1f else 0f, morphFloat, label = "centerAccentAlpha")
    val iconTint by animateColorAsState(
        targetValue = if (active) palette.onAccent else palette.textSecondary,
        animationSpec = tween(KeynearMotion.MORPH_DURATION_MS, easing = KeynearMotion.SpringEasing),
        label = "centerIconTint",
    )
    var lastRemainingSecs by remember { mutableLongStateOf(remainingSecs) }
    if (visibility == VisibilityState.EVERYONE_NEARBY) lastRemainingSecs = remainingSecs

    Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(KeynearLayout.ContentGap)) {
        Box(modifier = Modifier.size(SelfNodeSize), contentAlignment = Alignment.Center) {
            TactileSurface(
                modifier = Modifier.fillMaxSize(),
                shape = CircleShape,
                containerColor = palette.surface.copy(alpha = FOG_SURFACE_ALPHA),
                tone = TactileTone.SURFACE,
                cornerRadius = SelfNodeRadius,
                isCircle = true,
            ) {}
            if (accentAlpha > 0f) {
                TactileSurface(
                    modifier = Modifier.fillMaxSize().graphicsLayer { alpha = accentAlpha },
                    shape = CircleShape,
                    containerColor = palette.loading,
                    tone = TactileTone.ACCENT,
                    cornerRadius = SelfNodeRadius,
                    isCircle = true,
                ) {}
            }
            IconSmartphone(modifier = Modifier.size(SelfIconSize), tint = iconTint)
        }

        TactileSurface(
            shape = KeynearShapes.Xl,
            containerColor = palette.surface.copy(alpha = FOG_SURFACE_ALPHA),
            tone = TactileTone.SURFACE,
            cornerRadius = KeynearRadii.Xl,
        ) {
            Row(
                modifier = Modifier.padding(horizontal = KeynearDimensions.Dp14, vertical = KeynearDimensions.Dp6),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(localDeviceName, color = palette.textPrimary, fontSize = KeynearTypeSizes.Caption, fontWeight = FontWeight.Bold)
                AnimatedVisibility(
                    visible = visibility == VisibilityState.EVERYONE_NEARBY,
                    enter = expandHorizontally(morphSize) + fadeIn(morphFloat),
                    exit = shrinkHorizontally(morphSize) + fadeOut(morphFloat),
                ) {
                    val safeSecs = lastRemainingSecs.coerceAtLeast(0L)
                    val mins = safeSecs / SECONDS_PER_MINUTE
                    val secs = safeSecs % SECONDS_PER_MINUTE
                    Text(
                        text = "$mins:${secs.toString().padStart(2, '0')}",
                        color = palette.loadingInk,
                        fontSize = KeynearTypeSizes.Caption,
                        fontWeight = FontWeight.Bold,
                        modifier = Modifier.padding(start = KeynearLayout.CompactGap),
                    )
                }
            }
        }
    }
}
