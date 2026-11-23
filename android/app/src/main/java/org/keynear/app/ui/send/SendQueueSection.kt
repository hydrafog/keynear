package org.keynear.app.ui.send

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import org.keynear.app.ui.model.SendItemKind
import org.keynear.app.ui.model.SendQueueItemUi
import org.keynear.app.ui.model.formatBytes
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.designsystem.ActionButton
import org.keynear.designsystem.IconClipboard
import org.keynear.designsystem.IconClose
import org.keynear.designsystem.IconFile
import org.keynear.designsystem.IconFolder
import org.keynear.designsystem.IconPlus
import org.keynear.designsystem.KeynearDimensions
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearPressScale
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone

private val QueueRowMinHeight = 54.dp

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
            verticalArrangement = Arrangement.spacedBy(KeynearLayout.ContentGap),
        ) {
            if (queue.isEmpty()) {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = KeynearSpacing.Md),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
                ) {
                    TactileSurface(
                        modifier = Modifier.size(KeynearLayout.EmptyBadgeSize),
                        shape = KeynearShapes.Xl,
                        containerColor = palette.loadingTonal,
                        tone = TactileTone.ACCENT_TONAL,
                        cornerRadius = KeynearRadii.Xl,
                        contentAlignment = Alignment.Center,
                    ) {
                        IconFile(
                            modifier = Modifier.size(KeynearDimensions.Dp20),
                            tint = palette.onLoadingTonal,
                        )
                    }
                    Text(
                        text = "Nothing queued yet",
                        color = palette.textPrimary,
                        fontSize = KeynearTypeSizes.Description,
                        fontWeight = FontWeight.Bold,
                    )
                    Text(
                        text = "Pick a file, folder, clipboard, or note above",
                        color = palette.textMuted,
                        fontSize = KeynearTypeSizes.Caption,
                    )
                }
            } else {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = KeynearLayout.TextSubtitleGap),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    val countLabel = if (queue.size == 1) "1 item" else "${queue.size} items"
                    Text(
                        text = "$countLabel · ${formatBytes(totalBytes)}",
                        color = palette.textPrimary,
                        fontSize = KeynearTypeSizes.Description,
                        fontWeight = FontWeight.Bold,
                        modifier = Modifier.weight(1f),
                    )
                    ActionButton(
                        label = "Clear all",
                        onClick = actions.onClearQueue,
                        tonalSurface = true,
                        minHeight = KeynearLayout.EmptyBadgeSize,
                    )
                }

                Column(verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm)) {
                    queue.forEach { item ->
                        SendQueueItemRow(item = item, onRemove = { actions.onRemoveQueueItem(item.id) })
                    }
                }
            }
        }
    }
}

@Composable
private fun SendQueueItemRow(
    item: SendQueueItemUi,
    onRemove: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
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
                .heightIn(min = QueueRowMinHeight)
                .padding(horizontal = KeynearDimensions.Dp14, vertical = KeynearSpacing.Sm),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(KeynearLayout.ContentGap),
        ) {
            when (item.kind) {
                SendItemKind.FOLDER -> IconFolder(modifier = Modifier.size(KeynearDimensions.Dp20), tint = palette.loadingInk)
                SendItemKind.CLIPBOARD -> IconClipboard(modifier = Modifier.size(KeynearDimensions.Dp20), tint = palette.loadingInk)
                SendItemKind.TEXT -> IconPlus(modifier = Modifier.size(KeynearDimensions.Dp20), tint = palette.loadingInk)
                SendItemKind.FILE -> IconFile(modifier = Modifier.size(KeynearDimensions.Dp20), tint = palette.loadingInk)
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
                modifier = Modifier.size(KeynearLayout.EmptyBadgeSize),
                shape = KeynearShapes.Xl,
                containerColor = palette.surface,
                tone = TactileTone.SURFACE,
                cornerRadius = KeynearRadii.Xl,
                pressScale = KeynearPressScale.PRESS_95,
                contentAlignment = Alignment.Center,
                onClick = onRemove,
            ) {
                IconClose(
                    modifier = Modifier.size(KeynearDimensions.Dp20),
                    tint = palette.textSecondary,
                )
            }
        }
    }
}
