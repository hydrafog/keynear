package org.keynear.app.ui.receive

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.slideInVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
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
private const val TRANSFER_AUTO_DISMISS_MS = 3500L
private const val SWIPE_OFFSCREEN_FRACTION = 1.15f
private val SwipeDismissThreshold = 88.dp

@Composable
fun ReceiveScreen(
    snapshot: CoreSnapshot,
    localDeviceName: String,
    sheetExpanded: Boolean,
    onSetSheetExpanded: (Boolean) -> Unit,
    actions: KeynearActions,
    dismissedTransferIds: Set<String> = emptySet(),
    onDismissTransfers: (List<String>) -> Unit = {},
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    val pins = ReceiveStateHolder.buildMapPins(snapshot)
    val radarActive = snapshot.visibility != VisibilityState.HIDDEN
    val visibleTransfers = snapshot.activeTransfers.filter { it.transferId !in dismissedTransferIds }
    val hasBanner = snapshot.pendingPairing != null ||
        visibleTransfers.any {
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
        ReceiveHeaderOverlay(
            snapshot = snapshot,
            dismissedTransferIds = dismissedTransferIds,
            onDismissTransfers = onDismissTransfers,
            actions = actions,
        )

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
    dismissedTransferIds: Set<String>,
    onDismissTransfers: (List<String>) -> Unit,
    actions: KeynearActions,
) {
    var lastPairing by remember { mutableStateOf<PendingPairingUi?>(null) }
    var lastTransfers by remember { mutableStateOf<List<TransferUi>>(emptyList()) }
    val visibleTransfers = snapshot.activeTransfers.filter { it.transferId !in dismissedTransferIds }
    if (snapshot.pendingPairing != null) lastPairing = snapshot.pendingPairing
    if (visibleTransfers.isNotEmpty()) lastTransfers = visibleTransfers

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
            .padding(start = KeynearSpacing.Lg, end = KeynearSpacing.Lg, top = KeynearSpacing.Md),
        verticalArrangement = Arrangement.spacedBy(KeynearLayout.ContentGap),
    ) {
        VisibilitySegmentedControl(current = snapshot.visibility, onSelect = actions.onSetVisibility)
        AnimatedVisibility(visible = snapshot.pendingPairing != null, enter = bannerEnter, exit = bannerExit) {
            lastPairing?.let { prompt ->
                PendingPairingBanner(prompt = prompt, onApprove = actions.onApprovePairing, onReject = actions.onRejectPairing)
            }
        }
        AnimatedVisibility(visible = visibleTransfers.isNotEmpty(), enter = bannerEnter, exit = bannerExit) {
            if (lastTransfers.isNotEmpty()) {
                SwipeableTransferBanner(
                    transfers = lastTransfers,
                    actions = actions,
                    onDismiss = onDismissTransfers,
                )
            }
        }
    }
}

@Composable
private fun SwipeableTransferBanner(
    transfers: List<TransferUi>,
    actions: KeynearActions,
    onDismiss: (List<String>) -> Unit,
) {
    val scope = rememberCoroutineScope()
    val offsetX = remember { Animatable(0f) }
    val thresholdPx = with(LocalDensity.current) { SwipeDismissThreshold.toPx() }
    var widthPx by remember { mutableFloatStateOf(0f) }

    suspend fun animateDismissRight() {
        val target = (widthPx * SWIPE_OFFSCREEN_FRACTION).coerceAtLeast(thresholdPx * 4f)
        offsetX.animateTo(target, tween(KeynearMotion.EXIT_DURATION_MS, easing = KeynearMotion.ExitEasing))
        onDismiss(transfers.map { it.transferId })
    }

    LaunchedEffect(transfers.map { it.transferId }) {
        if (offsetX.value != 0f) offsetX.snapTo(0f)
    }

    LaunchedEffect(transfers) {
        if (transfers.isNotEmpty() && transfers.all { it.state.isTerminal() }) {
            delay(TRANSFER_AUTO_DISMISS_MS)
            animateDismissRight()
        }
    }

    Box(
        modifier = Modifier
            .fillMaxWidth()
            .onSizeChanged { widthPx = it.width.toFloat() }
            .graphicsLayer {
                translationX = offsetX.value
                alpha = if (widthPx > 0f) (1f - (offsetX.value / widthPx)).coerceIn(0f, 1f) else 1f
            }
            .swipeRightToDismiss(
                key = transfers,
                onDrag = { delta -> scope.launch { offsetX.snapTo((offsetX.value + delta).coerceAtLeast(0f)) } },
                onEnd = {
                    scope.launch {
                        if (offsetX.value >= thresholdPx) {
                            animateDismissRight()
                        } else {
                            offsetX.animateTo(0f, tween(KeynearMotion.SHORT_DURATION_MS, easing = KeynearMotion.SpringEasing))
                        }
                    }
                },
            ),
    ) {
        ActiveTransfersStrip(
            transfers = transfers,
            onAccept = actions.onAcceptTransfer,
            onDecline = actions.onDeclineTransfer,
            onCancel = actions.onCancelTransfer,
        )
    }
}

private fun Modifier.swipeRightToDismiss(
    key: Any,
    onDrag: (Float) -> Unit,
    onEnd: () -> Unit,
): Modifier = pointerInput(key) {
    detectHorizontalDragGestures(
        onHorizontalDrag = { change, dragAmount ->
            change.consume()
            onDrag(dragAmount)
        },
        onDragEnd = onEnd,
        onDragCancel = onEnd,
    )
}
