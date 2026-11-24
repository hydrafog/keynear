package org.keynear.designsystem

import android.graphics.BlurMaskFilter
import android.graphics.Canvas
import android.graphics.Paint as AndroidPaint
import android.graphics.Path as AndroidPath
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.unit.Dp
import kotlin.math.abs

private const val SHEET_CLIP_MARGIN_PX = 200f
private const val SHADOW_BLUR_THRESHOLD_PX = 0.5f
private const val CUSHION_BLUR_THRESHOLD_PX = 0.4f
private const val CUSHION_PADDING_MARGIN_PX = 8f
private const val HALF_FRACTION = 0.5f
private const val LIGHT_ACCENT_SHADOW_ALPHA = 0.52f
private const val DARK_ACCENT_SHADOW_ALPHA = 0.54f
private const val LIGHT_TONAL_TOP_RIM_ALPHA = 0.78f
private const val DARK_TONAL_TOP_RIM_ALPHA = 0.28f
private const val DARK_TONAL_TOP_RIM_ACCENT_MIX = 0.24f

internal const val SYMBOLIC_BEACON_ALPHA = 0.08f
internal const val SYMBOLIC_PEER_HALO_ALPHA = 0.22f
internal const val SYMBOLIC_PEER_CORE_ALPHA = 0.96f
internal const val SYMBOLIC_OUTER_ARC_ALPHA = 0.18f
internal const val SYMBOLIC_MIDDLE_ORBIT_ALPHA = 0.24f
internal const val SYMBOLIC_INNER_ORBIT_ALPHA = 0.30f
internal const val SYMBOLIC_WAVE_ALPHA = 0.68f
internal const val SYMBOLIC_INNER_WAVE_ALPHA = 0.88f

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

private val OUTER_SHADOW_VARIANTS: Map<TactileTone, Pair<List<KeynearOuterShadow>, List<KeynearOuterShadow>>> = mapOf(
    TactileTone.SURFACE to (KeynearElevation.OuterSurfaceLight to KeynearElevation.OuterSurfaceDark),
    TactileTone.TONAL to (KeynearElevation.OuterTonalLight to KeynearElevation.OuterTonalDark),
    TactileTone.ACCENT_TONAL to (KeynearElevation.OuterTonalLight to KeynearElevation.OuterTonalDark),
    TactileTone.DANGER to (KeynearElevation.OuterTonalLight to KeynearElevation.OuterTonalDark),
    TactileTone.DANGER_TONAL to (KeynearElevation.OuterTonalLight to KeynearElevation.OuterTonalDark),
    TactileTone.ACCENT to (KeynearElevation.OuterAccentLight to KeynearElevation.OuterAccentDark),
    TactileTone.SHEET to (KeynearElevation.OuterSheetLight to KeynearElevation.OuterSheetDark),
    TactileTone.THUMB to (KeynearElevation.OuterThumbLight to KeynearElevation.OuterThumbDark),
)

private val INSET_SHADOW_VARIANTS: Map<TactileTone, Pair<List<KeynearInsetShadow>, List<KeynearInsetShadow>>> = mapOf(
    TactileTone.SURFACE to (KeynearElevation.InsetSurfaceLight to KeynearElevation.InsetSurfaceDark),
    TactileTone.TONAL to (KeynearElevation.InsetTonalLight to KeynearElevation.InsetTonalDark),
    TactileTone.ACCENT to (KeynearElevation.InsetAccentLight to KeynearElevation.InsetAccentDark),
    TactileTone.DANGER to (KeynearElevation.InsetAccentLight to KeynearElevation.InsetAccentDark),
    TactileTone.SHEET to (KeynearElevation.InsetSheetLight to KeynearElevation.InsetSheetDark),
    TactileTone.THUMB to (KeynearElevation.InsetThumbLight to KeynearElevation.InsetThumbDark),
    TactileTone.INSET to (KeynearElevation.InsetInsetLight to KeynearElevation.InsetInsetDark),
)

private fun outerShadowSpecs(tone: TactileTone, isDark: Boolean, accentColor: Color?): List<ShadowSpec> {
    val variants = OUTER_SHADOW_VARIANTS[tone] ?: return emptyList()
    val specs = if (isDark) variants.second else variants.first
    val accentAlpha = if (isDark) DARK_ACCENT_SHADOW_ALPHA else LIGHT_ACCENT_SHADOW_ALPHA
    return specs.mapIndexed { idx, spec ->
        val resolvedColor = if (tone == TactileTone.ACCENT && idx == 0 && accentColor != null) {
            accentColor.copy(alpha = accentAlpha)
        } else {
            spec.color
        }
        ShadowSpec(spec.offsetY, spec.blurRadius, spec.spread, resolvedColor)
    }
}

private fun tonalTopRimSpec(isDark: Boolean, accentColor: Color?): List<InsetShadowSpec> {
    val topRimColor = if (isDark) {
        val base = accentColor ?: DarkKeynearPalette.accent
        mixSrgb(Color.White, base, DARK_TONAL_TOP_RIM_ACCENT_MIX).copy(alpha = DARK_TONAL_TOP_RIM_ALPHA)
    } else {
        Color.White.copy(alpha = LIGHT_TONAL_TOP_RIM_ALPHA)
    }
    return listOf(InsetShadowSpec(KeynearDimensions.Dp1, KeynearDimensions.Dp1, topRimColor))
}

private fun insetShadowSpecs(tone: TactileTone, isDark: Boolean, accentColor: Color?): List<InsetShadowSpec> {
    if (tone == TactileTone.ACCENT_TONAL) return tonalTopRimSpec(isDark, accentColor)
    val variants = INSET_SHADOW_VARIANTS[tone] ?: return emptyList()
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
    accentColor: Color? = null,
): Modifier {
    val specs = outerShadowSpecs(tone, isDark, accentColor)
    if (specs.isEmpty()) return this
    return drawBehind { drawOuterShadows(specs, cornerRadius, isCircle, tone) }
}

fun Modifier.tactileCushion(
    tone: TactileTone,
    isDark: Boolean,
    cornerRadius: Dp = KeynearRadii.TwoXl,
    isCircle: Boolean = false,
    accentColor: Color? = null,
): Modifier {
    val specs = insetShadowSpecs(tone, isDark, accentColor)
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
