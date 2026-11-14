package org.keynear.app.share

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.provider.OpenableColumns
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
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
import androidx.core.content.IntentCompat
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.keynear.app.MainActivity
import org.keynear.app.core.CoreClientProvider
import org.keynear.app.core.CoreSnapshot
import org.keynear.app.core.CoreState
import org.keynear.app.core.DeviceRoleUi
import org.keynear.app.core.PairedDeviceUi
import org.keynear.app.core.ShareValidationResult
import org.keynear.app.core.TransferUi
import org.keynear.app.core.formatBytes
import org.keynear.app.core.validateShareIntent
import org.keynear.app.ui.theme.KeynearRadii
import org.keynear.app.ui.theme.KeynearSpacing
import org.keynear.app.ui.theme.KeynearTheme
import org.keynear.app.ui.theme.LocalKeynearPalette

class ShareActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val incoming = intent
        val action = incoming?.action
        val streamUris = extractStreamUris(incoming, action)
        val sharedText = incoming?.getStringExtra(Intent.EXTRA_TEXT)
        val preselectedDeviceId = DirectShareManager.extractDeviceId(
            shortcutId = incoming?.getStringExtra(Intent.EXTRA_SHORTCUT_ID),
            explicitDeviceId = incoming?.getStringExtra(DirectShareManager.EXTRA_TARGET_DEVICE_ID),
        )
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
                    totalSizeBytes = computeTotalShareBytes(applicationContext, streamUris, sharedText)
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

    private fun computeTotalShareBytes(
        context: Context,
        uris: List<String>,
        sharedText: String?,
    ): Long {
        var total = 0L
        uris.forEach { raw ->
            runCatching {
                context.contentResolver.query(
                    Uri.parse(raw),
                    arrayOf(OpenableColumns.SIZE),
                    null,
                    null,
                    null,
                )?.use { cursor ->
                    if (cursor.moveToFirst()) {
                        val sizeIdx = cursor.getColumnIndex(OpenableColumns.SIZE)
                        if (sizeIdx >= 0 && !cursor.isNull(sizeIdx)) {
                            total += cursor.getLong(sizeIdx).coerceAtLeast(0L)
                        }
                    }
                }
            }
        }
        if (!sharedText.isNullOrBlank()) {
            total += sharedText.trim().toByteArray(Charsets.UTF_8).size.toLong()
        }
        return total
    }

    private fun extractStreamUris(intent: Intent?, action: String?): List<String> {
        if (intent == null) return emptyList()
        return when (action) {
            Intent.ACTION_SEND -> {
                val single = IntentCompat.getParcelableExtra(intent, Intent.EXTRA_STREAM, Uri::class.java)
                if (single != null) listOf(single.toString()) else emptyList()
            }
            Intent.ACTION_SEND_MULTIPLE -> {
                val multiple = IntentCompat.getParcelableArrayListExtra(
                    intent,
                    Intent.EXTRA_STREAM,
                    Uri::class.java,
                )
                multiple?.map { it.toString() } ?: emptyList()
            }
            else -> emptyList()
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
            .clickable(
                interactionSource = scrimInteraction,
                indication = null,
                onClick = onClose,
            ),
    ) {
        Surface(
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .fillMaxWidth()
                .clickable(
                    interactionSource = sheetInteraction,
                    indication = null,
                    onClick = {},
                ),
            shape = RoundedCornerShape(topStart = 28.dp, topEnd = 28.dp),
            color = palette.surface,
            shadowElevation = 16.dp,
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .navigationBarsPadding()
                    .padding(horizontal = KeynearSpacing.Lg, vertical = 12.dp)
                    .verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(14.dp),
            ) {
                Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
                    Box(
                        modifier = Modifier
                            .width(36.dp)
                            .height(4.dp)
                            .clip(RoundedCornerShape(999.dp))
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
                            is ShareValidationResult.Valid -> {
                                "Share ${validation.itemCount} item${if (validation.itemCount == 1) "" else "s"}"
                            }
                            is ShareValidationResult.Invalid -> "Unable to share"
                        }
                        Text(
                            text = headerTitle,
                            fontSize = 19.sp,
                            fontWeight = FontWeight.ExtraBold,
                            color = palette.textPrimary,
                        )
                        val subtitle = when (validation) {
                            is ShareValidationResult.Valid -> {
                                if (totalSizeBytes > 0L) {
                                    "${formatBytes(totalSizeBytes)} · Direct Share"
                                } else {
                                    "Direct Share"
                                }
                            }
                            is ShareValidationResult.Invalid -> validation.reason
                        }
                        Text(
                            text = subtitle,
                            fontSize = 12.sp,
                            color = palette.textMuted,
                        )
                    }
                    Box(
                        modifier = Modifier
                            .size(KeynearSpacing.MinTouchTarget)
                            .clip(CircleShape)
                            .clickable(onClick = onClose),
                        contentAlignment = Alignment.Center,
                    ) {
                        Surface(
                            modifier = Modifier.size(36.dp),
                            shape = CircleShape,
                            color = palette.surfaceTonal,
                        ) {
                            Box(contentAlignment = Alignment.Center) {
                                ShareCloseIcon(color = palette.textPrimary, size = 16.dp)
                            }
                        }
                    }
                }

                if (validation is ShareValidationResult.Invalid) {
                    Surface(
                        modifier = Modifier.fillMaxWidth(),
                        shape = RoundedCornerShape(KeynearRadii.Xl),
                        color = palette.errorTonal,
                    ) {
                        Text(
                            text = validation.reason,
                            fontSize = 13.sp,
                            color = palette.onDangerTonal,
                            modifier = Modifier.padding(KeynearSpacing.Lg),
                        )
                    }
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        Button(
                            onClick = onOpenMain,
                            modifier = Modifier
                                .weight(1f)
                                .defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                            shape = RoundedCornerShape(999.dp),
                            colors = ButtonDefaults.buttonColors(
                                containerColor = palette.accent,
                                contentColor = palette.onAccent,
                            ),
                        ) {
                            Text("Open Keynear", fontWeight = FontWeight.Bold)
                        }
                        OutlinedButton(
                            onClick = onClose,
                            modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                            shape = RoundedCornerShape(999.dp),
                        ) {
                            Text("Close", color = palette.textPrimary)
                        }
                    }
                } else {
                    sendError?.let { msg ->
                        Surface(
                            modifier = Modifier.fillMaxWidth(),
                            shape = RoundedCornerShape(KeynearRadii.Xl),
                            color = palette.errorTonal,
                        ) {
                            Text(
                                text = msg,
                                fontSize = 13.sp,
                                color = palette.onDangerTonal,
                                modifier = Modifier.padding(KeynearSpacing.Md),
                            )
                        }
                    }

                    val confirmed = sentTransfer
                    if (confirmed != null) {
                        Surface(
                            modifier = Modifier.fillMaxWidth(),
                            shape = RoundedCornerShape(20.dp),
                            color = palette.loadingTonal,
                        ) {
                            Column(
                                modifier = Modifier.padding(KeynearSpacing.Lg),
                                verticalArrangement = Arrangement.spacedBy(10.dp),
                            ) {
                                Text(
                                    text = "Sending ${confirmed.summaryName} to ${confirmed.peerDisplayName}",
                                    fontSize = 14.sp,
                                    fontWeight = FontWeight.Bold,
                                    color = palette.onLoadingTonal,
                                )
                                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                    Button(
                                        onClick = onOpenMain,
                                        modifier = Modifier
                                            .weight(1f)
                                            .defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                                        shape = RoundedCornerShape(999.dp),
                                        colors = ButtonDefaults.buttonColors(
                                            containerColor = palette.accent,
                                            contentColor = palette.onAccent,
                                        ),
                                    ) {
                                        Text("Open Keynear", fontWeight = FontWeight.Bold)
                                    }
                                    OutlinedButton(
                                        onClick = onClose,
                                        modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                                        shape = RoundedCornerShape(999.dp),
                                    ) {
                                        Text("Done", color = palette.onLoadingTonal)
                                    }
                                }
                            }
                        }
                    } else {
                        Text(
                            text = "DIRECT SHARE VIA KEYNEAR",
                            fontSize = 11.sp,
                            fontWeight = FontWeight.Bold,
                            letterSpacing = 0.8.sp,
                            color = palette.textMuted,
                        )

                        when {
                            pairedDevices == null -> {
                                Box(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .padding(vertical = 24.dp),
                                    contentAlignment = Alignment.Center,
                                ) {
                                    CircularProgressIndicator(color = palette.accent)
                                }
                            }
                            pairedDevices.isEmpty() -> {
                                Surface(
                                    modifier = Modifier.fillMaxWidth(),
                                    shape = RoundedCornerShape(20.dp),
                                    color = palette.surfaceTonal,
                                ) {
                                    Column(
                                        modifier = Modifier.padding(KeynearSpacing.Lg),
                                        verticalArrangement = Arrangement.spacedBy(10.dp),
                                    ) {
                                        Text(
                                            text = "No paired devices yet",
                                            fontSize = 15.sp,
                                            fontWeight = FontWeight.Bold,
                                            color = palette.textPrimary,
                                        )
                                        Text(
                                            text = "Open Keynear to pair a nearby device before sharing.",
                                            fontSize = 13.sp,
                                            color = palette.textMuted,
                                        )
                                        Button(
                                            onClick = onOpenMain,
                                            modifier = Modifier.defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget),
                                            shape = RoundedCornerShape(999.dp),
                                            colors = ButtonDefaults.buttonColors(
                                                containerColor = palette.accent,
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
                                        .heightIn(max = 320.dp),
                                    verticalArrangement = Arrangement.spacedBy(10.dp),
                                ) {
                                    rows.forEach { pair ->
                                        Row(
                                            modifier = Modifier.fillMaxWidth(),
                                            horizontalArrangement = Arrangement.spacedBy(10.dp),
                                        ) {
                                            pair.forEach { dev ->
                                                ShareDeviceTile(
                                                    device = dev,
                                                    highlighted = dev.deviceId == preselectedDeviceId,
                                                    isSending = sendingDeviceId == dev.deviceId,
                                                    enabled = sendingDeviceId == null,
                                                    onClick = {
                                                        sendingDeviceId = dev.deviceId
                                                        sendError = null
                                                        scope.launch {
                                                            val res = onSendToPeer(dev.deviceId)
                                                            sendingDeviceId = null
                                                            res.onSuccess { session ->
                                                                sentTransfer = session
                                                            }.onFailure { err ->
                                                                sendError = err.message ?: "Unable to send"
                                                            }
                                                        }
                                                    },
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
        modifier = modifier.defaultMinSize(minHeight = 96.dp),
        shape = RoundedCornerShape(18.dp),
        color = palette.surfaceTonal,
        border = if (highlighted) BorderStroke(1.5.dp, palette.accent) else null,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(12.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Surface(
                    modifier = Modifier.size(44.dp),
                    shape = RoundedCornerShape(14.dp),
                    color = palette.surface,
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        if (isSending) {
                            CircularProgressIndicator(
                                modifier = Modifier.size(20.dp),
                                strokeWidth = 2.dp,
                                color = palette.accent,
                            )
                        } else {
                            ShareRoleIcon(
                                role = device.role,
                                color = if (device.isOnline) palette.accent else palette.textSecondary,
                                size = 22.dp,
                            )
                        }
                    }
                }
            }
            Column {
                Text(
                    text = device.displayName,
                    fontSize = 14.sp,
                    fontWeight = FontWeight.Bold,
                    color = palette.textPrimary,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    text = if (isSending) {
                        "Starting..."
                    } else if (device.isOnline) {
                        "Ready · Wi-Fi"
                    } else {
                        "Standby"
                    },
                    fontSize = 12.sp,
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
