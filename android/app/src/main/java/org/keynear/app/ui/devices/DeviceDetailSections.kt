package org.keynear.app.ui.devices

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.ui.model.DeviceRoleUi
import org.keynear.app.ui.model.PairedDeviceUi
import org.keynear.designsystem.ActionButton
import org.keynear.designsystem.KeynearSwitch
import org.keynear.designsystem.RecessedInput
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.designsystem.IconLaptop
import org.keynear.designsystem.IconSmartphone
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
internal fun DeviceHeaderCard(
    device: PairedDeviceUi,
) {
    val palette = LocalKeynearPalette.current

TactileSurface(
    modifier = Modifier.fillMaxWidth(),
    shape = KeynearShapes.Card,
    containerColor = palette.surface,
    tone = TactileTone.SURFACE,
    cornerRadius = KeynearRadii.TwoXl,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(KeynearLayout.CardPaddingLarge),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Lg),
    ) {
        TactileSurface(
            modifier = Modifier.size(HeaderIconSize),
            shape = KeynearShapes.Xl,
            containerColor = palette.loadingTonal,
            tone = TactileTone.TONAL,
            cornerRadius = KeynearRadii.Xl,
        ) {
            Box(modifier = Modifier.size(HeaderIconSize), contentAlignment = Alignment.Center) {
                val iconTint = palette.onLoadingTonal
                if (device.role == DeviceRoleUi.DESKTOP) {
                    IconLaptop(modifier = Modifier.size(SectionIconSize), tint = iconTint)
                } else {
                    IconSmartphone(modifier = Modifier.size(SectionIconSize), tint = iconTint)
                }
            }
        }
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = device.displayName,
                color = palette.textPrimary,
                fontSize = DeviceNameSize,
                fontWeight = FontWeight.Bold,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                text = "${if (device.isOnline) "Connected via Wi-Fi" else "Bluetooth standby"} (${device.pairedAt})",
                color = palette.textMuted,
                fontSize = KeynearTypeSizes.Description,
            )
        }
    }
}
}
@Composable
internal fun DeviceSettingsCard(
    device: PairedDeviceUi,
    renameDraft: String,
    onRenameDraftChange: (String) -> Unit,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current

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
            .padding(KeynearLayout.CardPaddingLarge),
        verticalArrangement = Arrangement.spacedBy(ActionIconSize),
    ) {
        Column(verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm)) {
            Text(
                text = "Computer name",
                color = palette.textSecondary,
                fontSize = KeynearTypeSizes.Description,
                fontWeight = FontWeight.SemiBold,
            )
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(ContentGap),
            ) {
                RecessedInput(
                    value = renameDraft,
                    onValueChange = { onRenameDraftChange(it) },
                    placeholder = "Computer name",
                    modifier = Modifier.weight(1f),
                )
                ActionButton(
                    label = "Save",
                    primary = true,
                    enabled = renameDraft.trim().isNotEmpty(),
                    minHeight = ControlMinHeight,
                    onClick = { actions.onRenameDevice(device.deviceId, renameDraft) },
                )
            }
        }

        Row(
            modifier = Modifier.fillMaxWidth(),
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
                    text = "Auto-accept files",
                    color = palette.textPrimary,
                    fontSize = KeynearTypeSizes.Body,
                    fontWeight = FontWeight.Bold,
                )
                Text(
                    text = "Save files from ${device.displayName} without asking",
                    color = palette.textMuted,
                    fontSize = KeynearTypeSizes.Caption,
                )
            }
            KeynearSwitch(
                checked = device.autoAcceptFiles,
                onCheckedChange = { enabled ->
                    actions.onToggleAutoAccept(device.deviceId, enabled)
                },
            )
        }

        Column(
            modifier = Modifier.padding(top = KeynearSpacing.Xs),
            verticalArrangement = Arrangement.spacedBy(RowIconSize),
        ) {
            MetadataLine(label = "Key fingerprint", value = device.publicKeyFingerprint)
            MetadataLine(
                label = "Bluetooth address",
                value = device.lastBleAddress.ifBlank { "Unavailable" },
            )
            MetadataLine(
                label = "Local Wi-Fi endpoint",
                value = device.lastLanAddress.ifBlank { "Standby" },
            )
        }
    }
}
}
@Composable
private fun MetadataLine(label: String, value: String) {
    val palette = LocalKeynearPalette.current
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = label,
            color = palette.textMuted,
            fontSize = KeynearTypeSizes.Description,
        )
        Text(
            text = value,
            color = palette.textSecondary,
            fontSize = KeynearTypeSizes.Caption,
            fontWeight = FontWeight.SemiBold,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}
