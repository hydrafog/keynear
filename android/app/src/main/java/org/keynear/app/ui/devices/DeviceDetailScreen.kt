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
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.core.DeviceRoleUi
import org.keynear.app.core.PairedDeviceUi
import org.keynear.app.ui.components.ActionButton
import org.keynear.app.ui.components.KeynearSwitch
import org.keynear.app.ui.components.RecessedInput
import org.keynear.app.ui.components.TactileSurface
import org.keynear.app.ui.components.TactileTone
import org.keynear.app.ui.icons.IconLaptop
import org.keynear.app.ui.icons.IconRefreshCw
import org.keynear.app.ui.icons.IconSmartphone
import org.keynear.app.ui.icons.IconTrash2
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.app.ui.theme.KeynearLayout
import org.keynear.app.ui.theme.KeynearRadii
import org.keynear.app.ui.theme.KeynearShapes
import org.keynear.app.ui.theme.KeynearSpacing
import org.keynear.app.ui.theme.KeynearTypeSizes
import org.keynear.app.ui.theme.LocalKeynearPalette

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
                fontSize = KeynearTypeSizes.Description,
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
            .padding(
                horizontal = KeynearLayout.ScreenPaddingHorizontal,
                vertical = KeynearLayout.ScreenPaddingVertical,
            ),
        verticalArrangement = Arrangement.spacedBy(KeynearLayout.CompactSectionGap),
    ) {
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
                    containerColor = if (device.isOnline) palette.loadingTonal else palette.surfaceTonal,
                    tone = TactileTone.TONAL,
                    cornerRadius = KeynearRadii.Xl,
                ) {
                    Box(modifier = Modifier.size(HeaderIconSize), contentAlignment = Alignment.Center) {
                        val iconTint = if (device.isOnline) palette.onLoadingTonal else palette.textMuted
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
                            onValueChange = { renameDraft = it },
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
                            text = "Remove ${device.displayName}? You will need to confirm a new 6-digit code on both devices to pair again.",
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

        Spacer(modifier = Modifier.height(KeynearSpacing.Sm))
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
