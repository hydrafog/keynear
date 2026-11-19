package org.keynear.app.ui.settings

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
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
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.ui.model.DaemonConfigUi
import org.keynear.app.ui.model.PermissionsChecklistState
import org.keynear.designsystem.ActionButton
import org.keynear.designsystem.KeynearSwitch
import org.keynear.designsystem.RecessedInput
import org.keynear.designsystem.SettingRow
import org.keynear.designsystem.SettingsSection
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.designsystem.IconChevronDown
import org.keynear.designsystem.IconChevronRight
import org.keynear.designsystem.IconChevronUp
import org.keynear.designsystem.IconClose
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette

private val ActionMinHeight = 44.dp
private val ChipMinWidth = 50.dp
private val CompactGap = 6.dp
private val ContentGap = 10.dp
private val ControlIconSize = 20.dp
private val FieldHeight = 38.dp
private val RowIconSize = 14.dp
private val ValueTextSize = 19.sp


@OptIn(ExperimentalLayoutApi::class)
@Composable
fun SettingsScreen(
    config: DaemonConfigUi,
    permissions: PermissionsChecklistState,
    actions: KeynearActions,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    var mimeExpanded by rememberSaveable { mutableStateOf(false) }
    var mimeInput by rememberSaveable { mutableStateOf("") }

    Column(
        modifier = modifier
            .fillMaxSize()
            .background(palette.canvas)
            .verticalScroll(rememberScrollState())
            .padding(
                horizontal = KeynearLayout.ScreenPaddingHorizontal,
                vertical = KeynearLayout.ScreenPaddingVertical,
            ),
        verticalArrangement = Arrangement.spacedBy(KeynearLayout.SectionGap),
    ) {
        SettingsSection(title = "General & Storage") {
            SettingRow(
                title = "System permissions",
                subtitle = "Bluetooth, Wi-Fi, notifications, and battery access",
                onClick = actions.onOpenPermissions,
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
                ) {
                    Text(
                        text = if (permissions.missingPermissionsCount > 0) {
                            "${permissions.missingPermissionsCount} missing"
                        } else {
                            "Allowed"
                        },
                        color = if (permissions.missingPermissionsCount > 0) palette.error else palette.textMuted,
                        fontSize = KeynearTypeSizes.Description,
                        fontWeight = FontWeight.Bold,
                    )
                    IconChevronRight(modifier = Modifier.size(ControlIconSize), tint = palette.textMuted)
                }
            }
            SettingRow(
                title = "Save folder",
                subtitle = "Where received files are saved",
                onClick = actions.onPickSaveFolder,
            ) {
                TactileSurface(
                    shape = KeynearShapes.Xl,
                    containerColor = palette.surfaceTonal,
                    tone = TactileTone.TONAL,
                    cornerRadius = KeynearRadii.Xl,
                    onClick = actions.onPickSaveFolder,
                ) {
                    Box(
                        modifier = Modifier
                            .heightIn(min = ActionMinHeight)
                            .padding(horizontal = KeynearSpacing.Lg),
                        contentAlignment = Alignment.Center,
                    ) {
                        Text(
                            text = config.incomingDirectory,
                            color = palette.textPrimary,
                            fontSize = KeynearTypeSizes.Caption,
                            fontWeight = FontWeight.Bold,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }
            }
            SettingRow(
                title = "Auto-accept from paired devices",
                subtitle = "Save files from paired devices without asking",
            ) {
                KeynearSwitch(
                    checked = config.autoAcceptFromPaired,
                    onCheckedChange = { checked ->
                        actions.onUpdateConfig { it.copy(autoAcceptFromPaired = checked) }
                    },
                )
            }
        }

        SettingsSection(title = "Clipboard") {
            SettingRow(
                title = "Clipboard sync",
                subtitle = "Keep clipboard text in sync",
            ) {
                KeynearSwitch(
                    checked = config.clipboardSyncEnabled,
                    onCheckedChange = { checked ->
                        actions.onUpdateConfig { it.copy(clipboardSyncEnabled = checked) }
                    },
                )
            }
            SettingRow(
                title = "Sync clipboard images",
                subtitle = "Include images in clipboard sync",
            ) {
                KeynearSwitch(
                    checked = config.clipboardImagesEnabled,
                    onCheckedChange = { checked ->
                        actions.onUpdateConfig { it.copy(clipboardImagesEnabled = checked) }
                    },
                )
            }
            SettingRow(
                title = "2FA code pass-through",
                subtitle = "Pass verification codes to the computer",
            ) {
                KeynearSwitch(
                    checked = config.otpPassthroughEnabled,
                    onCheckedChange = { checked ->
                        actions.onUpdateConfig { it.copy(otpPassthroughEnabled = checked) }
                    },
                )
            }
            SettingRow(
                title = "Ignored clipboard formats",
                subtitle = "Formats that are never shared",
                onClick = { mimeExpanded = !mimeExpanded },
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
                ) {
                    Text(
                        text = config.ignoredMimeTypes.size.toString(),
                        color = palette.textMuted,
                        fontSize = KeynearTypeSizes.Description,
                        fontWeight = FontWeight.Bold,
                    )
                    if (mimeExpanded) {
                        IconChevronUp(modifier = Modifier.size(ControlIconSize), tint = palette.textMuted)
                    } else {
                        IconChevronDown(modifier = Modifier.size(ControlIconSize), tint = palette.textMuted)
                    }
                }
            }

            if (mimeExpanded) {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(start = KeynearSpacing.Lg, end = KeynearSpacing.Lg, top = KeynearSpacing.Xs, bottom = KeynearSpacing.Lg),
                    verticalArrangement = Arrangement.spacedBy(RowIconSize),
                ) {
                    FlowRow(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
                        verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
                    ) {
                        config.ignoredMimeTypes.forEach { mime ->
                            TactileSurface(
                                shape = KeynearShapes.Xl,
                                containerColor = palette.surfaceTonal,
                                tone = TactileTone.TONAL,
                                cornerRadius = KeynearRadii.Xl,
                            ) {
                                Row(
                                    modifier = Modifier
                                        .heightIn(min = FieldHeight)
                                        .padding(horizontal = RowIconSize, vertical = CompactGap),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
                                ) {
                                    Text(
                                        text = mime,
                                        color = palette.textPrimary,
                                        fontSize = KeynearTypeSizes.Caption,
                                        fontWeight = FontWeight.SemiBold,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                    )
                                    TactileSurface(
                                        shape = KeynearShapes.Lg,
                                        containerColor = palette.errorAction,
                                        tone = TactileTone.TONAL,
                                        cornerRadius = KeynearRadii.Lg,
                                        onClick = {
                                            actions.onUpdateConfig {
                                                it.copy(
                                                    ignoredMimeTypes = it.ignoredMimeTypes.filterNot { item ->
                                                        item == mime
                                                    },
                                                )
                                            }
                                        },
                                    ) {
                                        Box(
                                            modifier = Modifier.padding(KeynearSpacing.Xs),
                                            contentAlignment = Alignment.Center,
                                        ) {
                                            IconClose(
                                                modifier = Modifier.size(KeynearSpacing.Lg),
                                                tint = palette.error,
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(ContentGap),
                    ) {
                        RecessedInput(
                            value = mimeInput,
                            onValueChange = { mimeInput = it },
                            placeholder = "application/x-secret",
                            modifier = Modifier.weight(1f),
                        )
                        ActionButton(
                            label = "Add",
                            primary = false,
                            enabled = mimeInput.trim().isNotEmpty(),
                            minHeight = ChipMinWidth,
                            onClick = {
                                val nextMime = mimeInput.trim()
                                if (nextMime.isNotEmpty() && !config.ignoredMimeTypes.contains(nextMime)) {
                                    actions.onUpdateConfig {
                                        it.copy(ignoredMimeTypes = it.ignoredMimeTypes + nextMime)
                                    }
                                }
                                mimeInput = ""
                            },
                        )
                    }
                }
            }
        }

        SettingsSection(title = "Audio & Notifications") {
            SettingRow(
                title = "Smart audio handoff",
                subtitle = "Move audio between devices",
            ) {
                KeynearSwitch(
                    checked = config.audioHandoffEnabled,
                    onCheckedChange = { checked ->
                        actions.onUpdateConfig { it.copy(audioHandoffEnabled = checked) }
                    },
                )
            }
            SettingRow(
                title = "Completion notifications",
                subtitle = "Notify when files finish saving",
            ) {
                KeynearSwitch(
                    checked = config.notifyAutoAcceptComplete,
                    onCheckedChange = { checked ->
                        actions.onUpdateConfig { it.copy(notifyAutoAcceptComplete = checked) }
                    },
                )
            }
        }

        SettingsSection(title = "Appearance") {
            SettingRow(
                title = "Dark theme",
                subtitle = "Switch the app color theme",
            ) {
                KeynearSwitch(
                    checked = config.darkTheme,
                    onCheckedChange = { checked ->
                        actions.onUpdateConfig { it.copy(darkTheme = checked) }
                    },
                )
            }
        }

        Spacer(modifier = Modifier.height(KeynearSpacing.Sm))
    }
}

@Composable
fun PermissionsScreen(
    permissions: PermissionsChecklistState,
    actions: KeynearActions,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    val rows = listOf(
        Triple("Nearby devices", "Bluetooth and Wi-Fi discovery for nearby Linux desktops.", Pair(permissions.nearbyGranted, actions.onRequestNearbyOnboardingPermissions)),
        Triple("Local network access", "Direct peer-to-peer transfers over your local Wi-Fi.", Pair(permissions.localNetworkGranted, actions.onRequestLocalNetworkPermission)),
        Triple("Companion device presence", "Wakes the background service automatically when a paired PC is near.", Pair(permissions.companionAssociated, actions.onOpenAppSettings)),
        Triple("Notifications", "Shows transfer requests, 6-digit pairing codes, and live progress.", Pair(permissions.notificationsGranted, actions.onRequestNotificationsPermission)),
        Triple("Unrestricted battery", "Prevents Android from interrupting large transfers with the screen off.", Pair(permissions.batteryOptimizationIgnored, actions.onRequestBatteryOptimization)),
    )

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
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(ContentGap),
                verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
            ) {
                rows.forEach { (title, sub, actionPair) ->
                    val (granted, onClick) = actionPair
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(RowIconSize),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
                    ) {
                        Column(
                            modifier = Modifier.weight(1f),
                            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Xs),
                        ) {
                            Text(
                                text = title,
                                color = palette.textPrimary,
                                fontSize = KeynearTypeSizes.Body,
                                fontWeight = FontWeight.Bold,
                            )
                            Text(
                                text = sub,
                                color = palette.textMuted,
                                fontSize = KeynearTypeSizes.Caption,
                                lineHeight = ValueTextSize,
                            )
                        }
                        ActionButton(
                            label = if (granted) "Allowed" else "Allow",
                            primary = !granted,
                            minHeight = KeynearSpacing.MinTouchTarget,
                            onClick = onClick,
                        )
                    }
                }
            }
        }
    }
}
