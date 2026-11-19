package org.keynear.app.ui.receive

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.unit.dp
import org.keynear.app.ui.components.ActiveTransfersStrip
import org.keynear.app.ui.components.PendingPairingBanner
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.VisibilityState
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.LocalKeynearPalette

private val ContentGap = 10.dp
private val MapTopInset = 80.dp

@Composable
fun ReceiveScreen(
    snapshot: CoreSnapshot,
    localDeviceName: String,
    sheetExpanded: Boolean,
    onSetSheetExpanded: (Boolean) -> Unit,
    actions: KeynearActions,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    val pins = ReceiveStateHolder.buildMapPins(snapshot)
    val radarActive = snapshot.visibility != VisibilityState.HIDDEN

    Column(
        modifier = modifier
            .fillMaxSize()
            .background(
                brush = Brush.verticalGradient(
                    0.0f to palette.mapBlue,
                    0.65f to palette.mapBlue,
                    1.0f to palette.canvas,
                ),
            ),
    ) {
        Box(
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth(),
        ) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(top = MapTopInset),
            ) {
                RadarCanvasLayer(radarActive = radarActive)

                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center,
                ) {
                    CenterSelfNode(
                        localDeviceName = localDeviceName,
                        visibility = snapshot.visibility,
                        remainingSecs = snapshot.everyoneRemainingSecs,
                    )
                    OrbitPinsLayer(
                        pins = pins,
                        actions = actions,
                    )
                }
            }

            Column(
                modifier = Modifier
                    .align(Alignment.TopCenter)
                    .fillMaxWidth()
                    .padding(start = KeynearSpacing.Md, end = KeynearSpacing.Md, top = KeynearSpacing.Md),
                verticalArrangement = Arrangement.spacedBy(ContentGap),
            ) {
                VisibilitySegmentedControl(
                    current = snapshot.visibility,
                    onSelect = actions.onSetVisibility,
                )

                snapshot.pendingPairing?.let { prompt ->
                    PendingPairingBanner(
                        prompt = prompt,
                        onApprove = actions.onApprovePairing,
                        onReject = actions.onRejectPairing,
                    )
                }

                if (snapshot.activeTransfers.isNotEmpty()) {
                    ActiveTransfersStrip(
                        transfers = snapshot.activeTransfers,
                        onAccept = actions.onAcceptTransfer,
                        onDecline = actions.onDeclineTransfer,
                        onCancel = actions.onCancelTransfer,
                    )
                }
            }
        }

        DevicesBottomSheet(
            pins = pins,
            expanded = sheetExpanded,
            onSetExpanded = onSetSheetExpanded,
            actions = actions,
        )
    }
}
