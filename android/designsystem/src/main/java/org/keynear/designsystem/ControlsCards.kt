package org.keynear.designsystem

import android.graphics.BlurMaskFilter
import android.graphics.Paint as AndroidPaint
import android.graphics.Path as AndroidPath
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlin.math.abs

private val CaptionLetterSpacing = 0.8.sp
private val DefaultElevation = 5.dp
private val TextSubtitleGap = 2.dp
private val SheetClipMarginPx = 200f
private val ShadowBlurThresholdPx = 0.5f
private val CushionBlurThresholdPx = 0.4f
private val CushionPaddingMarginPx = 8f

enum class TactileTone {
    SURFACE,
    TONAL,
    ACCENT,
    SHEET,
    INSET,
    THUMB,
    NONE,
}

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
    val specs = when (tone) {
        TactileTone.SURFACE -> if (isDark) KeynearElevation.OuterSurfaceDark else KeynearElevation.OuterSurfaceLight
        TactileTone.TONAL -> if (isDark) KeynearElevation.OuterTonalDark else KeynearElevation.OuterTonalLight
        TactileTone.ACCENT -> if (isDark) KeynearElevation.OuterAccentDark else KeynearElevation.OuterAccentLight
        TactileTone.SHEET -> if (isDark) KeynearElevation.OuterSheetDark else KeynearElevation.OuterSheetLight
        TactileTone.THUMB -> if (isDark) KeynearElevation.OuterThumbDark else KeynearElevation.OuterThumbLight
        TactileTone.INSET, TactileTone.NONE -> KeynearElevation.OuterNone
    }
    return specs.map { ShadowSpec(it.offsetY, it.blurRadius, it.spread, it.color) }
}

private fun insetShadowSpecs(tone: TactileTone, isDark: Boolean): List<InsetShadowSpec> {
    val specs = when (tone) {
        TactileTone.SURFACE -> if (isDark) KeynearElevation.InsetSurfaceDark else KeynearElevation.InsetSurfaceLight
        TactileTone.TONAL -> if (isDark) KeynearElevation.InsetTonalDark else KeynearElevation.InsetTonalLight
        TactileTone.ACCENT -> if (isDark) KeynearElevation.InsetAccentDark else KeynearElevation.InsetAccentLight
        TactileTone.SHEET -> if (isDark) KeynearElevation.InsetSheetDark else KeynearElevation.InsetSheetLight
        TactileTone.THUMB -> if (isDark) KeynearElevation.InsetThumbDark else KeynearElevation.InsetThumbLight
        TactileTone.INSET -> if (isDark) KeynearElevation.InsetInsetDark else KeynearElevation.InsetInsetLight
        TactileTone.NONE -> KeynearElevation.InsetNone
    }
    return specs.map { InsetShadowSpec(it.offsetY, it.blurRadius, it.color) }
}

fun Modifier.tactileShadow(
    tone: TactileTone,
    isDark: Boolean,
    cornerRadius: Dp = KeynearRadii.TwoXl,
    isCircle: Boolean = false,
): Modifier {
    val specs = outerShadowSpecs(tone, isDark)
    if (specs.isEmpty()) return this
    return drawBehind {
        drawIntoCanvas { canvas ->
            val native = canvas.nativeCanvas
            val radiusPx = cornerRadius.toPx()
            specs.forEach { spec ->
                val blurPx = spec.blurRadius.toPx()
                val spreadPx = spec.spread.toPx()
                val offsetYPx = spec.offsetY.toPx()
                val paint = AndroidPaint(AndroidPaint.ANTI_ALIAS_FLAG).apply {
                    color = spec.color.toArgb()
                    if (blurPx > ShadowBlurThresholdPx) {
                        maskFilter = BlurMaskFilter(blurPx, BlurMaskFilter.Blur.NORMAL)
                    }
                }
                if (isCircle) {
                    val cx = size.width * 0.5f
                    val cy = size.height * 0.5f + offsetYPx
                    val r = (size.minDimension * 0.5f + spreadPx).coerceAtLeast(1f)
                    native.drawCircle(cx, cy, r, paint)
                } else if (tone == TactileTone.SHEET) {
                    native.save()
                    native.clipRect(-SheetClipMarginPx, -SheetClipMarginPx, size.width + SheetClipMarginPx, size.height)
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
                } else {
                    native.drawRoundRect(
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
        }
    }
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
            size.minDimension * 0.5f
        } else {
            cornerRadius.toPx().coerceAtMost(size.minDimension * 0.5f)
        }
        drawIntoCanvas { canvas ->
            val native = canvas.nativeCanvas
            specs.forEach { spec ->
                val blurPx = spec.blurRadius.toPx()
                val offsetYPx = spec.offsetY.toPx()
                val pad = blurPx * 2f + abs(offsetYPx) + CushionPaddingMarginPx
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
                    if (blurPx > CushionBlurThresholdPx) {
                        maskFilter = BlurMaskFilter(blurPx, BlurMaskFilter.Blur.NORMAL)
                    }
                }
                native.drawPath(path, paint)
            }
        }
        drawContent()
    }
}

@Composable
fun TactileSurface(
    modifier: Modifier = Modifier,
    shape: Shape = KeynearShapes.Card,
    containerColor: Color = LocalKeynearPalette.current.surface,
    shadowElevation: Dp = DefaultElevation,
    tone: TactileTone = if (shadowElevation == 0.dp) TactileTone.NONE else TactileTone.SURFACE,
    isAccent: Boolean = false,
    cornerRadius: Dp = KeynearRadii.TwoXl,
    isCircle: Boolean = false,
    onClick: (() -> Unit)? = null,
    enabled: Boolean = true,
    content: @Composable () -> Unit,
) {
    val isDark = LocalKeynearPalette.current.isDark
    val effectiveTone = if (isAccent) TactileTone.ACCENT else tone
    Box(
        modifier = modifier
            .tactileShadow(effectiveTone, isDark, cornerRadius, isCircle)
            .clip(shape)
            .background(containerColor)
            .tactileCushion(effectiveTone, isDark, cornerRadius, isCircle)
            .let { m -> if (onClick != null && enabled) m.clickable(onClick = onClick) else m },
    ) { content() }
}

@Composable
fun SectionHeading(
    title: String,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    Text(
        text = title.uppercase(),
        color = palette.textMuted,
        fontSize = KeynearTypeSizes.Caption,
        fontWeight = FontWeight.Bold,
        letterSpacing = CaptionLetterSpacing,
        modifier = modifier.padding(horizontal = KeynearSpacing.Xs),
    )
}

@Composable
fun SettingsSection(
    title: String,
    modifier: Modifier = Modifier,
    content: @Composable ColumnScope.() -> Unit,
) {
    val palette = LocalKeynearPalette.current
    Column(
        modifier = modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(KeynearLayout.SectionHeadingBottom),
    ) {
        SectionHeading(title = title)
        TactileSurface(
            modifier = Modifier.fillMaxWidth(),
            shape = KeynearShapes.Card,
            containerColor = palette.surface,
            tone = TactileTone.SURFACE,
            cornerRadius = KeynearRadii.TwoXl,
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(KeynearLayout.SectionInnerPadding),
                verticalArrangement = Arrangement.spacedBy(KeynearLayout.SectionRowGap),
                content = content,
            )
        }
    }
}

@Composable
fun SettingRow(
    title: String,
    subtitle: String? = null,
    onClick: (() -> Unit)? = null,
    trailing: @Composable () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val baseModifier = Modifier
        .fillMaxWidth()
        .defaultMinSize(minHeight = KeynearLayout.RowMinHeight)
        .clip(KeynearShapes.Xl)
        .let { m -> if (onClick != null) m.clickable(onClick = onClick) else m }
        .padding(
            horizontal = KeynearLayout.RowHorizontalPadding,
            vertical = KeynearLayout.RowVerticalPadding,
        )

    Row(
        modifier = baseModifier,
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
    ) {
        Column(
            modifier = Modifier
                .weight(1f)
                .padding(end = KeynearSpacing.Sm),
            verticalArrangement = Arrangement.spacedBy(TextSubtitleGap),
        ) {
            Text(
                text = title,
                color = palette.textPrimary,
                fontSize = KeynearTypeSizes.Body,
                fontWeight = FontWeight.SemiBold,
            )
            if (!subtitle.isNullOrBlank()) {
                Text(
                    text = subtitle,
                    color = palette.textMuted,
                    fontSize = KeynearTypeSizes.Caption,
                )
            }
        }
        trailing()
    }
}

