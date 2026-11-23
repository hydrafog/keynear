package org.keynear.app.share

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
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
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import kotlinx.coroutines.launch
import org.keynear.app.ui.model.PairedDeviceUi
import org.keynear.app.ui.model.ShareValidationResult
import org.keynear.app.ui.model.TransferUi
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearMotion
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.LocalKeynearPalette
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone

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
    val enterAnim = remember { Animatable(0f) }
    val slideOffsetPx = with(LocalDensity.current) { KeynearSpacing.Xl.toPx() }

    LaunchedEffect(Unit) {
        enterAnim.animateTo(
            targetValue = 1f,
            animationSpec = tween(durationMillis = KeynearMotion.SHEET_DURATION_MS, easing = KeynearMotion.SpringEasing),
        )
    }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(palette.scrim.copy(alpha = palette.scrim.alpha * enterAnim.value))
            .clickable(interactionSource = scrimInteraction, indication = null, onClick = onClose),
    ) {
        TactileSurface(
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .fillMaxWidth()
                .graphicsLayer {
                    val p = enterAnim.value
                    alpha = p
                    translationY = slideOffsetPx * (1f - p)
                }
                .clickable(interactionSource = sheetInteraction, indication = null, onClick = {}),
            shape = KeynearShapes.Sheet,
            containerColor = palette.surface,
            tone = TactileTone.SHEET,
            cornerRadius = KeynearRadii.ThreeXl,
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .navigationBarsPadding()
                    .padding(horizontal = KeynearSpacing.Lg, vertical = KeynearSpacing.Md)
                    .verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(KeynearLayout.GridGap),
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
