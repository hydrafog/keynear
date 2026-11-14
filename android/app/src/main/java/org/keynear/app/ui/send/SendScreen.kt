package org.keynear.app.ui.send

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
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
import org.keynear.app.core.CoreSnapshot
import org.keynear.app.core.DeviceRoleUi
import org.keynear.app.core.SendItemKind
import org.keynear.app.core.formatBytes
import org.keynear.app.ui.components.ActionButton
import org.keynear.app.ui.components.RecessedInput
import org.keynear.app.ui.components.SelectionTile
import org.keynear.app.ui.components.TactileSurface
import org.keynear.app.ui.icons.IconClipboard
import org.keynear.app.ui.icons.IconClose
import org.keynear.app.ui.icons.IconFile
import org.keynear.app.ui.icons.IconFolder
import org.keynear.app.ui.icons.IconLaptop
import org.keynear.app.ui.icons.IconSmartphone
import org.keynear.app.ui.icons.IconType
import org.keynear.app.ui.navigation.KeynearActions
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

    Column(
        modifier = modifier
            .fillMaxSize()
            .background(palette.canvas)
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Text(
            text = "SELECTION",
            color = palette.textMuted,
            fontSize = 12.sp,
            fontWeight = FontWeight.SemiBold,
            letterSpacing = 0.8.sp,
            modifier = Modifier.padding(horizontal = 4.dp),
        )

        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
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
                horizontalArrangement = Arrangement.spacedBy(8.dp),
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
                    IconType(modifier = Modifier.size(32.dp), tint = tint)
                }
            }
        }

        if (textComposerOpen) {
            TactileSurface(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(16.dp),
                containerColor = palette.surface,
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(12.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
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
                        onClick = {
                            actions.onStageTextSnippet(textDraft)
                            textDraft = ""
                            textComposerOpen = false
                        },
                    )
                }
            }
        }

        val queue = snapshot.sendQueue
        if (queue.isNotEmpty()) {
            val totalBytes = queue.sumOf { it.sizeBytes }
            TactileSurface(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(16.dp),
                containerColor = palette.surface,
            ) {
                Column(
                    modifier = Modifier.padding(14.dp),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            text = "${queue.size} item(s) (${formatBytes(totalBytes)})",
                            color = palette.textPrimary,
                            fontSize = 15.sp,
                            fontWeight = FontWeight.Bold,
                            modifier = Modifier.weight(1f),
                        )
                        ActionButton(
                            label = "Clear all",
                            onClick = actions.onClearQueue,
                            errorTonal = true,
                        )
                    }

                    queue.forEach { item ->
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .heightIn(min = 54.dp)
                                .clip(RoundedCornerShape(12.dp))
                                .background(palette.surfaceTonal)
                                .padding(horizontal = 10.dp, vertical = 8.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(10.dp),
                        ) {
                            Box(
                                modifier = Modifier
                                    .size(36.dp)
                                    .clip(RoundedCornerShape(10.dp))
                                    .background(palette.loadingTonal),
                                contentAlignment = Alignment.Center,
                            ) {
                                when (item.kind) {
                                    SendItemKind.FOLDER -> IconFolder(modifier = Modifier.size(18.dp), tint = palette.onLoadingTonal)
                                    SendItemKind.CLIPBOARD -> IconClipboard(modifier = Modifier.size(18.dp), tint = palette.onLoadingTonal)
                                    SendItemKind.TEXT -> IconType(modifier = Modifier.size(18.dp), tint = palette.onLoadingTonal)
                                    SendItemKind.FILE -> IconFile(modifier = Modifier.size(18.dp), tint = palette.onLoadingTonal)
                                }
                            }
                            Column(modifier = Modifier.weight(1f)) {
                                Text(
                                    text = item.name,
                                    color = palette.textPrimary,
                                    fontSize = 14.sp,
                                    fontWeight = FontWeight.SemiBold,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                )
                                Text(
                                    text = formatBytes(item.sizeBytes),
                                    color = palette.textMuted,
                                    fontSize = 11.sp,
                                )
                            }
                            Surface(
                                modifier = Modifier.size(40.dp),
                                shape = RoundedCornerShape(10.dp),
                                color = palette.errorAction,
                                onClick = { actions.onRemoveQueueItem(item.id) },
                            ) {
                                Box(contentAlignment = Alignment.Center) {
                                    IconClose(modifier = Modifier.size(16.dp), tint = palette.onDangerAction)
                                }
                            }
                        }
                    }
                }
            }
        }

        Text(
            text = "SEND TO",
            color = palette.textMuted,
            fontSize = 12.sp,
            fontWeight = FontWeight.SemiBold,
            letterSpacing = 0.8.sp,
            modifier = Modifier.padding(horizontal = 4.dp),
        )

        TactileSurface(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(16.dp),
            containerColor = palette.surface,
        ) {
            if (snapshot.pairedDevices.isEmpty()) {
                Text(
                    text = "No paired devices yet. Switch Receive visibility to Everyone nearby to pair first.",
                    color = palette.textMuted,
                    fontSize = 13.sp,
                    modifier = Modifier.padding(16.dp),
                )
            } else {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(PaddingValues(12.dp)),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    snapshot.pairedDevices.forEachIndexed { idx, device ->
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(12.dp),
                        ) {
                            Box(
                                modifier = Modifier
                                    .size(48.dp)
                                    .clip(RoundedCornerShape(14.dp))
                                    .background(palette.surfaceTonal),
                                contentAlignment = Alignment.Center,
                            ) {
                                val iconTint = if (device.isOnline) palette.loading else palette.textMuted
                                if (device.role == DeviceRoleUi.DESKTOP) {
                                    IconLaptop(modifier = Modifier.size(22.dp), tint = iconTint)
                                } else {
                                    IconSmartphone(modifier = Modifier.size(22.dp), tint = iconTint)
                                }
                            }
                            Column(modifier = Modifier.weight(1f)) {
                                Text(
                                    text = device.displayName,
                                    color = palette.textPrimary,
                                    fontSize = 15.sp,
                                    fontWeight = FontWeight.SemiBold,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                )
                                Text(
                                    text = "${device.role.label()} · ${if (device.isOnline) "Wi-Fi direct" else "Bluetooth wake"}",
                                    color = palette.textMuted,
                                    fontSize = 12.sp,
                                )
                            }
                            ActionButton(
                                label = "Send",
                                primary = idx == 0,
                                enabled = queue.isNotEmpty(),
                                onClick = { actions.onSendQueueToPeer(device) },
                            )
                        }
                    }
                }
            }
        }
        Spacer(modifier = Modifier.size(8.dp))
    }
}

