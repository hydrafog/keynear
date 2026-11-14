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
import org.keynear.app.core.CoreSnapshot
import org.keynear.app.core.DeviceRoleUi
import org.keynear.app.core.SendItemKind
import org.keynear.app.core.formatBytes
import org.keynear.app.ui.components.ActionButton
import org.keynear.app.ui.components.RecessedInput
import org.keynear.app.ui.components.SectionHeading
import org.keynear.app.ui.components.SelectionTile
import org.keynear.app.ui.components.TactileSurface
import org.keynear.app.ui.components.TactileTone
import org.keynear.app.ui.components.tactileCushion
import org.keynear.app.ui.icons.IconClipboard
import org.keynear.app.ui.icons.IconClose
import org.keynear.app.ui.icons.IconFile
import org.keynear.app.ui.icons.IconFolder
import org.keynear.app.ui.icons.IconLaptop
import org.keynear.app.ui.icons.IconPlus
import org.keynear.app.ui.icons.IconSend
import org.keynear.app.ui.icons.IconSmartphone
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.app.ui.theme.KeynearLayout
import org.keynear.app.ui.theme.KeynearRadii
import org.keynear.app.ui.theme.KeynearShapes
import org.keynear.app.ui.theme.LocalKeynearPalette

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
                        IconFile(modifier = Modifier.size(32.dp), tint = tint)
                    }
                    SelectionTile(
                        label = "Folder",
                        onClick = actions.onPickFolder,
                        modifier = Modifier.weight(1f),
                    ) { tint ->
                        IconFolder(modifier = Modifier.size(32.dp), tint = tint)
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
                        IconClipboard(modifier = Modifier.size(32.dp), tint = tint)
                    }
                    SelectionTile(
                        label = "Text",
                        active = textComposerOpen,
                        onClick = { textComposerOpen = !textComposerOpen },
                        modifier = Modifier.weight(1f),
                    ) { tint ->
                        IconPlus(modifier = Modifier.size(32.dp), tint = tint)
                    }
                }
            }
        }

        if (textComposerOpen) {
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
                        .padding(KeynearLayout.CardPadding),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    RecessedInput(
                        value = textDraft,
                        onValueChange = { textDraft = it },
                        placeholder = "Paste URL or note",
                        modifier = Modifier.weight(1f),
                    )
                    ActionButton(
                        label = "Add",
                        primary = true,
                        enabled = textDraft.isNotBlank(),
                        minHeight = 52.dp,
                        onClick = {
                            actions.onStageTextSnippet(textDraft)
                            textDraft = ""
                            textComposerOpen = false
                        },
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
                modifier = Modifier.padding(KeynearLayout.CardPadding),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(38.dp)
                        .padding(horizontal = 2.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = "${queue.size} item(s) (${formatBytes(totalBytes)})",
                        color = palette.textPrimary,
                        fontSize = 14.sp,
                        fontWeight = FontWeight.Bold,
                        modifier = Modifier.weight(1f),
                    )
                    if (queue.isNotEmpty()) {
                        ActionButton(
                            label = "Clear all",
                            onClick = actions.onClearQueue,
                            errorTonal = true,
                            minHeight = 38.dp,
                        )
                    }
                }

                if (queue.isEmpty()) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(56.dp)
                            .clip(KeynearShapes.Xl)
                            .background(palette.trackWell)
                            .tactileCushion(
                                tone = TactileTone.INSET,
                                isDark = palette.isDark,
                                cornerRadius = KeynearRadii.Xl,
                            )
                            .padding(horizontal = 14.dp, vertical = 8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        IconFile(
                            modifier = Modifier.size(20.dp),
                            tint = palette.textMuted,
                        )
                        Text(
                            text = "No items selected",
                            color = palette.textMuted,
                            fontSize = 14.sp,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                } else {
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
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
                                        .height(56.dp)
                                        .padding(horizontal = 14.dp, vertical = 8.dp),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(10.dp),
                                ) {
                                    when (item.kind) {
                                        SendItemKind.FOLDER -> IconFolder(
                                            modifier = Modifier.size(20.dp),
                                            tint = palette.textSecondary,
                                        )
                                        SendItemKind.CLIPBOARD -> IconClipboard(
                                            modifier = Modifier.size(20.dp),
                                            tint = palette.textSecondary,
                                        )
                                        SendItemKind.TEXT -> IconPlus(
                                            modifier = Modifier.size(20.dp),
                                            tint = palette.textSecondary,
                                        )
                                        SendItemKind.FILE -> IconFile(
                                            modifier = Modifier.size(20.dp),
                                            tint = palette.textSecondary,
                                        )
                                    }
                                    Text(
                                        text = item.name,
                                        color = palette.textPrimary,
                                        fontSize = 14.sp,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                        modifier = Modifier.weight(1f),
                                    )
                                    Text(
                                        text = formatBytes(item.sizeBytes),
                                        color = palette.textMuted,
                                        fontSize = 12.sp,
                                    )
                                    TactileSurface(
                                        modifier = Modifier.size(40.dp),
                                        shape = KeynearShapes.Xl,
                                        containerColor = palette.errorAction,
                                        tone = TactileTone.TONAL,
                                        cornerRadius = KeynearRadii.Xl,
                                        onClick = { actions.onRemoveQueueItem(item.id) },
                                    ) {
                                        Box(
                                            modifier = Modifier.size(40.dp),
                                            contentAlignment = Alignment.Center,
                                        ) {
                                            IconClose(
                                                modifier = Modifier.size(20.dp),
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
                                .heightIn(min = 68.dp)
                                .padding(horizontal = 14.dp, vertical = 12.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(12.dp),
                        ) {
                            Box(
                                modifier = Modifier
                                    .size(48.dp)
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
                                    modifier = Modifier.size(24.dp),
                                    tint = palette.textMuted,
                                )
                            }
                            Text(
                                text = "No paired devices",
                                color = palette.textMuted,
                                fontSize = 14.sp,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                        }
                    } else {
                        snapshot.pairedDevices.forEachIndexed { idx, device ->
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .heightIn(min = 68.dp)
                                    .padding(horizontal = 14.dp, vertical = 12.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(12.dp),
                            ) {
                                TactileSurface(
                                    modifier = Modifier.size(48.dp),
                                    shape = KeynearShapes.Xl,
                                    containerColor = palette.surfaceTonal,
                                    tone = TactileTone.TONAL,
                                    cornerRadius = KeynearRadii.Xl,
                                ) {
                                    Box(
                                        modifier = Modifier.size(48.dp),
                                        contentAlignment = Alignment.Center,
                                    ) {
                                        val iconTint = palette.textSecondary
                                        if (device.role == DeviceRoleUi.DESKTOP) {
                                            IconLaptop(modifier = Modifier.size(24.dp), tint = iconTint)
                                        } else {
                                            IconSmartphone(modifier = Modifier.size(24.dp), tint = iconTint)
                                        }
                                    }
                                }
                                Column(
                                    modifier = Modifier.weight(1f),
                                    verticalArrangement = Arrangement.spacedBy(2.dp),
                                ) {
                                    Text(
                                        text = device.displayName,
                                        color = palette.textPrimary,
                                        fontSize = 16.sp,
                                        fontWeight = FontWeight.Bold,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                    )
                                    Text(
                                        text = if (device.isOnline) "Connected via Wi-Fi" else "Bluetooth standby",
                                        color = palette.textMuted,
                                        fontSize = 12.sp,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                    )
                                }
                                ActionButton(
                                    label = "Send",
                                    primary = idx == 0,
                                    enabled = queue.isNotEmpty(),
                                    minHeight = 48.dp,
                                    leadingIcon = { tint ->
                                        IconSend(modifier = Modifier.size(16.dp), tint = tint)
                                    },
                                    onClick = { actions.onSendQueueToPeer(device) },
                                )
                            }
                        }
                    }
                }
            }
        }

        Spacer(modifier = Modifier.height(8.dp))
    }
}
