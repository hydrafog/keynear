package org.keynear.app.ui.settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
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
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.ui.model.DaemonConfigUi
import org.keynear.app.ui.model.PermissionsChecklistState
import org.keynear.designsystem.KeynearSwitch
import org.keynear.designsystem.SettingRow
import org.keynear.designsystem.SettingsSection
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.designsystem.IconChevronRight
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette

private val ActionMinHeight = 44.dp
private val ChipMinWidth = 50.dp
private val CompactGap = 6.dp
private val ControlIconSize = 20.dp
private val FieldHeight = 38.dp

@Composable
internal fun GeneralStorageSection(
    config: DaemonConfigUi,
    permissions: PermissionsChecklistState,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current

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
}
@Composable
internal fun AudioNotificationSection(
    config: DaemonConfigUi,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current

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
}
@Composable
internal fun AppearanceSection(
    config: DaemonConfigUi,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current

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
}
