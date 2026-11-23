package org.keynear.app.ui.send

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import org.keynear.app.ui.model.DeviceRoleUi
import org.keynear.app.ui.model.PairedDeviceUi
import org.keynear.app.ui.model.SendQueueItemUi
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.designsystem.ActionButton
import org.keynear.designsystem.IconLaptop
import org.keynear.designsystem.IconSend
import org.keynear.designsystem.IconSmartphone
import org.keynear.designsystem.KeynearDimensions
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette
import org.keynear.designsystem.SectionHeading
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone

private val TargetRowMinHeight = 68.dp

@Composable
internal fun SendTargetsSection(
    devices: List<PairedDeviceUi>,
    queue: List<SendQueueItemUi>,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current

    Column(verticalArrangement = Arrangement.spacedBy(KeynearLayout.SectionHeadingBottom)) {
        SectionHeading(title = "Send to")

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
            ) {
                if (devices.isEmpty()) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .heightIn(min = TargetRowMinHeight)
                            .padding(horizontal = KeynearSpacing.Lg, vertical = KeynearSpacing.Md),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
                    ) {
                        TactileSurface(
                            modifier = Modifier.size(KeynearLayout.EmptyBadgeSize),
                            shape = KeynearShapes.Xl,
                            containerColor = palette.loadingTonal,
                            tone = TactileTone.ACCENT_TONAL,
                            cornerRadius = KeynearRadii.Xl,
                            contentAlignment = Alignment.Center,
                        ) {
                            IconLaptop(
                                modifier = Modifier.size(KeynearDimensions.Dp20),
                                tint = palette.onLoadingTonal,
                            )
                        }
                        Text(
                            text = "No paired devices",
                            color = palette.textMuted,
                            fontSize = KeynearTypeSizes.Description,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                } else {
                    devices.forEachIndexed { idx, device ->
                        SendTargetRow(
                            device = device,
                            primary = idx == 0,
                            enabled = queue.isNotEmpty(),
                            onSend = { actions.onSendQueueToPeer(device) },
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun SendTargetRow(
    device: PairedDeviceUi,
    primary: Boolean,
    enabled: Boolean,
    onSend: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = TargetRowMinHeight)
            .padding(horizontal = KeynearSpacing.Lg, vertical = KeynearSpacing.Md),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
    ) {
        TactileSurface(
            modifier = Modifier.size(KeynearLayout.EmptyBadgeSize),
            shape = KeynearShapes.Xl,
            containerColor = palette.loadingTonal,
            tone = TactileTone.ACCENT_TONAL,
            cornerRadius = KeynearRadii.Xl,
            contentAlignment = Alignment.Center,
        ) {
            if (device.role == DeviceRoleUi.DESKTOP) {
                IconLaptop(modifier = Modifier.size(KeynearDimensions.Dp20), tint = palette.onLoadingTonal)
            } else {
                IconSmartphone(modifier = Modifier.size(KeynearDimensions.Dp20), tint = palette.onLoadingTonal)
            }
        }
        Column(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(KeynearLayout.TextSubtitleGap),
        ) {
            Text(
                text = device.displayName,
                color = palette.textPrimary,
                fontSize = KeynearTypeSizes.Body,
                fontWeight = FontWeight.SemiBold,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                text = if (device.isOnline) "Connected via Wi-Fi" else "Bluetooth standby",
                color = palette.textMuted,
                fontSize = KeynearTypeSizes.Caption,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
        ActionButton(
            label = "Send",
            primary = primary,
            enabled = enabled,
            minHeight = KeynearSpacing.MinTouchTarget,
            leadingIcon = { tint ->
                IconSend(modifier = Modifier.size(KeynearSpacing.Lg), tint = tint)
            },
            onClick = onSend,
        )
    }
}
