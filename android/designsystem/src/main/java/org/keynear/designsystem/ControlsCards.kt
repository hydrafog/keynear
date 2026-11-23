package org.keynear.designsystem

import androidx.compose.foundation.LocalIndication
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
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
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

@Composable
private fun Modifier.tactileInteractiveSurface(
    onClick: (() -> Unit)?,
    enabled: Boolean,
    interactionSource: MutableInteractionSource?,
    pressScale: Float,
): Modifier {
    if (onClick == null || !enabled) return this
    val resolved = interactionSource ?: remember { MutableInteractionSource() }
    return tactilePressScale(resolved, pressScale).clickable(
        interactionSource = resolved,
        indication = LocalIndication.current,
        onClick = onClick,
    )
}

private fun Modifier.tactileOptionalRim(rimColor: Color?, shape: Shape): Modifier =
    if (rimColor != null) border(KeynearDimensions.Dp1, rimColor, shape) else this

@Composable
fun TactileSurface(
    modifier: Modifier = Modifier,
    shape: Shape = KeynearShapes.Card,
    containerColor: Color = LocalKeynearPalette.current.surface,
    shadowElevation: Dp = KeynearDimensions.Dp5,
    tone: TactileTone = if (shadowElevation == 0.dp) TactileTone.NONE else TactileTone.SURFACE,
    isAccent: Boolean = false,
    cornerRadius: Dp = KeynearRadii.TwoXl,
    isCircle: Boolean = false,
    onClick: (() -> Unit)? = null,
    enabled: Boolean = true,
    contentAlignment: Alignment = Alignment.TopStart,
    pressScale: Float = KeynearPressScale.PRESS_98,
    interactionSource: MutableInteractionSource? = null,
    content: @Composable () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val isDark = palette.isDark
    val effectiveTone = if (isAccent) TactileTone.ACCENT else tone
    val rimColor = tactileSurfaceRimColor(effectiveTone, palette)
    Box(
        modifier = modifier
            .tactileInteractiveSurface(onClick, enabled, interactionSource, pressScale)
            .tactileShadow(effectiveTone, isDark, cornerRadius, isCircle)
            .clip(shape)
            .tactileSurfaceBackground(effectiveTone, containerColor, palette)
            .tactileOptionalRim(rimColor, shape)
            .tactileCushion(effectiveTone, isDark, cornerRadius, isCircle),
        contentAlignment = contentAlignment,
    ) { content() }
}

@Composable
fun SectionHeading(
    title: String,
    modifier: Modifier = Modifier,
    textAlign: TextAlign = TextAlign.Start,
) {
    val palette = LocalKeynearPalette.current
    Text(
        text = title.uppercase(),
        color = palette.textMuted,
        fontSize = KeynearTypeSizes.Caption,
        fontWeight = FontWeight.Bold,
        letterSpacing = KeynearTypeSizes.Micro,
        textAlign = textAlign,
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
            verticalArrangement = Arrangement.spacedBy(KeynearLayout.TextSubtitleGap),
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
