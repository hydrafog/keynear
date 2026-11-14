package org.keynear.app.ui.components

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.core.BluetoothStatus
import org.keynear.app.core.CoreSnapshot
import org.keynear.app.core.LocalNetworkStatus
import org.keynear.app.core.PendingPairingUi
import org.keynear.app.core.TransferDirectionUi
import org.keynear.app.core.TransferStateUi
import org.keynear.app.core.TransferUi
import org.keynear.app.core.formatBytes
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.app.ui.theme.LocalKeynearPalette

@Composable
fun TactileSurface(
    modifier: Modifier = Modifier,
    shape: Shape = RoundedCornerShape(16.dp),
    containerColor: Color = LocalKeynearPalette.current.surface,
    shadowElevation: Dp = 3.dp,
    onClick: (() -> Unit)? = null,
    content: @Composable () -> Unit,
) {
    if (onClick != null) {
        Surface(
            modifier = modifier,
            onClick = onClick,
            shape = shape,
            color = containerColor,
            shadowElevation = shadowElevation,
            content = content,
        )
    } else {
        Surface(
            modifier = modifier,
            shape = shape,
            color = containerColor,
            shadowElevation = shadowElevation,
            content = content,
        )
    }
}

@Composable
fun KeynearSwitch(
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    val trackColor by animateColorAsState(
        targetValue = if (checked) palette.loading else palette.switchTrackOff,
        label = "switchTrack",
    )
    val thumbOffset by animateDpAsState(
        targetValue = if (checked) 28.dp else 4.dp,
        label = "switchThumb",
    )

    Box(
        modifier = modifier
            .width(58.dp)
            .height(34.dp)
            .clip(CircleShape)
            .background(trackColor)
            .clickable(role = Role.Switch) { onCheckedChange(!checked) },
        contentAlignment = Alignment.CenterStart,
    ) {
        Surface(
            modifier = Modifier
                .offset(x = thumbOffset)
                .size(26.dp),
            shape = CircleShape,
            color = palette.switchThumb,
            shadowElevation = 2.dp,
        ) {}
    }
}

@Composable
fun SelectionTile(
    label: String,
    active: Boolean = false,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    icon: @Composable (Color) -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val bg = if (active) palette.loadingTonal else palette.surface
    val iconColor = if (active) palette.onLoadingTonal else palette.textSecondary
    val textColor = if (active) palette.onLoadingTonal else palette.textPrimary

    TactileSurface(
        modifier = modifier.heightIn(min = 128.dp),
        shape = RoundedCornerShape(16.dp),
        containerColor = bg,
        shadowElevation = if (active) 0.dp else 3.dp,
        onClick = onClick,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .heightIn(min = 128.dp)
                .padding(16.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            icon(iconColor)
            Spacer(modifier = Modifier.height(12.dp))
            Text(
                text = label,
                color = textColor,
                fontSize = 16.sp,
                fontWeight = FontWeight.Bold,
            )
        }
    }
}

@Composable
fun SettingsSection(
    title: String,
    modifier: Modifier = Modifier,
    content: @Composable ColumnScope.() -> Unit,
) {
    val palette = LocalKeynearPalette.current
    Column(
        modifier = modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Text(
            text = title.uppercase(),
            color = palette.textMuted,
            fontSize = 12.sp,
            fontWeight = FontWeight.SemiBold,
            letterSpacing = 0.8.sp,
            modifier = Modifier.padding(horizontal = 4.dp),
        )
        TactileSurface(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(16.dp),
            containerColor = palette.surface,
            shadowElevation = 2.dp,
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 12.dp, vertical = 8.dp),
                verticalArrangement = Arrangement.spacedBy(4.dp),
                content = content,
            )
        }
    }
}

@Composable
fun SettingRow(
    title: String,
    subtitle: String? = null,
    onClick: (() -> Unit)? = null,
    trailing: @Composable () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val baseModifier = Modifier
        .fillMaxWidth()
        .defaultMinSize(minHeight = 48.dp)
        .let { m -> if (onClick != null) m.clickable(onClick = onClick) else m }
        .padding(horizontal = 4.dp, vertical = 8.dp)

    Row(
        modifier = baseModifier,
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = title,
                color = palette.textPrimary,
                fontSize = 15.sp,
                fontWeight = FontWeight.SemiBold,
            )
            if (!subtitle.isNullOrBlank()) {
                Text(
                    text = subtitle,
                    color = palette.textMuted,
                    fontSize = 12.sp,
                )
            }
        }
        trailing()
    }
}

@Composable
fun ActionButton(
    label: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    primary: Boolean = false,
    danger: Boolean = false,
    errorTonal: Boolean = false,
    enabled: Boolean = true,
) {
    val palette = LocalKeynearPalette.current
    val container = when {
        danger -> palette.error
        errorTonal -> palette.errorAction
        primary -> palette.loading
        else -> palette.loadingTonal
    }
    val content = when {
        danger || primary -> Color(0xFFFFFFFE)
        errorTonal -> palette.onDangerAction
        else -> palette.onLoadingTonal
    }

    Button(
        onClick = onClick,
        enabled = enabled,
        modifier = modifier.heightIn(min = 44.dp),
        shape = RoundedCornerShape(12.dp),
        colors = ButtonDefaults.buttonColors(
            containerColor = container,
            contentColor = content,
            disabledContainerColor = container.copy(alpha = 0.45f),
            disabledContentColor = content.copy(alpha = 0.65f),
        ),
        contentPadding = PaddingValues(horizontal = 16.dp, vertical = 10.dp),
    ) {
        Text(text = label, fontSize = 14.sp, fontWeight = FontWeight.SemiBold)
    }
}

@Composable
fun RecessedInput(
    value: String,
    onValueChange: (String) -> Unit,
    placeholder: String,
    modifier: Modifier = Modifier,
    singleLine: Boolean = true,
) {
    val palette = LocalKeynearPalette.current
    Box(
        modifier = modifier
            .heightIn(min = 48.dp)
            .clip(RoundedCornerShape(12.dp))
            .background(palette.input)
            .padding(horizontal = 14.dp, vertical = 12.dp),
        contentAlignment = Alignment.CenterStart,
    ) {
        if (value.isEmpty()) {
            Text(
                text = placeholder,
                color = palette.textMuted,
                fontSize = 14.sp,
            )
        }
        BasicTextField(
            value = value,
            onValueChange = onValueChange,
            singleLine = singleLine,
            cursorBrush = SolidColor(palette.loading),
            textStyle = TextStyle(
                color = palette.textPrimary,
                fontSize = 14.sp,
                fontWeight = FontWeight.Medium,
            ),
            modifier = Modifier.fillMaxWidth(),
        )
    }
}

@Composable
fun TransportWarningBanner(
    snapshot: CoreSnapshot,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current
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

    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(14.dp),
        color = palette.warningTonal,
        onClick = onClick,
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 14.dp, vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Text(
                text = msg,
                color = palette.onWarningTonal,
                fontSize = 12.sp,
                fontWeight = FontWeight.Medium,
                modifier = Modifier.weight(1f),
            )
            Text(
                text = ctaLabel,
                color = palette.onWarningTonal,
                fontSize = 12.sp,
                fontWeight = FontWeight.Bold,
            )
        }
    }
}

@Composable
fun PendingPairingBanner(
    prompt: PendingPairingUi,
    onApprove: () -> Unit,
    onReject: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    TactileSurface(
        modifier = modifier.fillMaxWidth(),
        shape = RoundedCornerShape(16.dp),
        containerColor = palette.surface,
        shadowElevation = 5.dp,
    ) {
        Column(
            modifier = Modifier.padding(14.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(
                text = "PAIR WITH ${prompt.peerName.uppercase()}",
                color = palette.textMuted,
                fontSize = 11.sp,
                fontWeight = FontWeight.SemiBold,
            )
            Text(
                text = prompt.verificationCode,
                color = palette.textPrimary,
                fontSize = 24.sp,
                fontWeight = FontWeight.Bold,
                letterSpacing = 3.sp,
            )
            Text(
                text = "Confirm this code matches (${prompt.remainingSecs}s)",
                color = palette.textMuted,
                fontSize = 12.sp,
            )
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                ActionButton(
                    label = "Decline",
                    onClick = onReject,
                    errorTonal = true,
                    modifier = Modifier.weight(1f),
                )
                ActionButton(
                    label = "Pair",
                    onClick = onApprove,
                    primary = true,
                    modifier = Modifier.weight(1f),
                )
            }
        }
    }
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
        shape = RoundedCornerShape(16.dp),
        containerColor = palette.surface,
        shadowElevation = 4.dp,
    ) {
        Column(
            modifier = Modifier.padding(12.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            transfers.take(2).forEach { tx ->
                Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = tx.summaryName,
                                color = palette.textPrimary,
                                fontSize = 13.sp,
                                fontWeight = FontWeight.SemiBold,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                            val dir = if (tx.direction == TransferDirectionUi.SEND) "To" else "From"
                            Text(
                                text = "$dir ${tx.peerDisplayName} · ${formatBytes(tx.totalBytes)}",
                                color = palette.textMuted,
                                fontSize = 11.sp,
                            )
                        }
                        when (tx.state) {
                            TransferStateUi.PENDING_CONSENT -> {
                                ActionButton(
                                    label = "Decline",
                                    onClick = { onDecline(tx.transferId) },
                                    errorTonal = true,
                                )
                                ActionButton(
                                    label = "Accept",
                                    onClick = { onAccept(tx.transferId) },
                                    primary = true,
                                )
                            }
                            TransferStateUi.ACTIVE -> {
                                ActionButton(
                                    label = "Cancel",
                                    onClick = { onCancel(tx.transferId) },
                                    errorTonal = true,
                                )
                            }
                            else -> {}
                        }
                    }
                    val fraction = if (tx.totalBytes > 0L) {
                        (tx.transferredBytes.toFloat() / tx.totalBytes.toFloat()).coerceIn(0f, 1f)
                    } else 0f
                    LinearProgressIndicator(
                        progress = { fraction },
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(6.dp)
                            .clip(CircleShape),
                        color = palette.loading,
                        trackColor = palette.switchTrackOff,
                    )
                }
            }
        }
    }
}
