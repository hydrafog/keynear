package org.keynear.app.share

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import org.keynear.app.ui.model.ShareValidationResult
import org.keynear.app.ui.model.formatBytes
import org.keynear.designsystem.ActionButton
import org.keynear.designsystem.IconClose
import org.keynear.designsystem.KeynearDimensions
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearPressScale
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone

@Composable
internal fun ShareSheetHeader(
    validation: ShareValidationResult,
    totalSizeBytes: Long,
    onClose: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
        Box(
            modifier = Modifier
                .width(KeynearSpacing.MinTouchTarget)
                .height(KeynearDimensions.Dp6)
                .clip(CircleShape)
                .background(palette.sheetHandle),
        )
    }

    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            val headerTitle = when (validation) {
                is ShareValidationResult.Valid ->
                    "Share ${validation.itemCount} item${if (validation.itemCount == 1) "" else "s"}"
                is ShareValidationResult.Invalid -> "Unable to share"
            }
            Text(
                text = headerTitle,
                fontSize = KeynearTypeSizes.Sp19,
                fontWeight = FontWeight.Bold,
                color = palette.textPrimary,
            )
            val subtitle = when (validation) {
                is ShareValidationResult.Valid ->
                    if (totalSizeBytes > 0L) "${formatBytes(totalSizeBytes)} · Direct Share" else "Direct Share"
                is ShareValidationResult.Invalid -> validation.reason
            }
            Text(text = subtitle, fontSize = KeynearTypeSizes.Caption, color = palette.textMuted)
        }
        TactileSurface(
            modifier = Modifier.size(KeynearLayout.EmptyBadgeSize),
            shape = CircleShape,
            containerColor = palette.surfaceTonal,
            tone = TactileTone.TONAL,
            cornerRadius = KeynearDimensions.Dp22,
            isCircle = true,
            pressScale = KeynearPressScale.PRESS_95,
            contentAlignment = Alignment.Center,
            onClick = onClose,
        ) {
            IconClose(
                modifier = Modifier.size(KeynearDimensions.Dp20),
                tint = palette.textSecondary,
            )
        }
    }
}

@Composable
internal fun ShareInvalidSection(
    reason: String,
    onOpenMain: () -> Unit,
    onClose: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    TactileSurface(
        modifier = Modifier.fillMaxWidth(),
        shape = KeynearShapes.Xl,
        containerColor = palette.errorTonal,
        tone = TactileTone.DANGER_TONAL,
        cornerRadius = KeynearRadii.Xl,
    ) {
        Text(
            text = reason,
            fontSize = KeynearTypeSizes.Description,
            color = palette.onDangerTonal,
            modifier = Modifier.padding(KeynearSpacing.Lg),
        )
    }
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(KeynearLayout.ContentGap),
    ) {
        ActionButton(
            label = "Open Keynear",
            primary = true,
            onClick = onOpenMain,
            modifier = Modifier.weight(1f),
        )
        ActionButton(
            label = "Close",
            tonalSurface = true,
            onClick = onClose,
        )
    }
}
