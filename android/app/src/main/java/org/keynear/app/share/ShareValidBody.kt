package org.keynear.app.share

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.ui.model.PairedDeviceUi
import org.keynear.app.ui.model.TransferUi
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette

private val SheetCornerTop = 28.dp
private val SheetElevation = 16.dp
private val HandleWidth = 36.dp
private val HandleHeight = 4.dp
private val CloseBadgeSize = 36.dp
private val CardCornerRadius = 20.dp
private val DeviceGridMaxHeight = 320.dp
private val DeviceTileCornerRadius = 18.dp
private val DeviceTileMinHeight = 96.dp
private val DeviceIconContainerSize = 44.dp
private val DeviceIconContainerRadius = 14.dp
private val PillCorner = 999.dp
private val SheetGap = 14.dp
private val SectionGap = 10.dp
private val HeaderTextSize = 19.sp
private val EmptyHeaderSize = 15.sp
private val SmallHeaderSize = 13.sp
private val CaptionLetterSpacing = 0.8.sp
private val MicroLabelSize = 11.sp

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
        Surface(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(KeynearRadii.Xl),
            color = palette.errorTonal,
        ) {
            Text(
                text = msg,
                fontSize = SmallHeaderSize,
                color = palette.onDangerTonal,
                modifier = Modifier.padding(KeynearSpacing.Md),
            )
        }
    }

    if (sentTransfer != null) {
        Surface(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(CardCornerRadius),
            color = palette.loadingTonal,
        ) {
            Column(
                modifier = Modifier.padding(KeynearSpacing.Lg),
                verticalArrangement = Arrangement.spacedBy(SectionGap),
            ) {
                Text(
                    text = "Sending ${sentTransfer.summaryName} to ${sentTransfer.peerDisplayName}",
                    fontSize = KeynearTypeSizes.Description,
                    fontWeight = FontWeight.Bold,
                    color = palette.onLoadingTonal,
                )
                Row(horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm)) {
                    Button(
                        onClick = onOpenMain,
                        modifier = Modifier
                            .weight(1f)
                            .defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                        shape = RoundedCornerShape(PillCorner),
                        colors = ButtonDefaults.buttonColors(
                            containerColor = palette.loading,
                            contentColor = palette.onAccent,
                        ),
                    ) {
                        Text("Open Keynear", fontWeight = FontWeight.Bold)
                    }
                    OutlinedButton(
                        onClick = onClose,
                        modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                        shape = RoundedCornerShape(PillCorner),
                    ) {
                        Text("Done", color = palette.onLoadingTonal)
                    }
                }
            }
        }
        return
    }

    Text(
        text = "DIRECT SHARE VIA KEYNEAR",
        fontSize = MicroLabelSize,
        fontWeight = FontWeight.Bold,
        letterSpacing = CaptionLetterSpacing,
        color = palette.textMuted,
    )

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
            Surface(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(CardCornerRadius),
                color = palette.surfaceTonal,
            ) {
                Column(
                    modifier = Modifier.padding(KeynearSpacing.Lg),
                    verticalArrangement = Arrangement.spacedBy(SectionGap),
                ) {
                    Text(
                        text = "No paired devices yet",
                        fontSize = EmptyHeaderSize,
                        fontWeight = FontWeight.Bold,
                        color = palette.textPrimary,
                    )
                    Text(
                        text = "Open Keynear to pair a nearby device before sharing.",
                        fontSize = SmallHeaderSize,
                        color = palette.textMuted,
                    )
                    Button(
                        onClick = onOpenMain,
                        modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                        shape = RoundedCornerShape(PillCorner),
                        colors = ButtonDefaults.buttonColors(
                            containerColor = palette.loading,
                            contentColor = palette.onAccent,
                        ),
                    ) {
                        Text("Open Keynear", fontWeight = FontWeight.Bold)
                    }
                }
            }
        }
        else -> {
            val rows = pairedDevices.chunked(2)
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .heightIn(max = DeviceGridMaxHeight),
                verticalArrangement = Arrangement.spacedBy(SectionGap),
            ) {
                rows.forEach { pair ->
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(SectionGap),
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
