package org.keynear.app.ui.send

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import org.keynear.app.ui.model.SendItemKind
import org.keynear.app.ui.model.SendQueueItemUi
import org.keynear.app.ui.model.formatBytes
import org.keynear.designsystem.ActionButton
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.designsystem.tactileCushion
import org.keynear.designsystem.IconClipboard
import org.keynear.designsystem.IconClose
import org.keynear.designsystem.IconFile
import org.keynear.designsystem.IconFolder
import org.keynear.designsystem.IconPlus
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

@Composable
internal fun SendQueueSection(
    queue: List<SendQueueItemUi>,
    totalBytes: Long,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current

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
}
