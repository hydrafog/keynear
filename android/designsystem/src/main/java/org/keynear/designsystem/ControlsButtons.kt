package org.keynear.designsystem

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp

@Composable
fun SelectionTile(
    label: String,
    active: Boolean = false,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    icon: @Composable (Color) -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val bg = if (active) palette.loadingTonal else palette.surface
    val iconColor = if (active) palette.onLoadingTonal else palette.loadingInk
    val textColor = if (active) palette.onLoadingTonal else palette.textPrimary

    TactileSurface(
        modifier = modifier.heightIn(min = KeynearLayout.SelectionTileMinHeight),
        shape = KeynearShapes.Card,
        containerColor = bg,
        tone = if (active) TactileTone.ACCENT_TONAL else TactileTone.SURFACE,
        cornerRadius = KeynearRadii.TwoXl,
        onClick = onClick,
        pressScale = KeynearPressScale.PRESS_97,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .heightIn(min = KeynearLayout.SelectionTileMinHeight)
                .padding(KeynearLayout.CardPaddingLarge),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            icon(iconColor)
            Spacer(modifier = Modifier.height(KeynearSpacing.Md))
            Text(
                text = label,
                color = textColor,
                fontSize = KeynearTypeSizes.Body,
                fontWeight = FontWeight.Bold,
            )
        }
    }
}

private enum class ActionButtonStyle {
    DANGER,
    DANGER_TONAL,
    TONAL_SURFACE,
    PRIMARY,
    ACCENT_TONAL,
}

private data class ActionButtonAppearance(
    val container: Color,
    val content: Color,
    val tone: TactileTone,
)

private fun resolveActionStyle(
    danger: Boolean,
    errorTonal: Boolean,
    tonalSurface: Boolean,
    primary: Boolean,
): ActionButtonStyle = when {
    danger -> ActionButtonStyle.DANGER
    errorTonal -> ActionButtonStyle.DANGER_TONAL
    tonalSurface -> ActionButtonStyle.TONAL_SURFACE
    primary -> ActionButtonStyle.PRIMARY
    else -> ActionButtonStyle.ACCENT_TONAL
}

private fun actionButtonAppearance(
    style: ActionButtonStyle,
    palette: KeynearPalette,
    enabled: Boolean,
): ActionButtonAppearance {
    val base = when (style) {
        ActionButtonStyle.DANGER -> ActionButtonAppearance(palette.error, palette.onAccent, TactileTone.DANGER)
        ActionButtonStyle.DANGER_TONAL ->
            ActionButtonAppearance(palette.errorAction, palette.onDangerAction, TactileTone.DANGER_TONAL)
        ActionButtonStyle.TONAL_SURFACE ->
            ActionButtonAppearance(palette.surfaceTonal, palette.textPrimary, TactileTone.TONAL)
        ActionButtonStyle.PRIMARY -> ActionButtonAppearance(palette.loading, palette.onAccent, TactileTone.ACCENT)
        ActionButtonStyle.ACCENT_TONAL ->
            ActionButtonAppearance(palette.loadingTonal, palette.onLoadingTonal, TactileTone.ACCENT_TONAL)
    }
    return if (enabled) {
        base
    } else {
        ActionButtonAppearance(
            base.container.copy(alpha = KeynearOpacity.DISABLED_CONTAINER),
            base.content.copy(alpha = KeynearOpacity.DISABLED_CONTENT),
            TactileTone.NONE,
        )
    }
}

@Composable
fun ActionButton(
    label: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    primary: Boolean = false,
    danger: Boolean = false,
    errorTonal: Boolean = false,
    tonalSurface: Boolean = false,
    enabled: Boolean = true,
    minHeight: Dp = KeynearSpacing.MinTouchTarget,
    leadingIcon: (@Composable (Color) -> Unit)? = null,
) {
    val palette = LocalKeynearPalette.current
    val style = resolveActionStyle(danger, errorTonal, tonalSurface, primary)
    val appearance = actionButtonAppearance(style, palette, enabled)

    TactileSurface(
        modifier = modifier.heightIn(min = minHeight),
        shape = KeynearShapes.Xl,
        containerColor = appearance.container,
        tone = appearance.tone,
        cornerRadius = KeynearRadii.Xl,
        onClick = onClick,
        enabled = enabled,
        contentAlignment = Alignment.Center,
        pressScale = KeynearPressScale.PRESS_95,
    ) {
        Row(
            modifier = Modifier.padding(horizontal = KeynearLayout.ActionButtonHorizontalPadding),
            horizontalArrangement = Arrangement.Center,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (leadingIcon != null) {
                leadingIcon(appearance.content)
                Spacer(modifier = Modifier.width(KeynearSpacing.Sm))
            }
            Text(
                text = label,
                color = appearance.content,
                fontSize = KeynearTypeSizes.Description,
                fontWeight = FontWeight.Bold,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

@Composable
fun RecessedInput(
    value: String,
    onValueChange: (String) -> Unit,
    placeholder: String,
    modifier: Modifier = Modifier,
    singleLine: Boolean = true,
) {
    val palette = LocalKeynearPalette.current
    Box(
        modifier = modifier
            .heightIn(min = KeynearLayout.ControlMinHeight)
            .clip(KeynearShapes.Xl)
            .background(palette.input)
            .tactileCushion(
                tone = TactileTone.INSET,
                isDark = palette.isDark,
                cornerRadius = KeynearRadii.Xl,
            )
            .padding(horizontal = KeynearSpacing.Lg, vertical = KeynearSpacing.Md),
        contentAlignment = Alignment.CenterStart,
    ) {
        if (value.isEmpty()) {
            Text(
                text = placeholder,
                color = palette.textMuted,
                fontSize = KeynearTypeSizes.Body,
            )
        }
        BasicTextField(
            value = value,
            onValueChange = onValueChange,
            singleLine = singleLine,
            cursorBrush = SolidColor(palette.loading),
            textStyle = TextStyle(
                color = palette.textPrimary,
                fontSize = KeynearTypeSizes.Body,
                fontWeight = FontWeight.Medium,
            ),
            modifier = Modifier.fillMaxWidth(),
        )
    }
}
