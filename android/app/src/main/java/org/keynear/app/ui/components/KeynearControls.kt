package org.keynear.app.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import org.keynear.app.ui.model.formatBytes
import org.keynear.app.ui.model.BluetoothStatus
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.LocalNetworkStatus
import org.keynear.app.ui.model.PendingPairingUi
import org.keynear.app.ui.model.TransferDirectionUi
import org.keynear.app.ui.model.TransferStateUi
import org.keynear.app.ui.model.TransferUi
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.designsystem.ActionButton
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearProgressBar
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette
import org.keynear.designsystem.PairingPromptCard
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.designsystem.WarningBannerCard

private val CompactGap = 6.dp
private val ContentGap = 10.dp
private val TransferActionMinHeight = 44.dp

@Composable
fun TransportWarningBanner(
    snapshot: CoreSnapshot,
    actions: KeynearActions,
) {
    val (msg, ctaLabel, onClick) = when {
        snapshot.bluetoothStatus == BluetoothStatus.MISSING_PERMISSION ->
            Triple("Bluetooth permission required for discovery", "Grant", actions.onRequestBluetoothPermissions)
        snapshot.bluetoothStatus == BluetoothStatus.ADAPTER_DISABLED ->
            Triple("Bluetooth is turned off", "Settings", actions.onOpenBluetoothSettings)
        snapshot.localNetworkStatus == LocalNetworkStatus.MISSING_PERMISSION ->
            Triple("Local network permission required", "Allow", actions.onRequestLocalNetworkPermission)
        snapshot.localNetworkStatus == LocalNetworkStatus.BLOCKED_LNP ->
            Triple(snapshot.localNetworkMessage ?: "Local network discovery blocked", "Retry", actions.onRequestLocalNetworkPermission)
        else -> return
    }
    WarningBannerCard(message = msg, ctaLabel = ctaLabel, onClick = onClick)
}

@Composable
fun PendingPairingBanner(
    prompt: PendingPairingUi,
    onApprove: () -> Unit,
    onReject: () -> Unit,
    modifier: Modifier = Modifier,
) {
    PairingPromptCard(
        heading = "PAIR WITH ${prompt.peerName.uppercase()}",
        verificationCode = prompt.verificationCode,
        countdownText = "Confirm this code matches (${prompt.remainingSecs}s)",
        declineLabel = "Decline",
        confirmLabel = "Pair",
        onDecline = onReject,
        onConfirm = onApprove,
        modifier = modifier,
    )
}

@Composable
fun ActiveTransfersStrip(
    transfers: List<TransferUi>,
    onAccept: (String) -> Unit,
    onDecline: (String) -> Unit,
    onCancel: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    TactileSurface(
        modifier = modifier.fillMaxWidth(),
        shape = KeynearShapes.Card,
        containerColor = palette.surface.copy(alpha = 0.94f),
        tone = TactileTone.SURFACE,
        cornerRadius = KeynearRadii.TwoXl,
    ) {
        Column(
            modifier = Modifier.padding(KeynearLayout.CardPaddingLarge),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Lg),
        ) {
            transfers.take(2).forEach { tx ->
                val fraction = if (tx.totalBytes > 0L) {
                    (tx.transferredBytes.toFloat() / tx.totalBytes.toFloat()).coerceIn(0f, 1f)
                } else {
                    0f
                }
                val pct = (fraction * 100f).toInt()
                Column(verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Md)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.Top,
                        horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
                    ) {
                        Column(
                            modifier = Modifier.weight(1f),
                            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Xs),
                        ) {
                            Text(
                                text = tx.summaryName,
                                color = palette.textPrimary,
                                fontSize = KeynearTypeSizes.Body,
                                fontWeight = FontWeight.Bold,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                            val dir = if (tx.direction == TransferDirectionUi.SEND) "To" else "From"
                            Text(
                                text = "$dir ${tx.peerDisplayName} · ${formatBytes(tx.totalBytes)}",
                                color = palette.textMuted,
                                fontSize = KeynearTypeSizes.Description,
                            )
                        }
                        if (tx.state == TransferStateUi.ACTIVE) {
                            Text(
                                text = "$pct%",
                                color = palette.loadingInk,
                                fontSize = KeynearTypeSizes.Description,
                                fontWeight = FontWeight.Bold,
                            )
                        }
                    }
                    if (tx.state == TransferStateUi.ACTIVE) {
                        KeynearProgressBar(progress = fraction)
                    }
                    when (tx.state) {
                        TransferStateUi.PENDING_CONSENT -> {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
                            ) {
                                ActionButton(
                                    label = "Decline",
                                    onClick = { onDecline(tx.transferId) },
                                    tonalSurface = true,
                                    minHeight = KeynearSpacing.MinTouchTarget,
                                    modifier = Modifier.weight(1f),
                                )
                                ActionButton(
                                    label = "Accept",
                                    onClick = { onAccept(tx.transferId) },
                                    primary = true,
                                    minHeight = KeynearSpacing.MinTouchTarget,
                                    modifier = Modifier.weight(1f),
                                )
                            }
                        }
                        TransferStateUi.ACTIVE -> {
                            ActionButton(
                                label = "Cancel transfer",
                                onClick = { onCancel(tx.transferId) },
                                tonalSurface = true,
                                minHeight = KeynearSpacing.MinTouchTarget,
                                modifier = Modifier.fillMaxWidth(),
                            )
                        }
                        else -> {}
                    }
                }
            }
        }
    }
}
