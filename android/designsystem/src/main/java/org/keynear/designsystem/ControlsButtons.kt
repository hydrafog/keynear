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
import androidx.compose.ui.unit.dp

private val ActionButtonHorizontalPadding = 20.dp
private val ControlMinHeight = 52.dp
private val ContentGap = 10.dp
private val SelectionTileMinHeight = 128.dp
private const val DISABLED_CONTAINER_ALPHA = 0.45f
private const val DISABLED_CONTENT_ALPHA = 0.65f

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
    val iconColor = if (active) palette.onLoadingTonal else palette.textSecondary
    val textColor = if (active) palette.onLoadingTonal else palette.textPrimary

    TactileSurface(
        modifier = modifier.heightIn(min = SelectionTileMinHeight),
        shape = KeynearShapes.Card,
        containerColor = bg,
        tone = if (active) TactileTone.TONAL else TactileTone.SURFACE,
        cornerRadius = KeynearRadii.TwoXl,
        onClick = onClick,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .heightIn(min = SelectionTileMinHeight)
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

private fun actionContainerColor(
    palette: KeynearPalette,
    danger: Boolean,
    errorTonal: Boolean,
    tonalSurface: Boolean,
    primary: Boolean,
): Color = when {
    danger -> palette.error
    errorTonal -> palette.errorAction
    tonalSurface -> palette.surfaceTonal
    primary -> palette.loading
    else -> palette.loadingTonal
}

private fun actionContentColor(
    palette: KeynearPalette,
    danger: Boolean,
    errorTonal: Boolean,
    tonalSurface: Boolean,
    primary: Boolean,
): Color = when {
    danger || primary -> palette.onAccent
    errorTonal -> palette.onDangerAction
    tonalSurface -> palette.textPrimary
    else -> palette.onLoadingTonal
}

private fun actionTone(enabled: Boolean, primary: Boolean): TactileTone = when {
    !enabled -> TactileTone.NONE
    primary -> TactileTone.ACCENT
    else -> TactileTone.TONAL
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
    val container = actionContainerColor(palette, danger, errorTonal, tonalSurface, primary)
    val content = actionContentColor(palette, danger, errorTonal, tonalSurface, primary)
    val effectiveContainer = if (enabled) container else container.copy(alpha = DISABLED_CONTAINER_ALPHA)
    val effectiveContent = if (enabled) content else content.copy(alpha = DISABLED_CONTENT_ALPHA)
    val buttonTone = actionTone(enabled, primary)

    TactileSurface(
        modifier = modifier.heightIn(min = minHeight),
        shape = KeynearShapes.Xl,
        containerColor = effectiveContainer,
        tone = buttonTone,
        cornerRadius = KeynearRadii.Xl,
        onClick = onClick,
        enabled = enabled,
    ) {
        Row(
            modifier = Modifier
                .heightIn(min = minHeight)
                .padding(horizontal = ActionButtonHorizontalPadding, vertical = ContentGap),
            horizontalArrangement = Arrangement.Center,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (leadingIcon != null) {
                leadingIcon(effectiveContent)
                Spacer(modifier = Modifier.width(KeynearSpacing.Sm))
            }
            Text(
                text = label,
                color = effectiveContent,
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
            .heightIn(min = ControlMinHeight)
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

