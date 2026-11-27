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
import org.keynear.designsystem.KeynearAccents
import org.keynear.designsystem.normalizeAccentHex
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
    refreshPendingPairingCountdown()
    val activeConfig = cachedDaemonConfig.copy(
        darkTheme = cachedDarkTheme,
        accentColorHex = cachedAccentColorHex,
    )
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

internal fun computeRemainingPairingSecs(
    prompt: PendingPairingUi?,
    deadlineMs: Long,
    nowMs: Long,
): PendingPairingUi? {
    if (prompt == null) return null
    val remainingMs = deadlineMs - nowMs
    if (remainingMs <= 0L) return null
    val remainingSecs = (remainingMs + 999L) / 1000L
    return if (prompt.remainingSecs == remainingSecs) prompt else prompt.copy(remainingSecs = remainingSecs)
}

internal fun EventQueue.setPendingPairingImpl(
    prompt: PendingPairingUi?,
    nowMs: Long = System.currentTimeMillis(),
) {
    if (prompt == null || prompt.remainingSecs <= 0L) {
        pendingPairingDeadlineMs = 0L
        cachedPendingPairing = null
    } else {
        pendingPairingDeadlineMs = nowMs + prompt.remainingSecs * 1000L
        cachedPendingPairing = prompt
    }
}

private fun EventQueue.refreshPendingPairingCountdown(nowMs: Long = System.currentTimeMillis()) {
    val current = cachedPendingPairing ?: return
    val updated = computeRemainingPairingSecs(current, pendingPairingDeadlineMs, nowMs)
    cachedPendingPairing = updated
    if (updated == null) {
        pendingPairingDeadlineMs = 0L
    }
}

private const val UI_PREFS_NAME = "keynear_ui_prefs"
private const val PREF_DARK_THEME = "dark_theme"
private const val PREF_ACCENT_COLOR = "accent_color_hex"
private const val PREF_DISMISSED_TRANSFERS = "dismissed_transfer_ids"

internal fun readDarkThemePref(context: Context): Boolean =
    context.getSharedPreferences(UI_PREFS_NAME, Context.MODE_PRIVATE)
        .getBoolean(PREF_DARK_THEME, false)

internal fun writeDarkThemePref(context: Context, enabled: Boolean) {
    context.getSharedPreferences(UI_PREFS_NAME, Context.MODE_PRIVATE)
        .edit()
        .putBoolean(PREF_DARK_THEME, enabled)
        .apply()
}

internal fun readAccentColorPref(context: Context): String =
    normalizeAccentHex(
        context.getSharedPreferences(UI_PREFS_NAME, Context.MODE_PRIVATE)
            .getString(PREF_ACCENT_COLOR, KeynearAccents.DEFAULT_HEX) ?: KeynearAccents.DEFAULT_HEX,
    )

internal fun writeAccentColorPref(context: Context, hex: String) {
    context.getSharedPreferences(UI_PREFS_NAME, Context.MODE_PRIVATE)
        .edit()
        .putString(PREF_ACCENT_COLOR, normalizeAccentHex(hex))
        .apply()
}

internal fun readDismissedTransferIds(context: Context): Set<String> =
    context.getSharedPreferences(UI_PREFS_NAME, Context.MODE_PRIVATE)
        .getStringSet(PREF_DISMISSED_TRANSFERS, emptySet())
        ?.toSet()
        .orEmpty()

internal fun writeDismissedTransferIds(context: Context, ids: Set<String>) {
    context.getSharedPreferences(UI_PREFS_NAME, Context.MODE_PRIVATE)
        .edit()
        .putStringSet(PREF_DISMISSED_TRANSFERS, ids.toSet())
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
    val normalizedAccent = normalizeAccentHex(updated.accentColorHex)
    cachedDarkTheme = updated.darkTheme
    cachedAccentColorHex = normalizedAccent
    appContext?.let {
        writeDarkThemePref(it, updated.darkTheme)
        writeAccentColorPref(it, normalizedAccent)
    }
    val currentVisibility = VisibilityState.fromFfi(handle.visibilityMode())
    val path = configPath
    cachedDaemonConfig = if (path != null) {
        val saved = handle.saveDaemonConfig(path, updated.toFfi(currentVisibility))
        DaemonConfigUi.fromFfi(saved, updated.darkTheme, normalizedAccent)
    } else {
        updated.copy(accentColorHex = normalizedAccent)
    }
    orionNode?.let { node ->
        node.setAutoAcceptFromPaired(cachedDaemonConfig.autoAcceptFromPaired)
        syncClipboardPolicyToNode(node)
        runCatching {
            node.syncProximityConfig(
                cachedDaemonConfig.proximityLockEnabled,
                cachedDaemonConfig.proximityDistanceMeters,
            )
        }.onFailure { err ->
            Log.w("EventQueueState", "sync_proximity_config_failed", err)
        }
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
