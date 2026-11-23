package org.keynear.app.ui.receive

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.slideInVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import org.keynear.app.ui.components.ActiveTransfersStrip
import org.keynear.app.ui.components.PendingPairingBanner
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.PendingPairingUi
import org.keynear.app.ui.model.TransferStateUi
import org.keynear.app.ui.model.TransferUi
import org.keynear.app.ui.model.VisibilityState
import org.keynear.app.ui.model.radarLayoutFor
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearMotion
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.LocalKeynearPalette

private const val BANNER_ENTER_SCALE = 0.96f
private const val AURA_MID_STOP = 0.46f
private const val AURA_END_STOP = 0.72f

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
    val hasBanner = snapshot.pendingPairing != null ||
        snapshot.activeTransfers.any {
            it.state == TransferStateUi.PENDING_CONSENT || it.state == TransferStateUi.ACTIVE
        }
    val auraAlpha by animateFloatAsState(
        targetValue = if (radarActive) 1f else 0f,
        animationSpec = tween(durationMillis = KeynearMotion.DURATION_500_MS, easing = KeynearMotion.FadeEasing),
        label = "brandAuraAlpha",
    )

    Column(
        modifier = modifier
            .fillMaxSize()
            .background(palette.canvas),
    ) {
        ReceiveHeaderOverlay(snapshot = snapshot, actions = actions)

        BoxWithConstraints(
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth()
                .clipToBounds(),
        ) {
            val layout = radarLayoutFor(maxWidth.value, maxHeight.value)
            RadarCanvasLayer(radarActive = radarActive)

            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .graphicsLayer { alpha = auraAlpha },
                contentAlignment = Alignment.Center,
            ) {
                Box(
                    modifier = Modifier
                        .size(KeynearLayout.RadarAuraSize)
                        .clip(CircleShape)
                        .background(
                            Brush.radialGradient(
                                0.0f to palette.brandAuraInner,
                                AURA_MID_STOP to palette.brandAuraMid,
                                AURA_END_STOP to Color.Transparent,
                            ),
                        ),
                )
            }

            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .graphicsLayer {
                        scaleX = layout.scale
                        scaleY = layout.scale
                    },
                contentAlignment = Alignment.Center,
            ) {
                CenterSelfNode(
                    localDeviceName = localDeviceName,
                    visibility = snapshot.visibility,
                    remainingSecs = snapshot.everyoneRemainingSecs,
                )
                OrbitPinsLayer(
                    pins = pins,
                    radarActive = radarActive,
                    layout = layout,
                    actions = actions,
                )
            }
        }

        DevicesBottomSheet(
            pins = pins,
            hasBanner = hasBanner,
            expanded = sheetExpanded,
            onSetExpanded = onSetSheetExpanded,
            actions = actions,
        )
    }
}

@Composable
private fun ReceiveHeaderOverlay(
    snapshot: CoreSnapshot,
    actions: KeynearActions,
) {
    var lastPairing by remember { mutableStateOf<PendingPairingUi?>(null) }
    var lastTransfers by remember { mutableStateOf<List<TransferUi>>(emptyList()) }
    if (snapshot.pendingPairing != null) lastPairing = snapshot.pendingPairing
    if (snapshot.activeTransfers.isNotEmpty()) lastTransfers = snapshot.activeTransfers

    val slidePx = with(LocalDensity.current) { KeynearSpacing.Sm.roundToPx() }
    val bannerEnter = fadeIn(tween(KeynearMotion.MODAL_DURATION_MS, easing = KeynearMotion.SpringEasing)) +
        scaleIn(
            initialScale = BANNER_ENTER_SCALE,
            animationSpec = tween(KeynearMotion.MODAL_DURATION_MS, easing = KeynearMotion.SpringEasing),
        ) +
        slideInVertically(tween(KeynearMotion.MODAL_DURATION_MS, easing = KeynearMotion.SpringEasing)) { slidePx }
    val bannerExit = fadeOut(tween(KeynearMotion.EXIT_DURATION_MS, easing = KeynearMotion.ExitEasing)) +
        shrinkVertically(tween(KeynearMotion.EXIT_DURATION_MS, easing = KeynearMotion.ExitEasing))

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(
                start = KeynearSpacing.Lg,
                end = KeynearSpacing.Lg,
                top = KeynearSpacing.Md,
            ),
        verticalArrangement = Arrangement.spacedBy(KeynearLayout.ContentGap),
    ) {
        VisibilitySegmentedControl(
            current = snapshot.visibility,
            onSelect = actions.onSetVisibility,
        )
        AnimatedVisibility(visible = snapshot.pendingPairing != null, enter = bannerEnter, exit = bannerExit) {
            lastPairing?.let { prompt ->
                PendingPairingBanner(
                    prompt = prompt,
                    onApprove = actions.onApprovePairing,
                    onReject = actions.onRejectPairing,
                )
            }
        }
        AnimatedVisibility(visible = snapshot.activeTransfers.isNotEmpty(), enter = bannerEnter, exit = bannerExit) {
            if (lastTransfers.isNotEmpty()) {
                ActiveTransfersStrip(
                    transfers = lastTransfers,
                    onAccept = actions.onAcceptTransfer,
                    onDecline = actions.onDeclineTransfer,
                    onCancel = actions.onCancelTransfer,
                )
            }
        }
    }
}
