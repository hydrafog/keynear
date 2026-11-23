package org.keynear.app.ui.send

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.window.Dialog
import org.keynear.app.ui.components.ActiveTransfersStrip
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.TransferDirectionUi
import org.keynear.app.ui.model.TransferStateUi
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.designsystem.ActionButton
import org.keynear.designsystem.IconClipboard
import org.keynear.designsystem.IconFile
import org.keynear.designsystem.IconFolder
import org.keynear.designsystem.IconPlus
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearMotion
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette
import org.keynear.designsystem.RecessedInput
import org.keynear.designsystem.SectionHeading
import org.keynear.designsystem.SelectionTile
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone

private const val MODAL_BASE_SCALE = 0.96f

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
            AddTextDialog(
                textDraft = textDraft,
                onTextDraftChange = { textDraft = it },
                onDismiss = {
                    textDraft = ""
                    textComposerOpen = false
                },
                onAdd = {
                    actions.onStageTextSnippet(textDraft)
                    textDraft = ""
                    textComposerOpen = false
                },
            )
        }

        SendQueueSection(queue = queue, totalBytes = totalBytes, actions = actions)

        val outgoingTransfers = snapshot.activeTransfers.filter {
            it.direction == TransferDirectionUi.SEND &&
                (it.state == TransferStateUi.ACTIVE || it.state == TransferStateUi.PENDING_CONSENT)
        }
        AnimatedVisibility(
            visible = outgoingTransfers.isNotEmpty(),
            enter = fadeIn(tween(KeynearMotion.SHORT_DURATION_MS)) + expandVertically(),
            exit = fadeOut(tween(KeynearMotion.SHORT_DURATION_MS)) + shrinkVertically(),
        ) {
            ActiveTransfersStrip(
                transfers = outgoingTransfers,
                onAccept = actions.onAcceptTransfer,
                onDecline = actions.onDeclineTransfer,
                onCancel = actions.onCancelTransfer,
            )
        }

        SendTargetsSection(devices = snapshot.pairedDevices, queue = queue, actions = actions)

        Spacer(modifier = Modifier.height(KeynearSpacing.Sm))
    }
}

@Composable
private fun AddTextDialog(
    textDraft: String,
    onTextDraftChange: (String) -> Unit,
    onDismiss: () -> Unit,
    onAdd: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val density = LocalDensity.current
    val enterAnim = remember { Animatable(0f) }
    LaunchedEffect(Unit) {
        enterAnim.animateTo(
            targetValue = 1f,
            animationSpec = tween(durationMillis = KeynearMotion.MODAL_DURATION_MS, easing = KeynearMotion.SpringEasing),
        )
    }
    Dialog(onDismissRequest = onDismiss) {
        TactileSurface(
            modifier = Modifier.graphicsLayer {
                val p = enterAnim.value
                val scale = MODAL_BASE_SCALE + (1f - MODAL_BASE_SCALE) * p
                alpha = p
                scaleX = scale
                scaleY = scale
                translationY = with(density) { KeynearSpacing.Sm.toPx() } * (1f - p)
            },
            shape = KeynearShapes.Card,
            containerColor = palette.surface,
            tone = TactileTone.SURFACE,
            cornerRadius = KeynearRadii.TwoXl,
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(KeynearLayout.CardPadding),
                verticalArrangement = Arrangement.spacedBy(KeynearLayout.ContentGap),
            ) {
                Text(
                    text = "Add text",
                    color = palette.textPrimary,
                    fontSize = KeynearTypeSizes.Heading,
                    fontWeight = FontWeight.Bold,
                )
                RecessedInput(
                    value = textDraft,
                    onValueChange = onTextDraftChange,
                    placeholder = "Paste URL or note",
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
                        label = "Add",
                        primary = true,
                        enabled = textDraft.isNotBlank(),
                        onClick = onAdd,
                        modifier = Modifier.weight(1f),
                    )
                }
            }
        }
    }
}
