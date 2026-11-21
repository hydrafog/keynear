package org.keynear.app.share

import android.content.Intent
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.keynear.app.MainActivity
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.CoreState
import org.keynear.app.ui.model.ShareValidationResult
import org.keynear.designsystem.KeynearTheme

@Composable
internal fun ShareActivityContent(
    client: org.keynear.app.bridge.EventQueue,
    validation: ShareValidationResult,
    streamUris: List<String>,
    sharedText: String?,
    preselectedDeviceId: String?,
    onOpenMain: () -> Unit,
    onClose: () -> Unit,
) {
    val appContext = androidx.compose.ui.platform.LocalContext.current.applicationContext
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
        DirectShareManager.syncShortcuts(appContext, devices)
        totalSizeBytes = IntentResolver.computeTotalShareBytes(appContext, streamUris, sharedText)
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
        onOpenMain = onOpenMain,
        onClose = onClose,
    )
}
}
