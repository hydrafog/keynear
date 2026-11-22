package org.keynear.designsystem

import android.graphics.BlurMaskFilter
import android.graphics.Canvas
import android.graphics.Paint as AndroidPaint
import android.graphics.Path as AndroidPath
import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.material3.ColorScheme
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.unit.Dp
import kotlin.math.abs

private const val SHEET_CLIP_MARGIN_PX = 200f
private const val SHADOW_BLUR_THRESHOLD_PX = 0.5f
private const val CUSHION_BLUR_THRESHOLD_PX = 0.4f
private const val CUSHION_PADDING_MARGIN_PX = 8f
private const val HALF_FRACTION = 0.5f

private data class ShadowSpec(
    val offsetY: Dp,
    val blurRadius: Dp,
    val spread: Dp,
    val color: Color,
)

private data class InsetShadowSpec(
    val offsetY: Dp,
    val blurRadius: Dp,
    val color: Color,
)

private fun outerShadowSpecs(tone: TactileTone, isDark: Boolean): List<ShadowSpec> {
    val variants = when (tone) {
        TactileTone.SURFACE -> KeynearElevation.OuterSurfaceLight to KeynearElevation.OuterSurfaceDark
        TactileTone.TONAL -> KeynearElevation.OuterTonalLight to KeynearElevation.OuterTonalDark
        TactileTone.ACCENT -> KeynearElevation.OuterAccentLight to KeynearElevation.OuterAccentDark
        TactileTone.SHEET -> KeynearElevation.OuterSheetLight to KeynearElevation.OuterSheetDark
        TactileTone.THUMB -> KeynearElevation.OuterThumbLight to KeynearElevation.OuterThumbDark
        TactileTone.INSET, TactileTone.NONE -> KeynearElevation.OuterNone to KeynearElevation.OuterNone
    }
    val specs = if (isDark) variants.second else variants.first
    return specs.map { ShadowSpec(it.offsetY, it.blurRadius, it.spread, it.color) }
}

private fun insetShadowSpecs(tone: TactileTone, isDark: Boolean): List<InsetShadowSpec> {
    val variants = when (tone) {
        TactileTone.SURFACE -> KeynearElevation.InsetSurfaceLight to KeynearElevation.InsetSurfaceDark
        TactileTone.TONAL -> KeynearElevation.InsetTonalLight to KeynearElevation.InsetTonalDark
        TactileTone.ACCENT -> KeynearElevation.InsetAccentLight to KeynearElevation.InsetAccentDark
        TactileTone.SHEET -> KeynearElevation.InsetSheetLight to KeynearElevation.InsetSheetDark
        TactileTone.THUMB -> KeynearElevation.InsetThumbLight to KeynearElevation.InsetThumbDark
        TactileTone.INSET -> KeynearElevation.InsetInsetLight to KeynearElevation.InsetInsetDark
        TactileTone.NONE -> KeynearElevation.InsetNone to KeynearElevation.InsetNone
    }
    val specs = if (isDark) variants.second else variants.first
    return specs.map { InsetShadowSpec(it.offsetY, it.blurRadius, it.color) }
}

private fun DrawScope.drawOuterShadows(
    specs: List<ShadowSpec>,
    cornerRadius: Dp,
    isCircle: Boolean,
    tone: TactileTone,
) {
    val radiusPx = cornerRadius.toPx()
    drawIntoCanvas { canvas ->
        specs.forEach { spec ->
            drawOuterSpec(canvas.nativeCanvas, spec, radiusPx, isCircle, tone, size, density)
        }
    }
}

private fun drawOuterSpec(
    native: Canvas,
    spec: ShadowSpec,
    radiusPx: Float,
    isCircle: Boolean,
    tone: TactileTone,
    size: Size,
    density: Float,
) {
    val blurPx = spec.blurRadius.value * density
    val spreadPx = spec.spread.value * density
    val offsetYPx = spec.offsetY.value * density
    val paint = AndroidPaint(AndroidPaint.ANTI_ALIAS_FLAG).apply {
        color = spec.color.toArgb()
        if (blurPx > SHADOW_BLUR_THRESHOLD_PX) {
            maskFilter = BlurMaskFilter(blurPx, BlurMaskFilter.Blur.NORMAL)
        }
    }
    when {
        isCircle -> {
            val cx = size.width * HALF_FRACTION
            val cy = size.height * HALF_FRACTION + offsetYPx
            val r = (size.minDimension * HALF_FRACTION + spreadPx).coerceAtLeast(1f)
            native.drawCircle(cx, cy, r, paint)
        }
        tone == TactileTone.SHEET -> drawSheetShadow(native, size, radiusPx, spreadPx, offsetYPx, paint)
        else -> native.drawRoundRect(
            -spreadPx,
            -spreadPx + offsetYPx,
            size.width + spreadPx,
            size.height + spreadPx + offsetYPx,
            radiusPx,
            radiusPx,
            paint,
        )
    }
}

private fun drawSheetShadow(
    native: Canvas,
    size: Size,
    radiusPx: Float,
    spreadPx: Float,
    offsetYPx: Float,
    paint: AndroidPaint,
) {
    native.save()
    native.clipRect(-SHEET_CLIP_MARGIN_PX, -SHEET_CLIP_MARGIN_PX, size.width + SHEET_CLIP_MARGIN_PX, size.height)
    val path = AndroidPath().apply {
        addRoundRect(
            -spreadPx,
            -spreadPx + offsetYPx,
            size.width + spreadPx,
            size.height + spreadPx,
            floatArrayOf(radiusPx, radiusPx, radiusPx, radiusPx, 0f, 0f, 0f, 0f),
            AndroidPath.Direction.CW,
        )
    }
    native.drawPath(path, paint)
    native.restore()
}

fun Modifier.tactileShadow(
    tone: TactileTone,
    isDark: Boolean,
    cornerRadius: Dp = KeynearRadii.TwoXl,
    isCircle: Boolean = false,
): Modifier {
    val specs = outerShadowSpecs(tone, isDark)
    if (specs.isEmpty()) return this
    return drawBehind { drawOuterShadows(specs, cornerRadius, isCircle, tone) }
}

fun Modifier.tactileCushion(
    tone: TactileTone,
    isDark: Boolean,
    cornerRadius: Dp = KeynearRadii.TwoXl,
    isCircle: Boolean = false,
): Modifier {
    val specs = insetShadowSpecs(tone, isDark)
    if (specs.isEmpty()) return this
    return drawWithContent {
        val radiusPx = if (isCircle) {
            size.minDimension * HALF_FRACTION
        } else {
            cornerRadius.toPx().coerceAtMost(size.minDimension * HALF_FRACTION)
        }
        drawIntoCanvas { canvas ->
            val native = canvas.nativeCanvas
            specs.forEach { spec ->
                val blurPx = spec.blurRadius.toPx()
                val offsetYPx = spec.offsetY.toPx()
                val pad = blurPx * 2f + abs(offsetYPx) + CUSHION_PADDING_MARGIN_PX
                val path = AndroidPath().apply {
                    fillType = AndroidPath.FillType.EVEN_ODD
                    addRect(-pad, -pad, size.width + pad, size.height + pad, AndroidPath.Direction.CW)
                    val bottomRadius = if (tone == TactileTone.SHEET) 0f else radiusPx
                    addRoundRect(
                        0f,
                        offsetYPx,
                        size.width,
                        size.height + offsetYPx,
                        floatArrayOf(radiusPx, radiusPx, radiusPx, radiusPx, bottomRadius, bottomRadius, bottomRadius, bottomRadius),
                        AndroidPath.Direction.CW,
                    )
                }
                val paint = AndroidPaint(AndroidPaint.ANTI_ALIAS_FLAG).apply {
                    color = spec.color.toArgb()
                    if (blurPx > CUSHION_BLUR_THRESHOLD_PX) {
                        maskFilter = BlurMaskFilter(blurPx, BlurMaskFilter.Blur.NORMAL)
                    }
                }
                native.drawPath(path, paint)
            }
        }
        drawContent()
    }
}

internal val KeynearSpringEasing = CubicBezierEasing(0.22f, 1f, 0.36f, 1f)
internal const val SWITCH_GLIDE_DURATION_MS = 360
internal const val SYMBOLIC_STEM_ALPHA = 0.55f
internal const val SYMBOLIC_ARM_ALPHA = 0.75f
internal const val SYMBOLIC_OUTER_ARC_ALPHA = 0.58f

private fun tactileShade(
    base: Color,
    target: Color,
    darkFraction: Float,
    lightFraction: Float,
    isDark: Boolean,
): Color = lerp(base, target, if (isDark) darkFraction else lightFraction)

private fun tactileSurfaceStops(
    tone: TactileTone,
    containerColor: Color,
    palette: KeynearPalette,
): List<Color> {
    val alpha = containerColor.alpha
    val base = containerColor.copy(alpha = 1f)
    fun shaded(target: Color, dark: Float, light: Float): Color =
        tactileShade(base, target, dark, light, palette.isDark).copy(alpha = alpha)
    return when (tone) {
        TactileTone.ACCENT -> listOf(
            shaded(palette.focusRing, 0.36f, 0.36f),
            containerColor,
            shaded(palette.onCanvas, 0.24f, 0.28f),
        )
        TactileTone.SURFACE -> listOf(
            shaded(palette.onAccent, 0.05f, 0.65f),
            containerColor,
            shaded(palette.canvas, 0.32f, 0.42f),
        )
        TactileTone.SHEET -> listOf(
            shaded(palette.onAccent, 0.05f, 0.72f),
            containerColor,
            shaded(palette.canvas, 0.28f, 0.36f),
        )
        TactileTone.TONAL -> listOf(
            shaded(palette.onAccent, 0.04f, 0.38f),
            containerColor,
        )
        TactileTone.THUMB -> listOf(
            palette.onAccent.copy(alpha = alpha),
            shaded(palette.canvas, 0.22f, 0.22f),
        )
        TactileTone.INSET, TactileTone.NONE -> listOf(containerColor, containerColor)
    }
}

internal fun tactileSurfaceBrush(
    tone: TactileTone,
    containerColor: Color,
    palette: KeynearPalette,
): Brush = Brush.verticalGradient(colors = tactileSurfaceStops(tone, containerColor, palette))

internal fun tactileSurfaceRimColor(tone: TactileTone, palette: KeynearPalette): Color? = when (tone) {
    TactileTone.ACCENT -> palette.accentRim
    TactileTone.SURFACE, TactileTone.SHEET ->
        if (palette.isDark) palette.onAccent.copy(alpha = 0.08f) else palette.sheetHandle.copy(alpha = 0.48f)
    TactileTone.TONAL -> palette.tonalRim
    TactileTone.THUMB, TactileTone.INSET, TactileTone.NONE -> null
}

internal fun KeynearPalette.withMaterialAccent(scheme: ColorScheme): KeynearPalette = copy(
    accent = scheme.primary,
    onAccent = scheme.onPrimary,
    focusRing = scheme.primary,
    accentRim = scheme.primary.copy(alpha = if (isDark) 0.28f else 0.40f),
    loading = scheme.primary,
    loadingHover = scheme.primary,
    loadingInk = scheme.primary,
    loadingTonal = scheme.primaryContainer,
    onLoadingTonal = scheme.onPrimaryContainer,
    mapContour = scheme.primary.copy(alpha = 0.20f),
    mapContourFar = scheme.primary.copy(alpha = 0.15f),
    waveSpoke = scheme.primary.copy(alpha = 0.15f),
    waveRing = scheme.primary.copy(alpha = 0.28f),
    waveEcho = scheme.primary.copy(alpha = 0.22f),
)
