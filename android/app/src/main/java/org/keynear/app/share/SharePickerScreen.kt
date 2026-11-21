package org.keynear.app.share

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.launch
import org.keynear.app.ui.model.PairedDeviceUi
import org.keynear.app.ui.model.ShareValidationResult
import org.keynear.app.ui.model.TransferUi
import org.keynear.designsystem.KeynearSpacing
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
internal fun SharePickerScreen(
    validation: ShareValidationResult,
    totalSizeBytes: Long,
    pairedDevices: List<PairedDeviceUi>?,
    preselectedDeviceId: String?,
    onSendToPeer: suspend (String) -> Result<TransferUi>,
    onOpenMain: () -> Unit,
    onClose: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val scope = rememberCoroutineScope()
    var sendingDeviceId by remember { mutableStateOf<String?>(null) }
    var sendError by remember { mutableStateOf<String?>(null) }
    var sentTransfer by remember { mutableStateOf<TransferUi?>(null) }
    val scrimInteraction = remember { MutableInteractionSource() }
    val sheetInteraction = remember { MutableInteractionSource() }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(palette.scrim)
            .clickable(interactionSource = scrimInteraction, indication = null, onClick = onClose),
    ) {
        Surface(
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .fillMaxWidth()
                .clickable(interactionSource = sheetInteraction, indication = null, onClick = {}),
            shape = RoundedCornerShape(topStart = SheetCornerTop, topEnd = SheetCornerTop),
            color = palette.surface,
            shadowElevation = SheetElevation,
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .navigationBarsPadding()
                    .padding(horizontal = KeynearSpacing.Lg, vertical = KeynearSpacing.Md)
                    .verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(SheetGap),
            ) {
                ShareSheetHeader(validation = validation, totalSizeBytes = totalSizeBytes, onClose = onClose)
                when (validation) {
                    is ShareValidationResult.Invalid -> {
                        ShareInvalidSection(reason = validation.reason, onOpenMain = onOpenMain, onClose = onClose)
                    }
                    is ShareValidationResult.Valid -> {
                        ShareValidBody(
                            sendError = sendError,
                            sentTransfer = sentTransfer,
                            pairedDevices = pairedDevices,
                            preselectedDeviceId = preselectedDeviceId,
                            sendingDeviceId = sendingDeviceId,
                            onSelectPeer = { devId ->
                                sendingDeviceId = devId
                                sendError = null
                                scope.launch {
                                    val res = onSendToPeer(devId)
                                    sendingDeviceId = null
                                    res.onSuccess { session -> sentTransfer = session }
                                        .onFailure { err -> sendError = err.message ?: "Unable to send" }
                                }
                            },
                            onOpenMain = onOpenMain,
                            onClose = onClose,
                        )
                    }
                }
            }
        }
    }
}
