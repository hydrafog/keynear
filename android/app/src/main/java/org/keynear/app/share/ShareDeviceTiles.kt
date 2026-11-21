package org.keynear.app.share

import androidx.compose.foundation.Canvas
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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.ui.model.DeviceRoleUi
import org.keynear.app.ui.model.PairedDeviceUi
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
internal fun ShareDeviceTile(
    device: PairedDeviceUi,
    highlighted: Boolean,
    isSending: Boolean,
    enabled: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    Surface(
        onClick = onClick,
        enabled = enabled,
        modifier = modifier.defaultMinSize(minHeight = DeviceTileMinHeight),
        shape = RoundedCornerShape(DeviceTileCornerRadius),
        color = if (highlighted) palette.loadingTonal else palette.surfaceTonal,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Md),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Surface(
                    modifier = Modifier.size(DeviceIconContainerSize),
                    shape = RoundedCornerShape(DeviceIconContainerRadius),
                    color = if (device.isOnline) palette.loadingTonal else palette.surface,
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        if (isSending) {
                            CircularProgressIndicator(
                                modifier = Modifier.size(20.dp),
                                strokeWidth = 2.dp,
                                color = palette.loading,
                            )
                        } else {
                            ShareRoleIcon(
                                role = device.role,
                                color = if (device.isOnline) palette.onLoadingTonal else palette.textMuted,
                                size = 22.dp,
                            )
                        }
                    }
                }
            }
            Column {
                Text(
                    text = device.displayName,
                    fontSize = KeynearTypeSizes.Description,
                    fontWeight = FontWeight.Bold,
                    color = palette.textPrimary,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    text = when {
                        isSending -> "Starting..."
                        device.isOnline -> "Ready · Wi-Fi"
                        else -> "Standby"
                    },
                    fontSize = KeynearTypeSizes.Caption,
                    color = if (device.isOnline) palette.loading else palette.textMuted,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

@Composable
private fun ShareRoleIcon(role: DeviceRoleUi, color: Color, size: Dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.095f
        when (role) {
            DeviceRoleUi.MOBILE -> {
                drawRoundRect(
                    color = color,
                    topLeft = Offset(w * 0.26f, h * 0.12f),
                    size = Size(w * 0.48f, h * 0.76f),
                    cornerRadius = CornerRadius(w * 0.08f, w * 0.08f),
                    style = Stroke(width = sw),
                )
                drawCircle(color = color, radius = sw * 0.65f, center = Offset(w * 0.50f, h * 0.76f))
            }
            DeviceRoleUi.DESKTOP -> {
                drawRoundRect(
                    color = color,
                    topLeft = Offset(w * 0.18f, h * 0.22f),
                    size = Size(w * 0.64f, h * 0.46f),
                    cornerRadius = CornerRadius(w * 0.06f, w * 0.06f),
                    style = Stroke(width = sw),
                )
                drawLine(color, Offset(w * 0.10f, h * 0.76f), Offset(w * 0.90f, h * 0.76f), strokeWidth = sw, cap = StrokeCap.Round)
            }
        }
    }
}

@Composable
internal fun ShareCloseIcon(color: Color, size: Dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.12f
        drawLine(color, Offset(w * 0.24f, h * 0.24f), Offset(w * 0.76f, h * 0.76f), strokeWidth = sw, cap = StrokeCap.Round)
        drawLine(color, Offset(w * 0.76f, h * 0.24f), Offset(w * 0.24f, h * 0.76f), strokeWidth = sw, cap = StrokeCap.Round)
    }
}
