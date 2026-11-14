package org.keynear.app.share

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.RadioButton
import androidx.compose.material3.RadioButtonDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import androidx.compose.ui.text.style.TextOverflow
import androidx.core.content.IntentCompat
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.keynear.app.MainActivity
import org.keynear.app.R
import org.keynear.app.core.CoreClientProvider
import org.keynear.app.core.CoreSnapshot
import org.keynear.app.core.CoreState
import org.keynear.app.core.PairedDeviceUi
import org.keynear.app.core.ShareValidationResult
import org.keynear.app.core.TransferUi
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
            KeynearTheme {
                SharePickerScreen(
                    validation = validation,
                    streamUris = streamUris,
                    sharedText = sharedText,
                    preselectedDeviceId = preselectedDeviceId,
                    onLoadDevices = {
                        val devices = when (val state = client.loadState()) {
                            is CoreState.Empty -> state.snapshot.pairedDevices
                            is CoreState.Ready -> state.snapshot.pairedDevices
                            is CoreState.Error -> state.previous?.pairedDevices.orEmpty()
                            CoreState.Loading -> emptyList()
                        }
                        DirectShareManager.syncShortcuts(applicationContext, devices)
                        devices
                    },
                    onSendToPeer = { deviceId ->
                        client.sendContentUris(deviceId, streamUris, sharedText)
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
    streamUris: List<String>,
    sharedText: String?,
    preselectedDeviceId: String?,
    onLoadDevices: suspend () -> List<PairedDeviceUi>,
    onSendToPeer: suspend (String) -> Result<TransferUi>,
    onOpenMain: () -> Unit,
    onClose: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val scope = rememberCoroutineScope()
    var pairedDevices by remember { mutableStateOf<List<PairedDeviceUi>?>(null) }
    var selectedDeviceId by remember(preselectedDeviceId) { mutableStateOf<String?>(null) }
    var sending by remember { mutableStateOf(false) }
    var sendError by remember { mutableStateOf<String?>(null) }
    var sentTransfer by remember { mutableStateOf<TransferUi?>(null) }

    LaunchedEffect(preselectedDeviceId) {
        val loaded = withContext(Dispatchers.IO) { onLoadDevices() }
        pairedDevices = loaded
        if (selectedDeviceId == null && loaded.isNotEmpty()) {
            selectedDeviceId = loaded.firstOrNull { it.deviceId == preselectedDeviceId }?.deviceId
                ?: loaded.first().deviceId
        }
    }

    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.BottomCenter,
    ) {
        Box(
            modifier = Modifier
                .matchParentSize()
                .background(Color.Black.copy(alpha = 0.45f))
                .clickable(
                    interactionSource = remember { MutableInteractionSource() },
                    indication = null,
                    onClick = onClose,
                ),
        )
        Surface(
            modifier = Modifier
                .fillMaxWidth()
                .widthIn(max = 560.dp),
            shape = RoundedCornerShape(
                topStart = KeynearRadii.ThreeXl,
                topEnd = KeynearRadii.ThreeXl,
            ),
            color = palette.canvas,
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .verticalScroll(rememberScrollState())
                    .padding(KeynearSpacing.Lg),
                verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
            ) {
            Text(
                text = stringResource(R.string.share_title),
                style = MaterialTheme.typography.headlineSmall,
                color = palette.textPrimary,
            )

            if (validation is ShareValidationResult.Invalid) {
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(KeynearRadii.TwoXl),
                    colors = CardDefaults.cardColors(containerColor = palette.errorTonal),
                ) {
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(KeynearSpacing.Lg),
                        verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
                    ) {
                        Text(
                            text = stringResource(R.string.share_invalid_message),
                            style = MaterialTheme.typography.titleMedium,
                            color = palette.onDangerTonal,
                        )
                        Text(
                            text = "Reason: ${validation.reason}",
                            style = MaterialTheme.typography.bodyMedium,
                            color = palette.onDangerTonal,
                        )
                        Button(
                            onClick = onClose,
                            modifier = Modifier.defaultMinSize(
                                minWidth = KeynearSpacing.MinTouchTarget,
                                minHeight = KeynearSpacing.MinTouchTarget,
                            ),
                            shape = RoundedCornerShape(KeynearRadii.Xl),
                            colors = ButtonDefaults.buttonColors(
                                containerColor = palette.accent,
                                contentColor = palette.onAccent,
                            ),
                        ) {
                            Text(text = stringResource(R.string.action_close))
                        }
                    }
                }
                return@Column
            }

            val valid = validation as ShareValidationResult.Valid
            val summaryText = buildString {
                if (valid.itemCount > 0) {
                    append("${valid.itemCount} item(s)")
                }
                if (valid.hasText) {
                    if (isNotEmpty()) append(" + ")
                    append("shared text")
                }
            }
            Text(
                text = summaryText,
                style = MaterialTheme.typography.bodyMedium,
                color = palette.textSecondary,
            )

            val devices = pairedDevices
            if (devices == null) {
                LinearProgressIndicator(
                    modifier = Modifier.fillMaxWidth(),
                    color = palette.accent,
                    trackColor = palette.surfaceTonal,
                )
            } else if (devices.isEmpty()) {
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(KeynearRadii.TwoXl),
                    colors = CardDefaults.cardColors(containerColor = palette.surfaceTonal),
                ) {
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(KeynearSpacing.Lg),
                        verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
                    ) {
                        Text(
                            text = stringResource(R.string.empty_devices_title),
                            style = MaterialTheme.typography.titleMedium,
                            color = palette.textPrimary,
                        )
                        Text(
                            text = stringResource(R.string.share_no_devices_body),
                            style = MaterialTheme.typography.bodyMedium,
                            color = palette.textSecondary,
                        )
                        Row(horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm)) {
                            Button(
                                onClick = onOpenMain,
                                modifier = Modifier.defaultMinSize(
                                    minWidth = KeynearSpacing.MinTouchTarget,
                                    minHeight = KeynearSpacing.MinTouchTarget,
                                ),
                                shape = RoundedCornerShape(KeynearRadii.Xl),
                                colors = ButtonDefaults.buttonColors(
                                    containerColor = palette.accent,
                                    contentColor = palette.onAccent,
                                ),
                            ) {
                                Text(text = stringResource(R.string.action_open_keynear))
                            }
                            OutlinedButton(
                                onClick = onClose,
                                modifier = Modifier.defaultMinSize(
                                    minWidth = KeynearSpacing.MinTouchTarget,
                                    minHeight = KeynearSpacing.MinTouchTarget,
                                ),
                                shape = RoundedCornerShape(KeynearRadii.Xl),
                            ) {
                                Text(
                                    text = stringResource(R.string.action_close),
                                    color = palette.textPrimary,
                                )
                            }
                        }
                    }
                }
            } else {
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(KeynearRadii.TwoXl),
                    colors = CardDefaults.cardColors(containerColor = palette.surface),
                ) {
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(KeynearSpacing.Lg),
                        verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
                    ) {
                        Text(
                            text = stringResource(R.string.share_select_device_label),
                            style = MaterialTheme.typography.titleMedium,
                            color = palette.textPrimary,
                        )
                        devices.forEach { dev ->
                            val selected = selectedDeviceId == dev.deviceId
                            Surface(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .defaultMinSize(minHeight = KeynearSpacing.MinTouchTarget)
                                    .clickable(
                                        enabled = !sending,
                                        role = Role.RadioButton,
                                    ) {
                                        selectedDeviceId = dev.deviceId
                                        sendError = null
                                    },
                                shape = RoundedCornerShape(KeynearRadii.Xl),
                                color = if (selected) palette.surfaceTonal else palette.surface,
                            ) {
                                Row(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .padding(KeynearSpacing.Md),
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    RadioButton(
                                        selected = selected,
                                        onClick = null,
                                        colors = RadioButtonDefaults.colors(selectedColor = palette.accent),
                                    )
                                    Spacer(modifier = Modifier.width(KeynearSpacing.Sm))
                                    Column(modifier = Modifier.weight(1f)) {
                                        Text(
                                            text = dev.displayName,
                                            style = MaterialTheme.typography.bodyLarge,
                                            color = palette.textPrimary,
                                            maxLines = 1,
                                            overflow = TextOverflow.Ellipsis,
                                        )
                                        Text(
                                            text = "${dev.role.label()} · ${dev.publicKeyFingerprint}",
                                            style = MaterialTheme.typography.labelMedium,
                                            color = palette.textSecondary,
                                            maxLines = 1,
                                            overflow = TextOverflow.Ellipsis,
                                        )
                                    }
                                }
                            }
                        }

                        if (sending) {
                            LinearProgressIndicator(
                                modifier = Modifier.fillMaxWidth(),
                                color = palette.accent,
                                trackColor = palette.surfaceTonal,
                            )
                            Text(
                                text = stringResource(R.string.share_sending_status),
                                style = MaterialTheme.typography.bodyMedium,
                                color = palette.textSecondary,
                            )
                        }

                        sendError?.let { err ->
                            Surface(
                                modifier = Modifier.fillMaxWidth(),
                                shape = RoundedCornerShape(KeynearRadii.Xl),
                                color = palette.errorTonal,
                            ) {
                                Text(
                                    text = err,
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = palette.onDangerTonal,
                                    modifier = Modifier.padding(KeynearSpacing.Md),
                                )
                            }
                        }

                        sentTransfer?.let { completed ->
                            Surface(
                                modifier = Modifier.fillMaxWidth(),
                                shape = RoundedCornerShape(KeynearRadii.Xl),
                                color = palette.surfaceTonal,
                            ) {
                                Text(
                                    text = "${completed.summaryName}: ${completed.state.label()}",
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = palette.textPrimary,
                                    modifier = Modifier.padding(KeynearSpacing.Md),
                                )
                            }
                        }

                        Row(horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm)) {
                            Button(
                                onClick = {
                                    val targetId = selectedDeviceId ?: return@Button
                                    sending = true
                                    sendError = null
                                    scope.launch {
                                        val result = withContext(Dispatchers.IO) {
                                            onSendToPeer(targetId)
                                        }
                                        sending = false
                                        result.onSuccess { tx ->
                                            sentTransfer = tx
                                        }.onFailure { err ->
                                            sendError = err.message ?: "Transfer failed"
                                        }
                                    }
                                },
                                enabled = !sending && selectedDeviceId != null,
                                modifier = Modifier.defaultMinSize(
                                    minWidth = KeynearSpacing.MinTouchTarget,
                                    minHeight = KeynearSpacing.MinTouchTarget,
                                ),
                                shape = RoundedCornerShape(KeynearRadii.Xl),
                                colors = ButtonDefaults.buttonColors(
                                    containerColor = palette.accent,
                                    contentColor = palette.onAccent,
                                ),
                            ) {
                                Text(text = stringResource(R.string.action_send))
                            }
                            OutlinedButton(
                                onClick = onClose,
                                modifier = Modifier.defaultMinSize(
                                    minWidth = KeynearSpacing.MinTouchTarget,
                                    minHeight = KeynearSpacing.MinTouchTarget,
                                ),
                                shape = RoundedCornerShape(KeynearRadii.Xl),
                            ) {
                                Text(
                                    text = stringResource(R.string.action_close),
                                    color = palette.textPrimary,
                                )
                            }
                        }
                    }
                }
            }
        }
        }
    }
}
