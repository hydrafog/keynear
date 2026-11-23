package org.keynear.designsystem

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.interaction.InteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.graphicsLayer

private data class SheenAnchor(
    val xFraction: Float,
    val yFraction: Float,
)

private data class SheenOverlay(
    val color: Color,
    val anchor: SheenAnchor,
    val radiusExtent: Float,
)

private val SHEEN_BUILDERS: Map<TactileTone, (KeynearPalette) -> List<SheenOverlay>> = mapOf(
    TactileTone.SURFACE to { p -> listOf(SheenOverlay(p.surfaceSheen, SheenAnchor(0.5f, 0f), 0.82f)) },
    TactileTone.TONAL to { p -> listOf(SheenOverlay(p.tonalSheen, SheenAnchor(0.28f, 0f), 0.80f)) },
    TactileTone.SHEET to { p -> listOf(SheenOverlay(p.sheetSheen, SheenAnchor(0.5f, 0f), 0.74f)) },
    TactileTone.ACCENT to { p ->
        listOf(
            SheenOverlay(p.accentSheen, SheenAnchor(0.18f, -0.10f), 0.70f),
            SheenOverlay(p.accentShift, SheenAnchor(0.88f, 1.15f), 0.68f),
        )
    },
    TactileTone.ACCENT_TONAL to { p -> listOf(SheenOverlay(p.accentTonalSheen, SheenAnchor(0.18f, 0f), 0.76f)) },
    TactileTone.DANGER to { p -> listOf(SheenOverlay(p.dangerSheen, SheenAnchor(0.18f, -0.10f), 0.70f)) },
    TactileTone.DANGER_TONAL to { p ->
        val color = if (p.isDark) p.dangerSheen else p.accentTonalSheen
        listOf(SheenOverlay(color, SheenAnchor(0.15f, 0f), 0.75f))
    },
    TactileTone.THUMB to { p -> listOf(SheenOverlay(p.thumbSheen, SheenAnchor(0.30f, 0f), 0.76f)) },
)

private fun tactileSheenOverlays(tone: TactileTone, palette: KeynearPalette): List<SheenOverlay> =
    SHEEN_BUILDERS[tone]?.invoke(palette).orEmpty()

private fun DrawScope.drawSheenOverlay(overlay: SheenOverlay) {
    if (overlay.color.alpha == 0f) return
    val center = Offset(size.width * overlay.anchor.xFraction, size.height * overlay.anchor.yFraction)
    val radius = (size.maxDimension * overlay.radiusExtent).coerceAtLeast(1f)
    drawRect(
        brush = Brush.radialGradient(
            colors = listOf(overlay.color, Color.Transparent),
            center = center,
            radius = radius,
        ),
    )
}

fun Modifier.tactileSurfaceBackground(
    tone: TactileTone,
    containerColor: Color,
    palette: KeynearPalette,
): Modifier = drawBehind {
    drawRect(color = containerColor)
    tactileSheenOverlays(tone, palette).forEach { drawSheenOverlay(it) }
    if (tone == TactileTone.ACCENT_TONAL) {
        tactileTopRimColor(tone, palette)?.let { topRim ->
            if (topRim.alpha > 0f) {
                drawRect(
                    color = topRim,
                    size = Size(size.width, KeynearDimensions.Dp1.toPx()),
                )
            }
        }
    }
}

internal fun tactileSurfaceRimColor(tone: TactileTone, palette: KeynearPalette): Color? = when (tone) {
    TactileTone.SURFACE, TactileTone.SHEET -> palette.surfaceRim
    TactileTone.TONAL -> palette.tonalRim
    TactileTone.ACCENT, TactileTone.ACCENT_TONAL -> palette.accentRim
    TactileTone.DANGER, TactileTone.DANGER_TONAL -> palette.dangerRim
    TactileTone.THUMB, TactileTone.INSET, TactileTone.NONE -> null
}

internal fun tactileTopRimColor(tone: TactileTone, palette: KeynearPalette): Color? = when (tone) {
    TactileTone.ACCENT, TactileTone.ACCENT_TONAL -> palette.accentTopRim
    TactileTone.SURFACE, TactileTone.SHEET -> palette.surfaceTopRim
    TactileTone.TONAL,
    TactileTone.DANGER,
    TactileTone.DANGER_TONAL,
    TactileTone.THUMB,
    TactileTone.INSET,
    TactileTone.NONE, -> null
}

@Composable
fun Modifier.tactilePressScale(
    interactionSource: InteractionSource,
    targetScale: Float = KeynearPressScale.PRESS_98,
): Modifier {
    val pressed by interactionSource.collectIsPressedAsState()
    val scale by animateFloatAsState(
        targetValue = if (pressed) targetScale else 1f,
        animationSpec = tween(
            durationMillis = KeynearMotion.PRESS_DURATION_MS,
            easing = KeynearMotion.TactileEasing,
        ),
        label = "tactilePressScale",
    )
    return graphicsLayer {
        scaleX = scale
        scaleY = scale
    }
}
