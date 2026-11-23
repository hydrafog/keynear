package org.keynear.app.share

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import org.keynear.app.ui.model.PairedDeviceUi
import org.keynear.app.ui.model.TransferUi
import org.keynear.designsystem.ActionButton
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette
import org.keynear.designsystem.SectionHeading
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone

private val DeviceGridMaxHeight = 320.dp

@Composable
internal fun ShareValidBody(
    sendError: String?,
    sentTransfer: TransferUi?,
    pairedDevices: List<PairedDeviceUi>?,
    preselectedDeviceId: String?,
    sendingDeviceId: String?,
    onSelectPeer: (String) -> Unit,
    onOpenMain: () -> Unit,
    onClose: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    sendError?.let { msg ->
        TactileSurface(
            modifier = Modifier.fillMaxWidth(),
            shape = KeynearShapes.Xl,
            containerColor = palette.errorTonal,
            tone = TactileTone.DANGER_TONAL,
            cornerRadius = KeynearRadii.Xl,
        ) {
            Text(
                text = msg,
                fontSize = KeynearTypeSizes.Description,
                color = palette.onDangerTonal,
                modifier = Modifier.padding(KeynearSpacing.Md),
            )
        }
    }

    if (sentTransfer != null) {
        TactileSurface(
            modifier = Modifier.fillMaxWidth(),
            shape = KeynearShapes.Card,
            containerColor = palette.loadingTonal,
            tone = TactileTone.ACCENT_TONAL,
            cornerRadius = KeynearRadii.TwoXl,
        ) {
            Column(
                modifier = Modifier.padding(KeynearSpacing.Lg),
                verticalArrangement = Arrangement.spacedBy(KeynearLayout.ContentGap),
            ) {
                Text(
                    text = "Sending ${sentTransfer.summaryName} to ${sentTransfer.peerDisplayName}",
                    fontSize = KeynearTypeSizes.Description,
                    fontWeight = FontWeight.Bold,
                    color = palette.onLoadingTonal,
                )
                Row(horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm)) {
                    ActionButton(
                        label = "Open Keynear",
                        primary = true,
                        onClick = onOpenMain,
                        modifier = Modifier.weight(1f),
                    )
                    ActionButton(
                        label = "Done",
                        tonalSurface = true,
                        onClick = onClose,
                    )
                }
            }
        }
        return
    }

    SectionHeading(title = "Direct Share via Keynear")

    when {
        pairedDevices == null -> {
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(vertical = KeynearSpacing.Xl),
                contentAlignment = Alignment.Center,
            ) {
                CircularProgressIndicator(color = palette.loading)
            }
        }
        pairedDevices.isEmpty() -> {
            TactileSurface(
                modifier = Modifier.fillMaxWidth(),
                shape = KeynearShapes.Card,
                containerColor = palette.surfaceTonal,
                tone = TactileTone.TONAL,
                cornerRadius = KeynearRadii.TwoXl,
            ) {
                Column(
                    modifier = Modifier.padding(KeynearSpacing.Lg),
                    verticalArrangement = Arrangement.spacedBy(KeynearLayout.ContentGap),
                ) {
                    Text(
                        text = "No paired devices yet",
                        fontSize = KeynearTypeSizes.Body,
                        fontWeight = FontWeight.Bold,
                        color = palette.textPrimary,
                    )
                    Text(
                        text = "Open Keynear to pair a nearby device before sharing.",
                        fontSize = KeynearTypeSizes.Description,
                        color = palette.textMuted,
                    )
                    ActionButton(
                        label = "Open Keynear",
                        primary = true,
                        onClick = onOpenMain,
                    )
                }
            }
        }
        else -> {
            val rows = pairedDevices.chunked(2)
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .heightIn(max = DeviceGridMaxHeight),
                verticalArrangement = Arrangement.spacedBy(KeynearLayout.ContentGap),
            ) {
                rows.forEach { pair ->
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(KeynearLayout.ContentGap),
                    ) {
                        pair.forEach { dev ->
                            ShareDeviceTile(
                                device = dev,
                                highlighted = dev.deviceId == preselectedDeviceId,
                                isSending = sendingDeviceId == dev.deviceId,
                                enabled = sendingDeviceId == null,
                                onClick = { onSelectPeer(dev.deviceId) },
                                modifier = Modifier.weight(1f),
                            )
                        }
                        if (pair.size == 1) {
                            Spacer(modifier = Modifier.weight(1f))
                        }
                    }
                }
            }
        }
    }
}
