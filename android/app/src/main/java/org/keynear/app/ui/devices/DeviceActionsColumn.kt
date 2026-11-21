package org.keynear.app.ui.devices

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.ui.model.PairedDeviceUi
import org.keynear.designsystem.ActionButton
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.designsystem.IconRefreshCw
import org.keynear.designsystem.IconTrash2
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette

private val ActionIconSize = 20.dp
private val ContentGap = 10.dp
private val ControlMinHeight = 52.dp
private val DetailRowMinHeight = 54.dp
private val DeviceNameSize = 18.sp
private val HeaderIconSize = 56.dp
private val RowIconSize = 14.dp
private val SectionIconSize = 28.dp
private val TextSubtitleGap = 2.dp
private val ValueSize = 22.sp

@Composable
internal fun DeviceActionsColumn(
    device: PairedDeviceUi,
    confirmingUnpair: Boolean,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current

Column(verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Md)) {
    TactileSurface(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = DetailRowMinHeight),
        shape = KeynearShapes.Xl,
        containerColor = palette.surface,
        tone = TactileTone.SURFACE,
        cornerRadius = KeynearRadii.Xl,
        onClick = { actions.onResetBleBond(device.deviceId) },
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .heightIn(min = DetailRowMinHeight)
                .padding(horizontal = ActionIconSize),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.Center,
        ) {
            IconRefreshCw(modifier = Modifier.size(ActionIconSize), tint = palette.textPrimary)
            Spacer(modifier = Modifier.size(ContentGap))
            Text(
                text = "Reset Bluetooth bond",
                color = palette.textPrimary,
                fontSize = KeynearTypeSizes.Description,
                fontWeight = FontWeight.Bold,
            )
        }
    }

    if (!confirmingUnpair) {
        TactileSurface(
            modifier = Modifier
                .fillMaxWidth()
                .heightIn(min = DetailRowMinHeight),
            shape = KeynearShapes.Xl,
            containerColor = palette.surface,
            tone = TactileTone.SURFACE,
            cornerRadius = KeynearRadii.Xl,
            onClick = { actions.onRequestUnpair(device.deviceId) },
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .heightIn(min = DetailRowMinHeight)
                    .padding(horizontal = ActionIconSize),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.Center,
            ) {
                IconTrash2(modifier = Modifier.size(ActionIconSize), tint = palette.error)
                Spacer(modifier = Modifier.size(ContentGap))
                Text(
                    text = "Remove device",
                    color = palette.error,
                    fontSize = KeynearTypeSizes.Description,
                    fontWeight = FontWeight.Bold,
                )
            }
        }
    } else {
        TactileSurface(
            modifier = Modifier.fillMaxWidth(),
            shape = KeynearShapes.Card,
            containerColor = palette.errorTonal,
            tone = TactileTone.TONAL,
            cornerRadius = KeynearRadii.TwoXl,
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(KeynearLayout.CardPaddingLarge),
                verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Lg),
            ) {
                Text(
                    text = "Remove ${device.displayName}? You will need to confirm a new " +
                        "6-digit code on both devices to pair again.",
                    color = palette.onDangerTonal,
                    fontSize = KeynearTypeSizes.Description,
                    fontWeight = FontWeight.SemiBold,
                    lineHeight = ValueSize,
                )
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
                ) {
                    ActionButton(
                        label = "Remove device",
                        danger = true,
                        minHeight = ControlMinHeight,
                        onClick = { actions.onConfirmUnpair(device.deviceId) },
                        modifier = Modifier.weight(1f),
                    )
                    ActionButton(
                        label = "Cancel",
                        tonalSurface = true,
                        minHeight = ControlMinHeight,
                        onClick = actions.onCancelUnpair,
                        modifier = Modifier.weight(1f),
                    )
                }
            }
        }
    }
}
