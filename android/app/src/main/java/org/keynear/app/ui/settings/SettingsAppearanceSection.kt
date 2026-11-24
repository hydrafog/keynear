package org.keynear.app.ui.settings

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.window.Dialog
import org.keynear.app.ui.model.DaemonConfigUi
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.designsystem.ActionButton
import org.keynear.designsystem.IconCheck
import org.keynear.designsystem.IconPalette
import org.keynear.designsystem.KeynearAccents
import org.keynear.designsystem.KeynearDimensions
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearPressScale
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearSwitch
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette
import org.keynear.designsystem.RecessedInput
import org.keynear.designsystem.SettingRow
import org.keynear.designsystem.SettingsSection
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.designsystem.normalizeAccentHex
import org.keynear.designsystem.parseAccentHexOrNull
import org.keynear.designsystem.tactileCushion
import org.keynear.designsystem.tactilePressScale

@Composable
internal fun AppearanceSection(
    config: DaemonConfigUi,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current
    var customPickerOpen by rememberSaveable { mutableStateOf(false) }
    val activeHex = normalizeAccentHex(config.accentColorHex)
    val isCustomAccent = KeynearAccents.Presets.none { it.hex.equals(activeHex, ignoreCase = true) }

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

        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(
                    horizontal = KeynearLayout.RowHorizontalPadding,
                    vertical = KeynearDimensions.Dp14,
                ),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
        ) {
            Column(verticalArrangement = Arrangement.spacedBy(KeynearLayout.TextSubtitleGap)) {
                Text(
                    text = "Accent color",
                    color = palette.textPrimary,
                    fontSize = KeynearTypeSizes.Body,
                    fontWeight = FontWeight.SemiBold,
                )
                Text(
                    text = "Choose the app accent color",
                    color = palette.textMuted,
                    fontSize = KeynearTypeSizes.Caption,
                )
            }

            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(KeynearLayout.ContentGap),
            ) {
                KeynearAccents.Presets.forEach { preset ->
                    val selected = activeHex.equals(preset.hex, ignoreCase = true)
                    AccentColorTile(
                        label = "${preset.name} accent color",
                        selected = selected,
                        tileColor = preset.color,
                        isCustomPlaceholder = false,
                        onClick = {
                            actions.onUpdateConfig { it.copy(accentColorHex = preset.hex) }
                        },
                    )
                }
                val customTileColor = if (isCustomAccent) {
                    parseAccentHexOrNull(activeHex) ?: palette.switchTrackOff
                } else {
                    palette.switchTrackOff
                }
                AccentColorTile(
                    label = "Custom accent color",
                    selected = isCustomAccent,
                    tileColor = customTileColor,
                    isCustomPlaceholder = !isCustomAccent,
                    onClick = { customPickerOpen = true },
                )
            }
        }
    }

    if (customPickerOpen) {
        CustomAccentColorDialog(
            initialHex = activeHex,
            onDismiss = { customPickerOpen = false },
            onSelectHex = { chosenHex ->
                actions.onUpdateConfig { it.copy(accentColorHex = chosenHex) }
                customPickerOpen = false
            },
        )
    }
}

@Composable
private fun AccentColorTile(
    label: String,
    selected: Boolean,
    tileColor: Color,
    isCustomPlaceholder: Boolean,
    onClick: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val interactionSource = remember { MutableInteractionSource() }
    val ringModifier = if (selected) {
        Modifier
            .border(KeynearDimensions.Dp2, palette.focusRing, KeynearShapes.Xl)
            .padding(KeynearDimensions.Dp2)
    } else {
        Modifier
    }
    Box(
        modifier = Modifier
            .size(KeynearDimensions.Dp48)
            .tactilePressScale(interactionSource, KeynearPressScale.PRESS_95)
            .then(ringModifier)
            .clip(KeynearShapes.Xl)
            .background(tileColor)
            .let { m ->
                if (isCustomPlaceholder) {
                    m.tactileCushion(TactileTone.INSET, palette.isDark, KeynearRadii.Xl)
                } else {
                    m
                }
            }
            .semantics {
                contentDescription = label
                this.selected = selected
            }
            .clickable(
                interactionSource = interactionSource,
                indication = null,
                role = Role.Button,
                onClick = onClick,
            ),
        contentAlignment = Alignment.Center,
    ) {
        when {
            selected -> IconCheck(
                modifier = Modifier.size(KeynearDimensions.Dp20),
                tint = palette.onAccent,
            )
            isCustomPlaceholder -> IconPalette(
                modifier = Modifier.size(KeynearDimensions.Dp20),
                tint = palette.loadingInk,
            )
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun CustomAccentColorDialog(
    initialHex: String,
    onDismiss: () -> Unit,
    onSelectHex: (String) -> Unit,
) {
    val palette = LocalKeynearPalette.current
    var draftHex by rememberSaveable(initialHex) { mutableStateOf(initialHex) }
    val parsedDraft = parseAccentHexOrNull(draftHex)

    Dialog(onDismissRequest = onDismiss) {
        TactileSurface(
            shape = KeynearShapes.Card,
            containerColor = palette.surface,
            tone = TactileTone.SURFACE,
            cornerRadius = KeynearRadii.TwoXl,
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(KeynearLayout.CardPadding),
                verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
            ) {
                Text(
                    text = "Custom accent color",
                    color = palette.textPrimary,
                    fontSize = KeynearTypeSizes.Heading,
                    fontWeight = FontWeight.Bold,
                )
                FlowRow(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(KeynearLayout.ContentGap),
                    verticalArrangement = Arrangement.spacedBy(KeynearLayout.ContentGap),
                ) {
                    KeynearAccents.CustomPickerSwatches.forEach { rawCode ->
                        val swatchHex = normalizeAccentHex(rawCode)
                        val swatchColor = parseAccentHexOrNull(swatchHex) ?: palette.accent
                        val isMatch = normalizeAccentHex(draftHex).equals(swatchHex, ignoreCase = true) && parsedDraft != null
                        AccentColorTile(
                            label = "Custom color $swatchHex",
                            selected = isMatch,
                            tileColor = swatchColor,
                            isCustomPlaceholder = false,
                            onClick = { draftHex = swatchHex },
                        )
                    }
                }
                RecessedInput(
                    value = draftHex,
                    onValueChange = { draftHex = it },
                    placeholder = KeynearAccents.DEFAULT_HEX,
                    modifier = Modifier.fillMaxWidth(),
                )
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(KeynearLayout.ContentGap),
                ) {
                    ActionButton(
                        label = "Cancel",
                        tonalSurface = true,
                        onClick = onDismiss,
                        modifier = Modifier.weight(1f),
                    )
                    ActionButton(
                        label = "Apply",
                        primary = true,
                        enabled = parsedDraft != null,
                        onClick = { onSelectHex(normalizeAccentHex(draftHex)) },
                        modifier = Modifier.weight(1f),
                    )
                }
            }
        }
    }
}

