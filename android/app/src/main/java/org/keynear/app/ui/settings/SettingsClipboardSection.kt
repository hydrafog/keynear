package org.keynear.app.ui.settings

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
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
import org.keynear.app.ui.model.DaemonConfigUi
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.designsystem.ActionButton
import org.keynear.designsystem.IconChevronDown
import org.keynear.designsystem.IconChevronUp
import org.keynear.designsystem.IconClose
import org.keynear.designsystem.KeynearDimensions
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearMotion
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

private val ChipMinHeight = 38.dp
private val AddButtonMinHeight = 50.dp

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun ClipboardSettingsSection(
    config: DaemonConfigUi,
    actions: KeynearActions,
) {
    var mimeExpanded by rememberSaveable { mutableStateOf(false) }
    var mimeInput by rememberSaveable { mutableStateOf("") }
    val palette = LocalKeynearPalette.current

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
                    IconChevronUp(modifier = Modifier.size(KeynearDimensions.Dp20), tint = palette.textMuted)
                } else {
                    IconChevronDown(modifier = Modifier.size(KeynearDimensions.Dp20), tint = palette.textMuted)
                }
            }
        }

        AnimatedVisibility(
            visible = mimeExpanded,
            enter = expandVertically(
                tween(durationMillis = KeynearMotion.MODAL_DURATION_MS, easing = KeynearMotion.SpringEasing),
            ) + fadeIn(
                tween(durationMillis = KeynearMotion.FADE_IN_DURATION_MS, easing = KeynearMotion.FadeEasing),
            ),
            exit = shrinkVertically(
                tween(durationMillis = KeynearMotion.EXIT_DURATION_MS, easing = KeynearMotion.ExitEasing),
            ) + fadeOut(
                tween(durationMillis = KeynearMotion.EXIT_DURATION_MS, easing = KeynearMotion.ExitEasing),
            ),
        ) {
            ClipboardMimeEditor(
                ignoredMimeTypes = config.ignoredMimeTypes,
                mimeInput = mimeInput,
                onMimeInputChange = { mimeInput = it },
                onRemoveMime = { mime ->
                    actions.onUpdateConfig { it.copy(ignoredMimeTypes = it.ignoredMimeTypes.filterNot { item -> item == mime }) }
                },
                onAddMime = {
                    val nextMime = mimeInput.trim()
                    if (nextMime.isNotEmpty() && !config.ignoredMimeTypes.contains(nextMime)) {
                        actions.onUpdateConfig { it.copy(ignoredMimeTypes = it.ignoredMimeTypes + nextMime) }
                    }
                    mimeInput = ""
                },
            )
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun ClipboardMimeEditor(
    ignoredMimeTypes: List<String>,
    mimeInput: String,
    onMimeInputChange: (String) -> Unit,
    onRemoveMime: (String) -> Unit,
    onAddMime: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(
                start = KeynearSpacing.Lg,
                end = KeynearSpacing.Lg,
                top = KeynearSpacing.Xs,
                bottom = KeynearSpacing.Lg,
            ),
        verticalArrangement = Arrangement.spacedBy(KeynearDimensions.Dp14),
    ) {
        FlowRow(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
        ) {
            ignoredMimeTypes.forEach { mime ->
                TactileSurface(
                    shape = KeynearShapes.Xl,
                    containerColor = palette.surfaceTonal,
                    tone = TactileTone.TONAL,
                    cornerRadius = KeynearRadii.Xl,
                ) {
                    Row(
                        modifier = Modifier
                            .heightIn(min = ChipMinHeight)
                            .padding(horizontal = KeynearDimensions.Dp14, vertical = KeynearLayout.CompactGap),
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
                            containerColor = palette.surface,
                            tone = TactileTone.SURFACE,
                            cornerRadius = KeynearRadii.Lg,
                            pressScale = KeynearPressScale.PRESS_95,
                            onClick = { onRemoveMime(mime) },
                        ) {
                            Box(
                                modifier = Modifier.padding(KeynearSpacing.Xs),
                                contentAlignment = Alignment.Center,
                            ) {
                                IconClose(
                                    modifier = Modifier.size(KeynearSpacing.Lg),
                                    tint = palette.textSecondary,
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
            horizontalArrangement = Arrangement.spacedBy(KeynearLayout.ContentGap),
        ) {
            RecessedInput(
                value = mimeInput,
                onValueChange = onMimeInputChange,
                placeholder = "application/x-secret",
                modifier = Modifier.weight(1f),
            )
            ActionButton(
                label = "Add",
                primary = true,
                enabled = mimeInput.trim().isNotEmpty(),
                minHeight = AddButtonMinHeight,
                onClick = onAddMime,
            )
        }
    }
}
