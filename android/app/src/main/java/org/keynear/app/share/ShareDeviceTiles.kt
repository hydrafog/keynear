package org.keynear.app.share

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import org.keynear.app.ui.model.DeviceRoleUi
import org.keynear.app.ui.model.PairedDeviceUi
import org.keynear.designsystem.IconLaptop
import org.keynear.designsystem.IconSmartphone
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

private val DeviceTileMinHeight = 96.dp

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
    TactileSurface(
        modifier = modifier.defaultMinSize(minHeight = DeviceTileMinHeight),
        shape = KeynearShapes.Card,
        containerColor = if (highlighted) palette.loadingTonal else palette.surfaceTonal,
        tone = if (highlighted) TactileTone.ACCENT_TONAL else TactileTone.TONAL,
        cornerRadius = KeynearRadii.TwoXl,
        pressScale = KeynearPressScale.PRESS_95,
        onClick = onClick,
        enabled = enabled,
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
                TactileSurface(
                    modifier = Modifier.size(KeynearLayout.EmptyBadgeSize),
                    shape = KeynearShapes.Xl,
                    containerColor = palette.loadingTonal,
                    tone = TactileTone.ACCENT_TONAL,
                    cornerRadius = KeynearRadii.Xl,
                    contentAlignment = Alignment.Center,
                ) {
                    if (isSending) {
                        CircularProgressIndicator(
                            modifier = Modifier.size(KeynearDimensions.Dp20),
                            strokeWidth = KeynearDimensions.Dp2,
                            color = palette.loading,
                        )
                    } else if (device.role == DeviceRoleUi.DESKTOP) {
                        IconLaptop(
                            modifier = Modifier.size(KeynearDimensions.Dp20),
                            tint = palette.onLoadingTonal,
                        )
                    } else {
                        IconSmartphone(
                            modifier = Modifier.size(KeynearDimensions.Dp20),
                            tint = palette.onLoadingTonal,
                        )
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
                    color = if (device.isOnline) palette.loadingInk else palette.textMuted,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}
