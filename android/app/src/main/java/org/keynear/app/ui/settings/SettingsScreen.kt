package org.keynear.app.ui.settings

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
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
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.core.DaemonConfigUi
import org.keynear.app.core.PermissionsChecklistState
import org.keynear.app.ui.components.ActionButton
import org.keynear.app.ui.components.KeynearSwitch
import org.keynear.app.ui.components.RecessedInput
import org.keynear.app.ui.components.SettingRow
import org.keynear.app.ui.components.SettingsSection
import org.keynear.app.ui.components.TactileSurface
import org.keynear.app.ui.icons.IconChevronRight
import org.keynear.app.ui.icons.IconClose
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.app.ui.theme.LocalKeynearPalette

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
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        SettingsSection(title = "Receiving") {
            SettingRow(
                title = "Save folder",
                subtitle = config.incomingDirectory,
                onClick = actions.onPickSaveFolder,
            ) {
                Surface(
                    shape = RoundedCornerShape(10.dp),
                    color = palette.loadingTonal,
                ) {
                    Text(
                        text = "Change",
                        color = palette.onLoadingTonal,
                        fontSize = 12.sp,
                        fontWeight = FontWeight.SemiBold,
                        modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
                    )
                }
            }
            SettingRow(
                title = "Auto-accept from paired",
                subtitle = "Accept files from paired computers automatically",
            ) {
                KeynearSwitch(
                    checked = config.autoAcceptFromPaired,
                    onCheckedChange = { checked ->
                        actions.onUpdateConfig { it.copy(autoAcceptFromPaired = checked) }
                    },
                )
            }
        }

        SettingsSection(title = "Clipboard & continuity") {
            SettingRow(
                title = "Sync clipboard with paired",
                subtitle = "Relay copied text across your active session",
            ) {
                KeynearSwitch(
                    checked = config.clipboardSyncEnabled,
                    onCheckedChange = { checked ->
                        actions.onUpdateConfig { it.copy(clipboardSyncEnabled = checked) }
                    },
                )
            }
            SettingRow(
                title = "Include images over Wi-Fi",
                subtitle = "Relay clipboard screenshots while connected over LAN",
            ) {
                KeynearSwitch(
                    checked = config.clipboardImagesEnabled,
                    onCheckedChange = { checked ->
                        actions.onUpdateConfig { it.copy(clipboardImagesEnabled = checked) }
                    },
                )
            }
            SettingRow(
                title = "Auto-clear SMS OTP codes",
                subtitle = "Remove synced verification codes after handoff",
            ) {
                KeynearSwitch(
                    checked = config.otpPassthroughEnabled,
                    onCheckedChange = { checked ->
                        actions.onUpdateConfig { it.copy(otpPassthroughEnabled = checked) }
                    },
                )
            }
            SettingRow(
                title = "Ignored MIME types",
                subtitle = "Sensitive clipboard formats never synced",
                onClick = { mimeExpanded = !mimeExpanded },
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Surface(
                        shape = CircleShape,
                        color = palette.loadingTonal,
                    ) {
                        Text(
                            text = config.ignoredMimeTypes.size.toString(),
                            color = palette.onLoadingTonal,
                            fontSize = 12.sp,
                            fontWeight = FontWeight.Bold,
                            modifier = Modifier.padding(horizontal = 10.dp, vertical = 3.dp),
                        )
                    }
                    IconChevronRight(tint = palette.textMuted)
                }
            }

            if (mimeExpanded) {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(top = 4.dp, bottom = 6.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    config.ignoredMimeTypes.forEach { mime ->
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .clip(RoundedCornerShape(12.dp))
                                .background(palette.surfaceTonal)
                                .padding(horizontal = 12.dp, vertical = 8.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            Text(
                                text = mime,
                                color = palette.textSecondary,
                                fontSize = 12.sp,
                                fontWeight = FontWeight.Medium,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                                modifier = Modifier.weight(1f),
                            )
                            Surface(
                                modifier = Modifier.size(34.dp),
                                shape = RoundedCornerShape(10.dp),
                                color = palette.errorAction,
                                onClick = {
                                    actions.onUpdateConfig {
                                        it.copy(ignoredMimeTypes = it.ignoredMimeTypes.filterNot { item -> item == mime })
                                    }
                                },
                            ) {
                                Box(contentAlignment = Alignment.Center) {
                                    IconClose(modifier = Modifier.size(14.dp), tint = palette.onDangerAction)
                                }
                            }
                        }
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        RecessedInput(
                            value = mimeInput,
                            onValueChange = { mimeInput = it },
                            placeholder = "application/x-custom-secret",
                            modifier = Modifier.weight(1f),
                        )
                        ActionButton(
                            label = "Add",
                            primary = true,
                            enabled = mimeInput.trim().isNotEmpty(),
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

        SettingsSection(title = "Audio & notifications") {
            SettingRow(
                title = "Audio call handoff",
                subtitle = "Alert connected computer on incoming phone calls",
            ) {
                KeynearSwitch(
                    checked = config.audioHandoffEnabled,
                    onCheckedChange = { checked ->
                        actions.onUpdateConfig { it.copy(audioHandoffEnabled = checked) }
                    },
                )
            }
            SettingRow(
                title = "Notify when auto-accepted",
                subtitle = "Show silent banner when trusted transfers complete",
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
                subtitle = "Use dark slate palette instead of daytime canvas",
            ) {
                KeynearSwitch(
                    checked = config.darkTheme,
                    onCheckedChange = { checked ->
                        actions.onUpdateConfig { it.copy(darkTheme = checked) }
                    },
                )
            }
        }

        SettingsSection(title = "System") {
            SettingRow(
                title = "System permissions",
                subtitle = if (permissions.missingPermissionsCount == 0) {
                    "All recommended permissions granted"
                } else {
                    "${permissions.missingPermissionsCount} permission(s) require attention"
                },
                onClick = actions.onOpenPermissions,
            ) {
                IconChevronRight(tint = palette.textMuted)
            }
        }

        Spacer(modifier = Modifier.height(8.dp))
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
        Triple("Nearby devices", "Bluetooth scan, advertise, and Wi-Fi Direct", Pair(permissions.nearbyGranted, actions.onRequestNearbyOnboardingPermissions)),
        Triple("Local network access", "Discover paired devices on this Wi-Fi LAN", Pair(permissions.localNetworkGranted, actions.onRequestLocalNetworkPermission)),
        Triple("Companion device", "Wake Keynear on trusted Bluetooth sightings", Pair(permissions.companionAssociated, actions.onOpenAppSettings)),
        Triple("Notifications", "Show incoming transfer consent alerts", Pair(permissions.notificationsGranted, actions.onRequestNotificationsPermission)),
        Triple("Battery optimization", "Keep background transfers reliable", Pair(permissions.batteryOptimizationIgnored, actions.onRequestBatteryOptimization)),
    )

    Column(
        modifier = modifier
            .fillMaxSize()
            .background(palette.canvas)
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        TactileSurface(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(16.dp),
            containerColor = palette.surface,
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(14.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                rows.forEach { (title, sub, actionPair) ->
                    val (granted, onClick) = actionPair
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .heightIn(min = 48.dp)
                            .clickable(onClick = onClick),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = title,
                                color = palette.textPrimary,
                                fontSize = 15.sp,
                                fontWeight = FontWeight.SemiBold,
                            )
                            Text(
                                text = sub,
                                color = palette.textMuted,
                                fontSize = 12.sp,
                            )
                        }
                        ActionButton(
                            label = if (granted) "Allowed" else "Allow",
                            primary = !granted,
                            onClick = onClick,
                        )
                    }
                }
            }
        }
    }
}

