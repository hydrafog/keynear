package org.keynear.app.bridge

import android.content.ClipData
import android.content.ClipboardManager
import android.content.ContentValues
import android.content.Context
import android.net.Uri
import android.os.Environment
import android.os.Handler
import android.os.Looper
import android.provider.MediaStore
import android.util.Log
import org.keynear.app.ble.KeynearBleConstants
import org.keynear.app.ble.PairingNotifier
import org.keynear.app.data.DeviceRepo
import org.keynear.app.data.PairingRepo
import org.keynear.app.data.TransferRepo
import org.keynear.app.share.DirectShareManager
import org.keynear.app.ui.model.BluetoothStatus
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.CoreState
import org.keynear.app.ui.model.DaemonConfigUi
import org.keynear.app.ui.model.DiscoveredPeerUi
import org.keynear.app.ui.model.LocalNetworkStatus
import org.keynear.app.ui.model.PendingPairingUi
import org.keynear.app.ui.model.SendItemKind
import org.keynear.app.ui.model.TransferStateUi
import org.keynear.app.ui.model.PairedDeviceUi
import org.keynear.app.ui.model.TransferUi
import org.keynear.app.ui.model.SendQueueItemUi
import org.keynear.app.ui.model.VisibilityState
import org.keynear.app.ui.model.validateDisplayNameInput
import uniffi.keynear_core.CoreHandle
import uniffi.keynear_core.FfiClipboardItem
import uniffi.keynear_core.FfiDeviceRole
import uniffi.keynear_core.FfiLanEndpoint
import uniffi.keynear_core.FfiPairedDevice
import uniffi.keynear_core.OrionNodeHandle
import uniffi.keynear_core.defaultConfigSummary
import uniffi.keynear_core.everyoneNearbyTimeoutSecs
import java.io.File
import java.util.Locale
import java.util.concurrent.ConcurrentHashMap

internal fun EventQueue.getPairedDevices(): List<FfiPairedDevice> =
    DeviceRepo.getPairedDevices(handle, devicesPath)
internal fun EventQueue.snapshotFromCurrent(): CoreSnapshot? = when (val state = runCatching { buildState() }.getOrNull()) {
    is CoreState.Empty -> state.snapshot
    is CoreState.Ready -> state.snapshot
    else -> null
}
internal fun EventQueue.buildState(): CoreState = try {
    val status = handle.status()
    val visibility = VisibilityState.fromFfi(handle.visibilityMode())
    val remaining = handle.everyoneNearbyRemainingSecs().toLong()
    val rawPaired = getPairedDevices()
    val peerNames = rawPaired.associate { it.deviceId to it.displayName }
    val paired = DeviceRepo.mapPairedDevices(rawPaired)
    syncShortcutSnapshotIfNeeded(paired)
    val transfers = collectActiveTransfers(peerNames)
    decrementPendingPairing()
    val activeConfig = cachedDaemonConfig.copy(darkTheme = cachedDarkTheme)
    val queue = sendQueueStore.snapshot()
    val snapshot = CoreSnapshot(
        version = status.version,
        maxControlFrameBytes = status.maxControlFrameBytes.toLong(),
        ready = status.ready,
        visibility = visibility,
        configSummary = defaultConfigSummary(),
        everyoneRemainingSecs = remaining,
        pairedDevices = paired,
        nearbyPeers = cachedNearby,
        pendingPairing = cachedPendingPairing,
        bluetoothStatus = cachedBtStatus,
        warningMessage = cachedWarning,
        localNetworkStatus = cachedLnpStatus,
        localNetworkMessage = cachedLnpMessage,
        activeTransfers = transfers,
        daemonConfig = activeConfig,
        sendQueue = queue,
    )
    if (isSnapshotEmpty(paired, transfers, queue)) {
        CoreState.Empty(snapshot)
    } else {
        CoreState.Ready(snapshot)
    }
} catch (t: Throwable) {
    CoreState.Error(t.message ?: "Unable to initialize keynear core")
}

private fun EventQueue.syncShortcutSnapshotIfNeeded(paired: List<PairedDeviceUi>) {
    val shortcutSnapshot = paired.map { it.deviceId to it.displayName }
    appContext?.let { ctx ->
        if (lastSyncedShortcutSnapshot != shortcutSnapshot) {
            lastSyncedShortcutSnapshot = shortcutSnapshot
            DirectShareManager.syncShortcuts(ctx, paired)
        }
    }
}

private fun EventQueue.collectActiveTransfers(peerNames: Map<String, String>): List<TransferUi> =
    orionNode?.let { node ->
        runCatching { node.listTransfers() }.getOrDefault(emptyList())
            .map { TransferRepo.mapTransferSessionToUi(it, peerNames) }
    } ?: emptyList()

private fun EventQueue.decrementPendingPairing() {
    val pending = cachedPendingPairing?.let { prompt ->
        if (prompt.remainingSecs <= 1L) null else prompt.copy(remainingSecs = prompt.remainingSecs - 1L)
    }
    if (cachedPendingPairing != null) {
        cachedPendingPairing = pending
    }
}

private const val UI_PREFS_NAME = "keynear_ui_prefs"
private const val PREF_DARK_THEME = "dark_theme"

internal fun readDarkThemePref(context: Context): Boolean =
    context.getSharedPreferences(UI_PREFS_NAME, Context.MODE_PRIVATE)
        .getBoolean(PREF_DARK_THEME, false)

internal fun writeDarkThemePref(context: Context, enabled: Boolean) {
    context.getSharedPreferences(UI_PREFS_NAME, Context.MODE_PRIVATE)
        .edit()
        .putBoolean(PREF_DARK_THEME, enabled)
        .apply()
}

private fun EventQueue.isSnapshotEmpty(
    paired: List<PairedDeviceUi>,
    transfers: List<TransferUi>,
    queue: List<SendQueueItemUi>,
): Boolean = paired.isEmpty() && cachedNearby.isEmpty() && cachedPendingPairing == null &&
    transfers.isEmpty() && queue.isEmpty()

internal fun EventQueue.updateDaemonConfigImpl(transform: (DaemonConfigUi) -> DaemonConfigUi): CoreState = try {
    val updated = transform(cachedDaemonConfig)
    cachedDarkTheme = updated.darkTheme
    appContext?.let {
        writeDarkThemePref(it, updated.darkTheme)
    }
    val currentVisibility = VisibilityState.fromFfi(handle.visibilityMode())
    val path = configPath
    cachedDaemonConfig = if (path != null) {
        val saved = handle.saveDaemonConfig(path, updated.toFfi(currentVisibility))
        DaemonConfigUi.fromFfi(saved, updated.darkTheme)
    } else {
        updated
    }
    orionNode?.let { node ->
        node.setAutoAcceptFromPaired(cachedDaemonConfig.autoAcceptFromPaired)
        syncClipboardPolicyToNode(node)
    }
    buildState()
} catch (t: Throwable) {
    CoreState.Error(t.message ?: "Unable to save settings", snapshotFromCurrent())
}
internal fun EventQueue.renamePairedDeviceImpl(deviceId: String, newDisplayName: String): CoreState {
    val validated = validateDisplayNameInput(newDisplayName).getOrElse { err ->
        return CoreState.Error(err.message ?: "Invalid device name", snapshotFromCurrent())
    }
    val path = devicesPath ?: return CoreState.Error("Device store path unavailable", snapshotFromCurrent())
    return try {
        handle.renamePairedDevice(path, deviceId, validated)
        lastSyncedShortcutSnapshot = null
        buildState()
    } catch (t: Throwable) {
        CoreState.Error(t.message ?: "Unable to rename device", snapshotFromCurrent())
    }
}
internal fun EventQueue.setAutoAcceptFilesImpl(deviceId: String, enabled: Boolean): CoreState {
    val path = devicesPath ?: return CoreState.Error("Device store path unavailable")
    return try {
        handle.setAutoAcceptFiles(path, deviceId, enabled)
        syncAllPairedPeersToOrion()
        buildState()
    } catch (t: Throwable) {
        CoreState.Error(t.message ?: "Unable to update auto-accept preference", snapshotFromCurrent())
    }
}
