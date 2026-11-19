package org.keynear.app.share

import android.content.Intent
import android.os.Bundle
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.keynear.app.MainActivity
import org.keynear.app.bridge.CoreClientProvider
import org.keynear.app.bridge.validateShareIntent
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.CoreState
import org.keynear.app.ui.model.DeviceRoleUi
import org.keynear.app.ui.model.PairedDeviceUi
import org.keynear.app.ui.model.ShareValidationResult
import org.keynear.app.ui.model.TransferUi
import org.keynear.app.ui.model.formatBytes
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTheme
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

class ShareActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val incoming = intent
        val action = incoming?.action
        val streamUris = IntentResolver.extractStreamUris(incoming, action)
        val sharedText = incoming?.getStringExtra(Intent.EXTRA_TEXT)
        val preselectedDeviceId = IntentResolver.extractPreselectedDeviceId(incoming)
        val validation = validateShareIntent(action, streamUris, sharedText)

        when (validation) {
            is ShareValidationResult.Valid -> {
                Log.i(
                    TAG,
                    "share_intent_validated items=${validation.itemCount} has_text=${validation.hasText}",
                )
            }
            is ShareValidationResult.Invalid -> {
                Log.w(TAG, "share_intent_rejected reason=${validation.reason}")
            }
        }

        val client = CoreClientProvider.init(applicationContext)
        setContent {
            var loadedSnapshot by remember { mutableStateOf<CoreSnapshot?>(null) }
            var totalSizeBytes by remember { mutableLongStateOf(0L) }
            val systemDark = isSystemInDarkTheme()
            val darkTheme = loadedSnapshot?.daemonConfig?.darkTheme ?: systemDark

            LaunchedEffect(Unit) {
                val state = withContext(Dispatchers.IO) { client.loadState() }
                val snapshot = when (state) {
                    is CoreState.Empty -> state.snapshot
                    is CoreState.Ready -> state.snapshot
                    is CoreState.Error -> state.previous
                    CoreState.Loading -> null
                }
                loadedSnapshot = snapshot
                val devices = snapshot?.pairedDevices.orEmpty()
                withContext(Dispatchers.IO) {
                    DirectShareManager.syncShortcuts(applicationContext, devices)
                    totalSizeBytes = IntentResolver.computeTotalShareBytes(applicationContext, streamUris, sharedText)
                }
            }

            KeynearTheme(darkTheme = darkTheme) {
                SharePickerScreen(
                    validation = validation,
                    totalSizeBytes = totalSizeBytes,
                    pairedDevices = loadedSnapshot?.pairedDevices,
                    preselectedDeviceId = preselectedDeviceId,
                    onSendToPeer = { deviceId ->
                        withContext(Dispatchers.IO) {
                            client.sendContentUris(deviceId, streamUris, sharedText)
                        }
                    },
                    onOpenMain = {
                        startActivity(Intent(this, MainActivity::class.java))
                        finish()
                    },
                    onClose = { finish() },
                )
            }
        }
    }

    companion object {
        private const val TAG = "ShareActivity"
    }
}

@Composable
private fun SharePickerScreen(
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

@Composable
private fun ShareSheetHeader(
    validation: ShareValidationResult,
    totalSizeBytes: Long,
    onClose: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
        Box(
            modifier = Modifier
                .width(HandleWidth)
                .height(HandleHeight)
                .clip(RoundedCornerShape(PillCorner))
                .background(palette.sheetHandle),
        )
    }

    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            val headerTitle = when (validation) {
                is ShareValidationResult.Valid ->
                    "Share ${validation.itemCount} item${if (validation.itemCount == 1) "" else "s"}"
                is ShareValidationResult.Invalid -> "Unable to share"
            }
            Text(
                text = headerTitle,
                fontSize = HeaderTextSize,
                fontWeight = FontWeight.ExtraBold,
                color = palette.textPrimary,
            )
            val subtitle = when (validation) {
                is ShareValidationResult.Valid ->
                    if (totalSizeBytes > 0L) "${formatBytes(totalSizeBytes)} · Direct Share" else "Direct Share"
                is ShareValidationResult.Invalid -> validation.reason
            }
            Text(text = subtitle, fontSize = KeynearTypeSizes.Caption, color = palette.textMuted)
        }
        Box(
            modifier = Modifier
                .size(KeynearSpacing.MinTouchTarget)
                .clip(CircleShape)
                .clickable(onClick = onClose),
            contentAlignment = Alignment.Center,
        ) {
            Surface(modifier = Modifier.size(CloseBadgeSize), shape = CircleShape, color = palette.surfaceTonal) {
                Box(contentAlignment = Alignment.Center) {
                    ShareCloseIcon(color = palette.error, size = KeynearSpacing.Lg)
                }
            }
        }
    }
}

@Composable
private fun ShareInvalidSection(
    reason: String,
    onOpenMain: () -> Unit,
    onClose: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.Xl),
        color = palette.errorTonal,
    ) {
        Text(
            text = reason,
            fontSize = SmallHeaderSize,
            color = palette.onDangerTonal,
            modifier = Modifier.padding(KeynearSpacing.Lg),
        )
    }
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(SectionGap),
    ) {
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
            Text("Close", color = palette.textPrimary)
        }
    }
}

@Composable
private fun ShareValidBody(
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

@Composable
private fun ShareDeviceTile(
    device: PairedDeviceUi,
    highlighted: Boolean,
    isSending: Boolean,
    enabled: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    Surface(
        onClick = onClick,
        enabled = enabled,
        modifier = modifier.defaultMinSize(minHeight = DeviceTileMinHeight),
        shape = RoundedCornerShape(DeviceTileCornerRadius),
        color = if (highlighted) palette.loadingTonal else palette.surfaceTonal,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Md),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Surface(
                    modifier = Modifier.size(DeviceIconContainerSize),
                    shape = RoundedCornerShape(DeviceIconContainerRadius),
                    color = if (device.isOnline) palette.loadingTonal else palette.surface,
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        if (isSending) {
                            CircularProgressIndicator(
                                modifier = Modifier.size(20.dp),
                                strokeWidth = 2.dp,
                                color = palette.loading,
                            )
                        } else {
                            ShareRoleIcon(
                                role = device.role,
                                color = if (device.isOnline) palette.onLoadingTonal else palette.textMuted,
                                size = 22.dp,
                            )
                        }
                    }
                }
            }
            Column {
                Text(
                    text = device.displayName,
                    fontSize = KeynearTypeSizes.Description,
                    fontWeight = FontWeight.Bold,
                    color = palette.textPrimary,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    text = when {
                        isSending -> "Starting..."
                        device.isOnline -> "Ready · Wi-Fi"
                        else -> "Standby"
                    },
                    fontSize = KeynearTypeSizes.Caption,
                    color = if (device.isOnline) palette.loading else palette.textMuted,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

@Composable
private fun ShareRoleIcon(role: DeviceRoleUi, color: Color, size: Dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.095f
        when (role) {
            DeviceRoleUi.MOBILE -> {
                drawRoundRect(
                    color = color,
                    topLeft = Offset(w * 0.26f, h * 0.12f),
                    size = Size(w * 0.48f, h * 0.76f),
                    cornerRadius = CornerRadius(w * 0.08f, w * 0.08f),
                    style = Stroke(width = sw),
                )
                drawCircle(color = color, radius = sw * 0.65f, center = Offset(w * 0.50f, h * 0.76f))
            }
            DeviceRoleUi.DESKTOP -> {
                drawRoundRect(
                    color = color,
                    topLeft = Offset(w * 0.18f, h * 0.22f),
                    size = Size(w * 0.64f, h * 0.46f),
                    cornerRadius = CornerRadius(w * 0.06f, w * 0.06f),
                    style = Stroke(width = sw),
                )
                drawLine(color, Offset(w * 0.10f, h * 0.76f), Offset(w * 0.90f, h * 0.76f), strokeWidth = sw, cap = StrokeCap.Round)
            }
        }
    }
}

@Composable
private fun ShareCloseIcon(color: Color, size: Dp) {
    Canvas(modifier = Modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val sw = w * 0.12f
        drawLine(color, Offset(w * 0.24f, h * 0.24f), Offset(w * 0.76f, h * 0.76f), strokeWidth = sw, cap = StrokeCap.Round)
        drawLine(color, Offset(w * 0.76f, h * 0.24f), Offset(w * 0.24f, h * 0.76f), strokeWidth = sw, cap = StrokeCap.Round)
    }
}
