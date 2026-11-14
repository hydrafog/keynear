package org.keynear.app.ui.devices

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.core.DeviceRoleUi
import org.keynear.app.core.PairedDeviceUi
import org.keynear.app.ui.components.ActionButton
import org.keynear.app.ui.components.KeynearSwitch
import org.keynear.app.ui.components.RecessedInput
import org.keynear.app.ui.components.SettingRow
import org.keynear.app.ui.components.SettingsSection
import org.keynear.app.ui.components.TactileSurface
import org.keynear.app.ui.icons.IconLaptop
import org.keynear.app.ui.icons.IconSmartphone
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.app.ui.theme.LocalKeynearPalette

@Composable
fun DeviceDetailScreen(
    device: PairedDeviceUi?,
    confirmingUnpair: Boolean,
    actions: KeynearActions,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    if (device == null) {
        Box(
            modifier = modifier
                .fillMaxSize()
                .background(palette.canvas),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = "Device no longer paired.",
                color = palette.textMuted,
                fontSize = 14.sp,
            )
        }
        return
    }

    var renameDraft by remember(device.deviceId, device.displayName) {
        mutableStateOf(device.displayName)
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .background(palette.canvas)
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        TactileSurface(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(16.dp),
            containerColor = palette.surface,
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(16.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(14.dp),
            ) {
                Box(
                    modifier = Modifier
                        .size(56.dp)
                        .clip(RoundedCornerShape(14.dp))
                        .background(if (device.isOnline) palette.loadingTonal else palette.surfaceTonal),
                    contentAlignment = Alignment.Center,
                ) {
                    val iconTint = if (device.isOnline) palette.onLoadingTonal else palette.textMuted
                    if (device.role == DeviceRoleUi.DESKTOP) {
                        IconLaptop(modifier = Modifier.size(26.dp), tint = iconTint)
                    } else {
                        IconSmartphone(modifier = Modifier.size(26.dp), tint = iconTint)
                    }
                }
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = device.displayName,
                        color = palette.textPrimary,
                        fontSize = 18.sp,
                        fontWeight = FontWeight.Bold,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Text(
                        text = "${device.role.label()} · ${if (device.isOnline) "Connected via Wi-Fi" else "Bluetooth standby"} (${device.pairedAt})",
                        color = palette.textMuted,
                        fontSize = 12.sp,
                    )
                }
            }
        }

        SettingsSection(title = "Device name") {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(vertical = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                RecessedInput(
                    value = renameDraft,
                    onValueChange = { renameDraft = it },
                    placeholder = "Device name",
                    modifier = Modifier.weight(1f),
                )
                ActionButton(
                    label = "Save",
                    primary = true,
                    enabled = renameDraft.trim().isNotEmpty(),
                    onClick = { actions.onRenameDevice(device.deviceId, renameDraft) },
                )
            }
        }

        SettingsSection(title = "Transfers") {
            SettingRow(
                title = "Auto-accept files",
                subtitle = "Receive files from ${device.displayName} without prompting",
            ) {
                KeynearSwitch(
                    checked = device.autoAcceptFiles,
                    onCheckedChange = { enabled ->
                        actions.onToggleAutoAccept(device.deviceId, enabled)
                    },
                )
            }
        }

        SettingsSection(title = "Connection") {
            MetadataLine(label = "Public key", value = device.publicKeyFingerprint)
            MetadataLine(
                label = "Bluetooth address",
                value = device.lastBleAddress.ifBlank { "Unavailable" },
            )
            MetadataLine(
                label = "LAN endpoint",
                value = device.lastLanAddress.ifBlank { "Standby" },
            )
            Spacer(modifier = Modifier.height(4.dp))
            ActionButton(
                label = "Reset Bluetooth bond",
                onClick = { actions.onResetBleBond(device.deviceId) },
                modifier = Modifier.fillMaxWidth(),
            )
        }

        if (!confirmingUnpair) {
            ActionButton(
                label = "Remove device",
                danger = true,
                onClick = { actions.onRequestUnpair(device.deviceId) },
                modifier = Modifier.fillMaxWidth(),
            )
        } else {
            TactileSurface(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(16.dp),
                containerColor = palette.errorTonal,
                shadowElevation = 0.dp,
            ) {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(14.dp),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    Text(
                        text = "Remove ${device.displayName}? You will need to confirm a new 6-digit code on both devices to pair again.",
                        color = palette.onDangerTonal,
                        fontSize = 13.sp,
                        fontWeight = FontWeight.Medium,
                    )
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        ActionButton(
                            label = "Cancel",
                            onClick = actions.onCancelUnpair,
                            modifier = Modifier.weight(1f),
                        )
                        ActionButton(
                            label = "Remove",
                            danger = true,
                            onClick = { actions.onConfirmUnpair(device.deviceId) },
                            modifier = Modifier.weight(1f),
                        )
                    }
                }
            }
        }

        Spacer(modifier = Modifier.height(8.dp))
    }
}

@Composable
private fun MetadataLine(label: String, value: String) {
    val palette = LocalKeynearPalette.current
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 4.dp, vertical = 4.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = label,
            color = palette.textMuted,
            fontSize = 12.sp,
            fontWeight = FontWeight.Medium,
        )
        Text(
            text = value,
            color = palette.textSecondary,
            fontSize = 12.sp,
            fontWeight = FontWeight.SemiBold,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

