package org.keynear.app.ui.receive

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.ui.model.VisibilityState
import org.keynear.designsystem.IconEyeOff
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
import kotlin.math.roundToInt

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
