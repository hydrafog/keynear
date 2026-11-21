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

        SendQueueSection(queue = queue, totalBytes = totalBytes, actions = actions)

        SendTargetsSection(devices = snapshot.pairedDevices, queue = queue, actions = actions)

        Spacer(modifier = Modifier.height(KeynearSpacing.Sm))
    }
}
