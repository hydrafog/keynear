package org.keynear.app.share

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.ui.model.ShareValidationResult
import org.keynear.app.ui.model.formatBytes
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette

private val SheetCornerTop = 28.dp
private val SheetElevation = 16.dp
private val HandleWidth = 36.dp
private val HandleHeight = 4.dp
private val CloseBadgeSize = 36.dp
private val CardCornerRadius = 20.dp
private val DeviceGridMaxHeight = 320.dp
private val DeviceTileCornerRadius = 18.dp
private val DeviceTileMinHeight = 96.dp
private val DeviceIconContainerSize = 44.dp
private val DeviceIconContainerRadius = 14.dp
private val PillCorner = 999.dp
private val SheetGap = 14.dp
private val SectionGap = 10.dp
private val HeaderTextSize = 19.sp
private val EmptyHeaderSize = 15.sp
private val SmallHeaderSize = 13.sp
private val CaptionLetterSpacing = 0.8.sp
private val MicroLabelSize = 11.sp

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
                .width(HandleWidth)
                .height(HandleHeight)
                .clip(RoundedCornerShape(PillCorner))
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
                fontSize = HeaderTextSize,
                fontWeight = FontWeight.ExtraBold,
                color = palette.textPrimary,
            )
            val subtitle = when (validation) {
                is ShareValidationResult.Valid ->
                    if (totalSizeBytes > 0L) "${formatBytes(totalSizeBytes)} · Direct Share" else "Direct Share"
                is ShareValidationResult.Invalid -> validation.reason
            }
            Text(text = subtitle, fontSize = KeynearTypeSizes.Caption, color = palette.textMuted)
        }
        Box(
            modifier = Modifier
                .size(KeynearSpacing.MinTouchTarget)
                .clip(CircleShape)
                .clickable(onClick = onClose),
            contentAlignment = Alignment.Center,
        ) {
            Surface(modifier = Modifier.size(CloseBadgeSize), shape = CircleShape, color = palette.surfaceTonal) {
                Box(contentAlignment = Alignment.Center) {
                    ShareCloseIcon(color = palette.textSecondary, size = KeynearSpacing.Lg)
                }
            }
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
    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.Xl),
        color = palette.errorTonal,
    ) {
        Text(
            text = reason,
            fontSize = SmallHeaderSize,
            color = palette.onDangerTonal,
            modifier = Modifier.padding(KeynearSpacing.Lg),
        )
    }
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(SectionGap),
    ) {
        Button(
            onClick = onOpenMain,
            modifier = Modifier
                .weight(1f)
                .defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
            shape = RoundedCornerShape(PillCorner),
            colors = ButtonDefaults.buttonColors(
                containerColor = palette.loading,
                contentColor = palette.onAccent,
            ),
        ) {
            Text("Open Keynear", fontWeight = FontWeight.Bold)
        }
        OutlinedButton(
            onClick = onClose,
            modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
            shape = RoundedCornerShape(PillCorner),
        ) {
            Text("Close", color = palette.textPrimary)
        }
    }
}
