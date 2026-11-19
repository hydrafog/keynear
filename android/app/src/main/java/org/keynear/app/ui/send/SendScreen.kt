package org.keynear.app.ui.send

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
import androidx.compose.ui.window.Dialog
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
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.DeviceRoleUi
import org.keynear.app.ui.model.SendItemKind
import org.keynear.app.ui.model.formatBytes
import org.keynear.designsystem.ActionButton
import org.keynear.designsystem.RecessedInput
import org.keynear.designsystem.SectionHeading
import org.keynear.designsystem.SelectionTile
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.designsystem.tactileCushion
import org.keynear.designsystem.IconClipboard
import org.keynear.designsystem.IconClose
import org.keynear.designsystem.IconFile
import org.keynear.designsystem.IconFolder
import org.keynear.designsystem.IconLaptop
import org.keynear.designsystem.IconPlus
import org.keynear.designsystem.IconSend
import org.keynear.designsystem.IconSmartphone
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette

private val ActionIconSize = 20.dp
private val ButtonHeight = 40.dp
private val ContentGap = 10.dp
private val QueueHeaderHeight = 38.dp
private val RowIconSize = 14.dp
private val RowMinHeight = 56.dp
private val TargetRowMinHeight = 68.dp
private val TextSubtitleGap = 2.dp


@Composable
fun SendScreen(
    snapshot: CoreSnapshot,
    actions: KeynearActions,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    var textComposerOpen by rememberSaveable { mutableStateOf(false) }
    var textDraft by rememberSaveable { mutableStateOf("") }
    val queue = snapshot.sendQueue
    val totalBytes = queue.sumOf { it.sizeBytes }

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
        Column(verticalArrangement = Arrangement.spacedBy(KeynearLayout.SectionHeadingBottom)) {
            SectionHeading(title = "Selection")

            Column(verticalArrangement = Arrangement.spacedBy(KeynearLayout.GridGap)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(KeynearLayout.GridGap),
                ) {
                    SelectionTile(
                        label = "File",
                        onClick = actions.onPickFiles,
                        modifier = Modifier.weight(1f),
                    ) { tint ->
                        IconFile(modifier = Modifier.size(KeynearSpacing.TwoXl), tint = tint)
                    }
                    SelectionTile(
                        label = "Folder",
                        onClick = actions.onPickFolder,
                        modifier = Modifier.weight(1f),
                    ) { tint ->
                        IconFolder(modifier = Modifier.size(KeynearSpacing.TwoXl), tint = tint)
                    }
                }
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(KeynearLayout.GridGap),
                ) {
                    SelectionTile(
                        label = "Clipboard",
                        onClick = actions.onStageClipboard,
                        modifier = Modifier.weight(1f),
                    ) { tint ->
                        IconClipboard(modifier = Modifier.size(KeynearSpacing.TwoXl), tint = tint)
                    }
                    SelectionTile(
                        label = "Text",
                        onClick = { textComposerOpen = true },
                        modifier = Modifier.weight(1f),
                    ) { tint ->
                        IconPlus(modifier = Modifier.size(KeynearSpacing.TwoXl), tint = tint)
                    }
                }
            }
        }

        if (textComposerOpen) {
            Dialog(onDismissRequest = { textComposerOpen = false }) {
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
                        verticalArrangement = Arrangement.spacedBy(ContentGap),
                    ) {
                        Text(
                            text = "Add text",
                            color = palette.textPrimary,
                            fontSize = KeynearTypeSizes.Heading,
                            fontWeight = FontWeight.Bold,
                        )
                        RecessedInput(
                            value = textDraft,
                            onValueChange = { textDraft = it },
                            placeholder = "Paste URL or note",
                            modifier = Modifier.fillMaxWidth(),
                        )
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(ContentGap),
                        ) {
                            ActionButton(
                                label = "Cancel",
                                onClick = {
                                    textDraft = ""
                                    textComposerOpen = false
                                },
                                modifier = Modifier.weight(1f),
                            )
                            ActionButton(
                                label = "Add",
                                primary = true,
                                enabled = textDraft.isNotBlank(),
                                onClick = {
                                    actions.onStageTextSnippet(textDraft)
                                    textDraft = ""
                                    textComposerOpen = false
                                },
                                modifier = Modifier.weight(1f),
                            )
                        }
                    }
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
                modifier = Modifier.padding(KeynearLayout.CardPadding),
                verticalArrangement = Arrangement.spacedBy(ContentGap),
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(QueueHeaderHeight)
                        .padding(horizontal = TextSubtitleGap),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = "${queue.size} item(s) (${formatBytes(totalBytes)})",
                        color = palette.textPrimary,
                        fontSize = KeynearTypeSizes.Description,
                        fontWeight = FontWeight.Bold,
                        modifier = Modifier.weight(1f),
                    )
                    if (queue.isNotEmpty()) {
                        ActionButton(
                            label = "Clear all",
                            onClick = actions.onClearQueue,
                            errorTonal = true,
                            minHeight = QueueHeaderHeight,
                        )
                    }
                }

                if (queue.isEmpty()) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(RowMinHeight)
                            .clip(KeynearShapes.Xl)
                            .background(palette.trackWell)
                            .tactileCushion(
                                tone = TactileTone.INSET,
                                isDark = palette.isDark,
                                cornerRadius = KeynearRadii.Xl,
                            )
                            .padding(horizontal = RowIconSize, vertical = KeynearSpacing.Sm),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(ContentGap),
                    ) {
                        IconFile(
                            modifier = Modifier.size(ActionIconSize),
                            tint = palette.textMuted,
                        )
                        Text(
                            text = "No items selected",
                            color = palette.textMuted,
                            fontSize = KeynearTypeSizes.Description,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                } else {
                    Column(verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm)) {
                        queue.forEach { item ->
                            TactileSurface(
                                modifier = Modifier.fillMaxWidth(),
                                shape = KeynearShapes.Xl,
                                containerColor = palette.surfaceTonal,
                                tone = TactileTone.TONAL,
                                cornerRadius = KeynearRadii.Xl,
                            ) {
                                Row(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .height(RowMinHeight)
                                        .padding(horizontal = RowIconSize, vertical = KeynearSpacing.Sm),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(ContentGap),
                                ) {
                                    when (item.kind) {
                                        SendItemKind.FOLDER -> IconFolder(
                                            modifier = Modifier.size(ActionIconSize),
                                            tint = palette.textSecondary,
                                        )
                                        SendItemKind.CLIPBOARD -> IconClipboard(
                                            modifier = Modifier.size(ActionIconSize),
                                            tint = palette.textSecondary,
                                        )
                                        SendItemKind.TEXT -> IconPlus(
                                            modifier = Modifier.size(ActionIconSize),
                                            tint = palette.textSecondary,
                                        )
                                        SendItemKind.FILE -> IconFile(
                                            modifier = Modifier.size(ActionIconSize),
                                            tint = palette.textSecondary,
                                        )
                                    }
                                    Text(
                                        text = item.name,
                                        color = palette.textPrimary,
                                        fontSize = KeynearTypeSizes.Description,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                        modifier = Modifier.weight(1f),
                                    )
                                    Text(
                                        text = formatBytes(item.sizeBytes),
                                        color = palette.textMuted,
                                        fontSize = KeynearTypeSizes.Caption,
                                    )
                                    TactileSurface(
                                        modifier = Modifier.size(ButtonHeight),
                                        shape = KeynearShapes.Xl,
                                        containerColor = palette.errorAction,
                                        tone = TactileTone.TONAL,
                                        cornerRadius = KeynearRadii.Xl,
                                        onClick = { actions.onRemoveQueueItem(item.id) },
                                    ) {
                                        Box(
                                            modifier = Modifier.size(ButtonHeight),
                                            contentAlignment = Alignment.Center,
                                        ) {
                                            IconClose(
                                                modifier = Modifier.size(ActionIconSize),
                                                tint = palette.error,
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

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
                    if (snapshot.pairedDevices.isEmpty()) {
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .heightIn(min = TargetRowMinHeight)
                                .padding(horizontal = RowIconSize, vertical = KeynearSpacing.Md),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
                        ) {
                            Box(
                                modifier = Modifier
                                    .size(KeynearSpacing.MinTouchTarget)
                                    .clip(KeynearShapes.Xl)
                                    .background(palette.trackWell)
                                    .tactileCushion(
                                        tone = TactileTone.INSET,
                                        isDark = palette.isDark,
                                        cornerRadius = KeynearRadii.Xl,
                                    ),
                                contentAlignment = Alignment.Center,
                            ) {
                                IconLaptop(
                                    modifier = Modifier.size(KeynearSpacing.Xl),
                                    tint = palette.textMuted,
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
                        snapshot.pairedDevices.forEachIndexed { idx, device ->
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .heightIn(min = TargetRowMinHeight)
                                    .padding(horizontal = RowIconSize, vertical = KeynearSpacing.Md),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
                            ) {
                                TactileSurface(
                                    modifier = Modifier.size(KeynearSpacing.MinTouchTarget),
                                    shape = KeynearShapes.Xl,
                                    containerColor = palette.surfaceTonal,
                                    tone = TactileTone.TONAL,
                                    cornerRadius = KeynearRadii.Xl,
                                ) {
                                    Box(
                                        modifier = Modifier.size(KeynearSpacing.MinTouchTarget),
                                        contentAlignment = Alignment.Center,
                                    ) {
                                        val iconTint = palette.textSecondary
                                        if (device.role == DeviceRoleUi.DESKTOP) {
                                            IconLaptop(modifier = Modifier.size(KeynearSpacing.Xl), tint = iconTint)
                                        } else {
                                            IconSmartphone(modifier = Modifier.size(KeynearSpacing.Xl), tint = iconTint)
                                        }
                                    }
                                }
                                Column(
                                    modifier = Modifier.weight(1f),
                                    verticalArrangement = Arrangement.spacedBy(TextSubtitleGap),
                                ) {
                                    Text(
                                        text = device.displayName,
                                        color = palette.textPrimary,
                                        fontSize = KeynearTypeSizes.Body,
                                        fontWeight = FontWeight.Bold,
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
                                    primary = idx == 0,
                                    enabled = queue.isNotEmpty(),
                                    minHeight = KeynearSpacing.MinTouchTarget,
                                    leadingIcon = { tint ->
                                        IconSend(modifier = Modifier.size(KeynearSpacing.Lg), tint = tint)
                                    },
                                    onClick = { actions.onSendQueueToPeer(device) },
                                )
                            }
                        }
                    }
                }
            }
        }

        Spacer(modifier = Modifier.height(KeynearSpacing.Sm))
    }
}
