package org.keynear.app.core

import android.Manifest
import android.companion.CompanionDeviceManager
import android.content.ContentValues
import android.content.Context
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.os.ParcelFileDescriptor
import android.os.PowerManager
import android.provider.MediaStore
import android.provider.OpenableColumns
import android.util.Log
import androidx.core.content.ContextCompat
import org.keynear.app.ble.KeynearBleConstants
import org.keynear.app.ble.PairingNotificationManager
import org.keynear.app.lan.LocalNetworkConstants
import org.keynear.app.service.KeynearTransferJobService
import org.keynear.app.share.DirectShareManager
import uniffi.keynear_core.CoreHandle
import uniffi.keynear_core.FfiDaemonConfig
import uniffi.keynear_core.FfiDeviceRole
import uniffi.keynear_core.FfiLanEndpoint
import uniffi.keynear_core.FfiPairedDevice
import uniffi.keynear_core.FfiTransferDirection
import uniffi.keynear_core.FfiTransferSession
import uniffi.keynear_core.FfiTransferState
import uniffi.keynear_core.FfiVisibilityMode
import uniffi.keynear_core.OrionNodeHandle
import uniffi.keynear_core.defaultConfigSummary
import uniffi.keynear_core.everyoneNearbyTimeoutSecs
import java.io.File
import java.io.FileOutputStream
import java.security.SecureRandom
import java.util.Locale
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong
import kotlin.math.roundToInt


class UniffiCoreClient(
    private var appContext: Context? = null,
) : CoreBridge {
    private val handle: CoreHandle by lazy { CoreHandle() }
    private var devicesPath: String? = appContext?.let { File(it.filesDir, "devices.toml").absolutePath }
    private var configPath: String? = appContext?.let { File(it.filesDir, "config.toml").absolutePath }
    private var secretStore: KeystoreSecretStore? = appContext?.let { runCatching { KeystoreSecretStore.open(it) }.getOrNull() }
    private var localDeviceId: String = appContext?.let { resolveStableLocalDeviceId(it) } ?: "android-device"
    private var mediaStoreTracker: PendingMediaStoreTracker<Uri>? =
        appContext?.let { PendingMediaStoreTracker(AndroidMediaStoreOps(it.applicationContext)) }
    private val sendQueueStore = SendQueueStore()

    @Volatile
    private var cachedDarkTheme: Boolean = appContext?.let { readDarkThemePref(it) } ?: false

    @Volatile
    private var cachedDaemonConfig: DaemonConfigUi = DaemonConfigUi(darkTheme = cachedDarkTheme)

    @Volatile
    private var orionNode: OrionNodeHandle? = null

    @Volatile
    private var cachedNearby: List<DiscoveredPeerUi> = emptyList()

    @Volatile
    private var cachedPendingPairing: PendingPairingUi? = null

    @Volatile
    private var cachedBtStatus: BluetoothStatus = BluetoothStatus.READY

    @Volatile
    private var cachedWarning: String? = null

    @Volatile
    private var cachedLnpStatus: LocalNetworkStatus = LocalNetworkStatus.READY

    @Volatile
    private var cachedLnpMessage: String? = null

    private val notifiedConsentIds = ConcurrentHashMap.newKeySet<String>()
    private val notifiedTerminalIds = ConcurrentHashMap.newKeySet<String>()
    private val fdHandedOffIds = ConcurrentHashMap.newKeySet<String>()

    @Volatile
    private var lastSyncedShortcutSnapshot: List<Pair<String, String>>? = null

    @Synchronized
    fun attachContext(context: Context) {
        val app = context.applicationContext
        if (appContext == null) {
            appContext = app
        }
        if (devicesPath == null) {
            devicesPath = File(app.filesDir, "devices.toml").absolutePath
        }
        if (configPath == null) {
            configPath = File(app.filesDir, "config.toml").absolutePath
        }
        if (secretStore == null) {
            secretStore = runCatching { KeystoreSecretStore.open(app) }.getOrNull()
        }
        if (mediaStoreTracker == null) {
            mediaStoreTracker = PendingMediaStoreTracker(AndroidMediaStoreOps(app))
        }
        localDeviceId = resolveStableLocalDeviceId(app)
        cachedDarkTheme = readDarkThemePref(app)
        refreshConfigFromDisk()
    }

    private fun readDarkThemePref(context: Context): Boolean =
        context.getSharedPreferences(UI_PREFS_NAME, Context.MODE_PRIVATE)
            .getBoolean(PREF_DARK_THEME, false)

    private fun writeDarkThemePref(context: Context, enabled: Boolean) {
        context.getSharedPreferences(UI_PREFS_NAME, Context.MODE_PRIVATE)
            .edit()
            .putBoolean(PREF_DARK_THEME, enabled)
            .apply()
    }

    private fun refreshConfigFromDisk() {
        val path = configPath ?: return
        runCatching {
            val loaded = handle.loadDaemonConfig(path)
            handle.setVisibilityMode(loaded.visibilityMode)
            cachedDaemonConfig = DaemonConfigUi.fromFfi(loaded, cachedDarkTheme)
        }
    }

    fun coreHandle(): CoreHandle = handle

    fun keystoreStore(): KeystoreSecretStore? = secretStore

    fun devicesTomlPath(): String? = devicesPath

    fun localDeviceId(): String = localDeviceId

    fun localDisplayName(): String = android.os.Build.MODEL?.takeIf { it.isNotBlank() } ?: "Android"

    fun getOrCreateStaticPrivateKey(): ByteArray {
        val store = secretStore
        return if (store != null) {
            store.getOrCreateStaticPrivateKey { handle.generateStaticKeypair().privateKey }
        } else {
            handle.generateStaticKeypair().privateKey
        }
    }

    private fun getOrCreateQuicSecretKey(): ByteArray {
        val randomGen = {
            ByteArray(32).also { SecureRandom().nextBytes(it) }
        }
        val store = secretStore
        return if (store != null) {
            store.getOrCreateQuicSecretKey(randomGen)
        } else {
            randomGen()
        }
    }

    @Synchronized
    fun ensureOrionNode(context: Context): OrionNodeHandle? {
        attachContext(context)
        orionNode?.let { return it }
        val app = context.applicationContext
        val incomingDir = File(app.filesDir, "incoming").apply { mkdirs() }
        val quicKey = getOrCreateQuicSecretKey()
        return try {
            val node = OrionNodeHandle(
                deviceId = localDeviceId,
                displayName = localDisplayName(),
                capabilityFlags = KeynearBleConstants.ANDROID_CAPABILITY_FLAGS,
                quicSecretKey = quicKey,
                incomingDirectory = incomingDir.absolutePath,
                autoAcceptFromPaired = false,
                requireExplicitFdForSingleFile = true,
            )
            orionNode = node
            syncAllPairedPeersToOrion(node)
            node
        } catch (t: Throwable) {
            Log.w(TAG, "orion_node_start_failed: ${t.message}")
            null
        } finally {
            quicKey.fill(0)
        }
    }

    @Synchronized
    fun shutdownOrion() {
        mediaStoreTracker?.cleanupAllPending()
        orionNode?.let { node ->
            runCatching { node.shutdown() }
            runCatching { node.close() }
        }
        orionNode = null
        fdHandedOffIds.clear()
        notifiedConsentIds.clear()
        notifiedTerminalIds.clear()
    }

    fun localLanEndpoint(): FfiLanEndpoint? = orionNode?.localLanEndpoint()

    fun syncAllPairedPeersToOrion(node: OrionNodeHandle? = orionNode) {
        val activeNode = node ?: return
        val store = secretStore ?: return
        for (dev in getPairedDevices()) {
            val eidBytes = decodeEndpointIdHex(dev.lastEndpointId) ?: continue
            val secrets = store.getPeerSecrets(dev.deviceId) ?: continue
            try {
                val addrs = if (dev.lastLanAddress.isBlank()) emptyList() else listOf(dev.lastLanAddress)
                runCatching {
                    activeNode.syncPeer(
                        deviceId = dev.deviceId,
                        endpointId = eidBytes,
                        socketAddrs = addrs,
                        reconnectSecret = secrets.reconnectSecret,
                        autoAcceptFiles = dev.autoAcceptFiles,
                    )
                }
            } finally {
                secrets.zeroize()
            }
        }
    }

    fun encodeLocalLanEndpointForBleAddress(bleAddress: String): ByteArray? {
        val dev = getPairedDevices().firstOrNull { it.lastBleAddress.equals(bleAddress, ignoreCase = true) }
            ?: return null
        return encodeLocalLanEndpointForPeer(dev.deviceId)
    }

    fun encodeLocalLanEndpointForPeer(deviceId: String): ByteArray? =
        encodeLanEndpointForPeerHelper(handle, orionNode, secretStore, deviceId)

    fun onEncryptedLanEndpointFromBle(encoded: ByteArray, bleAddress: String? = null): FfiLanEndpoint? {
        val store = secretStore ?: return null
        val decoded = decodeEncryptedLanEndpointFromBle(handle, store, getPairedDevices(), encoded, bleAddress)
        if (decoded != null) {
            onPeerLanEndpointDiscovered(decoded.deviceId, decoded.endpointId, decoded.socketAddrs)
        }
        return decoded
    }

    fun onPeerLanEndpointDiscovered(
        deviceId: String,
        endpointId: ByteArray,
        socketAddrs: List<String>,
    ) {
        if (endpointId.size != 32) return
        val devices = getPairedDevices()
        val dev = devices.firstOrNull { it.deviceId == deviceId } ?: return
        val store = secretStore ?: return
        val secrets = store.getPeerSecrets(deviceId) ?: return
        try {
            orionNode?.syncPeer(
                deviceId = deviceId,
                endpointId = endpointId,
                socketAddrs = socketAddrs,
                reconnectSecret = secrets.reconnectSecret,
                autoAcceptFiles = dev.autoAcceptFiles,
            )
            val path = devicesPath
            val eidHex = encodeHex(endpointId)
            val firstAddr = socketAddrs.firstOrNull().orEmpty()
            if (path != null && (dev.lastEndpointId != eidHex || (firstAddr.isNotEmpty() && dev.lastLanAddress != firstAddr))) {
                runCatching {
                    handle.savePairedDevice(
                        devicesTomlPath = path,
                        device = dev.copy(
                            lastEndpointId = eidHex,
                            lastLanAddress = if (firstAddr.isNotEmpty()) firstAddr else dev.lastLanAddress,
                        ),
                    )
                }
            }
        } finally {
            secrets.zeroize()
        }
    }

    fun syncTransferStates(context: Context): Boolean {
        val app = context.applicationContext
        attachContext(app)
        val node = orionNode ?: return false
        val sessions = runCatching { node.listTransfers() }.getOrNull() ?: return false
        val peerNames = getPairedDevices().associate { it.deviceId to it.displayName }
        var changed = false
        var hasActiveTransfers = false

        for (session in sessions) {
            val uiState = TransferStateUi.fromFfi(session.state)
            val peerName = peerNames[session.peerDeviceId] ?: session.peerDeviceId
            val summary = summarizeTransferItems(session)

            if (session.direction == FfiTransferDirection.RECEIVE && uiState == TransferStateUi.PENDING_CONSENT) {
                if (notifiedConsentIds.add(session.transferId)) {
                    PairingNotificationManager.showIncomingTransferConsent(
                        context = app,
                        transferId = session.transferId,
                        peerName = peerName,
                        summary = summary,
                        sizeLabel = formatByteCount(session.totalBytes.toLong()),
                    )
                    changed = true
                }
            }

            if (uiState == TransferStateUi.ACTIVE) {
                hasActiveTransfers = true
                if (session.direction == FfiTransferDirection.RECEIVE && isSingleFileReceive(session) && !fdHandedOffIds.contains(session.transferId)) {
                    if (prepareAndHandOffReceiveFd(node, session)) {
                        changed = true
                    }
                }
                PairingNotificationManager.showOrUpdateTransferProgress(
                    context = app,
                    transfer = mapTransferSessionToUi(session, peerNames),
                )
            }

            mediaStoreTracker?.reconcileState(session.transferId, uiState)

            if (uiState.isTerminal()) {
                PairingNotificationManager.cancelTransferNotification(app, session.transferId)
                if (notifiedTerminalIds.add(session.transferId)) {
                    changed = true
                    if (session.direction == FfiTransferDirection.RECEIVE && uiState == TransferStateUi.COMPLETED) {
                        val autoAccepted = getPairedDevices()
                            .firstOrNull { it.deviceId == session.peerDeviceId }
                            ?.autoAcceptFiles == true
                        PairingNotificationManager.showTransferCompleted(
                            context = app,
                            peerName = peerName,
                            summary = summary,
                            autoAccepted = autoAccepted,
                        )
                    } else if (uiState == TransferStateUi.FAILED) {
                        PairingNotificationManager.showTransferError(
                            context = app,
                            summary = summary,
                            reason = "Connection interrupted or rejected",
                        )
                    }
                }
            }
        }
        KeynearTransferJobService.syncActiveTransfers(app, hasActiveTransfers)
        return changed
    }

    private fun isSingleFileReceive(session: FfiTransferSession): Boolean =
        session.items.size == 1 && !session.items[0].isDirectory

    private fun prepareAndHandOffReceiveFd(
        node: OrionNodeHandle,
        session: FfiTransferSession,
    ): Boolean {
        val tracker = mediaStoreTracker ?: return false
        val item = session.items.firstOrNull() ?: return false
        if (!fdHandedOffIds.add(session.transferId)) {
            return false
        }
        val ok = runCatching {
            tracker.prepareAndHandOff(
                transferId = session.transferId,
                fileName = item.name,
                mimeType = item.mimeType,
            ) { rawFd ->
                node.provideReceiveFd(session.transferId, rawFd)
            }
        }.getOrDefault(false)
        if (!ok) {
            fdHandedOffIds.remove(session.transferId)
        }
        return ok
    }

    fun saveCompletedPairing(
        peerDeviceId: String,
        peerDisplayName: String,
        peerRole: FfiDeviceRole,
        peerStaticPubkeyHex: String,
        peerCapabilityFlags: UInt,
        bleAddress: String,
        reconnectSecret: ByteArray,
    ) {
        val store = secretStore
        val path = devicesPath
        if (store != null) {
            store.putPeerSecrets(
                deviceId = peerDeviceId,
                record = PeerSecretRecord(
                    irk = reconnectSecret.copyOf(),
                    reconnectSecret = reconnectSecret.copyOf(),
                ),
            )
        }
        if (path != null) {
            try {
                handle.savePairedDevice(
                    devicesTomlPath = path,
                    device = FfiPairedDevice(
                        deviceId = peerDeviceId,
                        displayName = peerDisplayName,
                        deviceRole = peerRole,
                        staticX25519Pubkey = peerStaticPubkeyHex,
                        lastBleAddress = bleAddress,
                        lastLanAddress = "",
                        lastEndpointId = "",
                        capabilityFlags = peerCapabilityFlags,
                        autoAcceptFiles = false,
                        pairedAt = (System.currentTimeMillis() / 1000L).toString(),
                    ),
                )
            } catch (t: Throwable) {
                store?.deletePeerSecrets(peerDeviceId)
                throw t
            }
        }
    }

    override fun updateNearbyPeers(peers: List<DiscoveredPeerUi>) {
        cachedNearby = peers
    }

    override fun updateBluetoothStatus(status: BluetoothStatus, warning: String?) {
        cachedBtStatus = status
        cachedWarning = warning
    }

    override fun updateLocalNetworkStatus(status: LocalNetworkStatus, message: String?) {
        cachedLnpStatus = status
        cachedLnpMessage = message
    }

    override fun currentLocalNetworkStatus(): LocalNetworkStatus = cachedLnpStatus

    override fun setPendingPairing(prompt: PendingPairingUi?) {
        cachedPendingPairing = prompt
    }

    override fun loadState(): CoreState = buildState()

    override fun cycleVisibility(): CoreState {
        return try {
            handle.cycleVisibilityMode()
            val mode = VisibilityState.fromFfi(handle.visibilityMode())
            persistCurrentConfig(mode)
            buildState()
        } catch (t: Throwable) {
            CoreState.Error(t.message ?: "Unable to cycle visibility mode")
        }
    }

    override fun setVisibility(mode: VisibilityState): CoreState {
        return try {
            handle.setVisibilityMode(mode.toFfi())
            persistCurrentConfig(mode)
            buildState()
        } catch (t: Throwable) {
            CoreState.Error(t.message ?: "Unable to set visibility mode", snapshotFromCurrent())
        }
    }

    private fun persistCurrentConfig(visibility: VisibilityState) {
        val path = configPath ?: return
        runCatching {
            val saved = handle.saveDaemonConfig(path, cachedDaemonConfig.toFfi(visibility))
            cachedDaemonConfig = DaemonConfigUi.fromFfi(saved, cachedDarkTheme)
        }
    }

    override fun resetBleBond(deviceId: String): CoreState {
        return try {
            secretStore?.deletePeerSecrets(deviceId)
            buildState()
        } catch (t: Throwable) {
            CoreState.Error(t.message ?: "Unable to reset Bluetooth bond", snapshotFromCurrent())
        }
    }

    override fun stageFileUris(uris: List<String>): CoreState {
        val ctx = appContext
        uris.forEach { raw ->
            if (raw.isBlank()) return@forEach
            if (ctx != null) {
                val (name, size, mime) = queryContentUriMetadata(ctx, Uri.parse(raw))
                sendQueueStore.addFile(name, size, mime, raw)
            } else {
                val name = raw.substringAfterLast('/').ifBlank { "shared-file.bin" }
                sendQueueStore.addFile(name, 0L, "application/octet-stream", raw)
            }
        }
        return buildState()
    }

    override fun stageFolderUri(
        folderUri: String,
        displayName: String,
        fileUris: List<String>,
    ): CoreState {
        val ctx = appContext
        if (fileUris.isNotEmpty()) {
            fileUris.forEach { raw ->
                if (raw.isBlank()) return@forEach
                if (ctx != null) {
                    val (name, size, mime) = queryContentUriMetadata(ctx, Uri.parse(raw))
                    sendQueueStore.addFile(name, size, mime, raw)
                } else {
                    val name = raw.substringAfterLast('/').ifBlank { "shared-file.bin" }
                    sendQueueStore.addFile(name, 0L, "application/octet-stream", raw)
                }
            }
        } else if (folderUri.isNotBlank()) {
            sendQueueStore.addFolder(displayName, folderUri)
        }
        return buildState()
    }

    override fun stageClipboardText(text: String): CoreState {
        sendQueueStore.addClipboard(text)
        return buildState()
    }

    override fun stageTextSnippet(text: String): CoreState {
        sendQueueStore.addTextSnippet(text)
        return buildState()
    }

    override fun removeSendQueueItem(itemId: String): CoreState {
        sendQueueStore.remove(itemId)
        return buildState()
    }

    override fun clearSendQueue(): CoreState {
        sendQueueStore.clear()
        return buildState()
    }

    override fun sendQueueToPeer(peerDeviceId: String): Result<TransferUi> {
        val items = sendQueueStore.snapshot()
        if (items.isEmpty()) {
            return Result.failure(IllegalArgumentException("Stage at least one file or snippet first"))
        }
        val uris = items.mapNotNull { item ->
            item.uriString.takeIf { it.isNotBlank() && item.kind != SendItemKind.FOLDER }
        }
        val textCombined = items
            .mapNotNull { it.textPayload.takeIf { txt -> txt.isNotEmpty() } }
            .joinToString(separator = "\n\n")
            .ifBlank { null }
        val result = sendContentUris(peerDeviceId, uris, textCombined)
        if (result.isSuccess) {
            sendQueueStore.clear()
        }
        return result
    }

    override fun updateDaemonConfig(transform: (DaemonConfigUi) -> DaemonConfigUi): CoreState {
        return try {
            val updated = transform(cachedDaemonConfig)
            cachedDarkTheme = updated.darkTheme
            appContext?.let { writeDarkThemePref(it, updated.darkTheme) }
            val currentVisibility = VisibilityState.fromFfi(handle.visibilityMode())
            val path = configPath
            cachedDaemonConfig = if (path != null) {
                val saved = handle.saveDaemonConfig(path, updated.toFfi(currentVisibility))
                DaemonConfigUi.fromFfi(saved, updated.darkTheme)
            } else {
                updated
            }
            buildState()
        } catch (t: Throwable) {
            CoreState.Error(t.message ?: "Unable to save settings", snapshotFromCurrent())
        }
    }

    override fun tick(): CoreState = buildState()

    override fun unpairDevice(deviceId: String): CoreState {
        return try {
            devicesPath?.let { handle.removePairedDevice(it, deviceId) }
            secretStore?.deletePeerSecrets(deviceId)
            runCatching { orionNode?.removePeer(deviceId) }
            lastSyncedShortcutSnapshot = null
            buildState()
        } catch (t: Throwable) {
            CoreState.Error(t.message ?: "Unable to unpair device")
        }
    }

    override fun renamePairedDevice(deviceId: String, newDisplayName: String): CoreState {
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

    override fun setAutoAcceptFiles(deviceId: String, enabled: Boolean): CoreState {
        val path = devicesPath ?: return CoreState.Error("Device store path unavailable")
        return try {
            handle.setAutoAcceptFiles(path, deviceId, enabled)
            syncAllPairedPeersToOrion()
            buildState()
        } catch (t: Throwable) {
            CoreState.Error(t.message ?: "Unable to update auto-accept preference", snapshotFromCurrent())
        }
    }

    fun acceptIncomingTransferOrSync(context: Context, transferId: String): CoreState {
        attachContext(context)
        return acceptTransfer(transferId)
    }

    override fun acceptTransfer(transferId: String): CoreState {
        val node = orionNode ?: return CoreState.Error("Orion transfer node is not running", snapshotFromCurrent())
        return try {
            val session = node.respondConsent(transferId, true)
            appContext?.let { PairingNotificationManager.cancelTransferNotification(it, transferId) }
            if (isSingleFileReceive(session)) {
                val handedOff = prepareAndHandOffReceiveFd(node, session)
                if (!handedOff) {
                    runCatching { node.cancelTransfer(transferId) }
                    return CoreState.Error("Unable to open MediaStore download output", snapshotFromCurrent())
                }
            }
            buildState()
        } catch (t: Throwable) {
            CoreState.Error(t.message ?: "Unable to accept transfer", snapshotFromCurrent())
        }
    }

    override fun declineTransfer(transferId: String): CoreState {
        val node = orionNode ?: return buildState()
        return try {
            node.respondConsent(transferId, false)
            appContext?.let { PairingNotificationManager.cancelTransferNotification(it, transferId) }
            mediaStoreTracker?.reconcileState(transferId, TransferStateUi.CANCELLED)
            buildState()
        } catch (t: Throwable) {
            CoreState.Error(t.message ?: "Unable to decline transfer", snapshotFromCurrent())
        }
    }

    override fun cancelTransfer(transferId: String): CoreState {
        val node = orionNode ?: return buildState()
        return try {
            node.cancelTransfer(transferId)
            appContext?.let { PairingNotificationManager.cancelTransferNotification(it, transferId) }
            mediaStoreTracker?.reconcileState(transferId, TransferStateUi.CANCELLED)
            buildState()
        } catch (t: Throwable) {
            CoreState.Error(t.message ?: "Unable to cancel transfer", snapshotFromCurrent())
        }
    }

    override fun sendContentUris(
        peerDeviceId: String,
        uris: List<String>,
        sharedText: String?,
    ): Result<TransferUi> {
        val ctx = appContext ?: return Result.failure(IllegalStateException("Context not attached"))
        val node = ensureOrionNode(ctx)
            ?: return Result.failure(IllegalStateException("Local transfer node could not start"))
        syncAllPairedPeersToOrion(node)

        val peerNames = getPairedDevices().associate { it.deviceId to it.displayName }
        return sendContentUrisViaOrion(
            context = ctx,
            node = node,
            peerDeviceId = peerDeviceId,
            uris = uris,
            sharedText = sharedText,
            peerNames = peerNames,
        )
    }

    fun getPairedDevices(): List<FfiPairedDevice> {
        val path = devicesPath ?: return emptyList()
        return runCatching { handle.loadPairedDevices(path) }.getOrDefault(emptyList())
    }

    fun timeoutSecs(): Long = runCatching { everyoneNearbyTimeoutSecs().toLong() }.getOrDefault(600L)

    private fun snapshotFromCurrent(): CoreSnapshot? = when (val state = runCatching { buildState() }.getOrNull()) {
        is CoreState.Empty -> state.snapshot
        is CoreState.Ready -> state.snapshot
        else -> null
    }

    private fun buildState(): CoreState = try {
        val status = handle.status()
        val visibility = VisibilityState.fromFfi(handle.visibilityMode())
        val remaining = handle.everyoneNearbyRemainingSecs().toLong()
        val rawPaired = getPairedDevices()
        val peerNames = rawPaired.associate { it.deviceId to it.displayName }
        val paired = rawPaired.map { dev ->
            val online = dev.lastLanAddress.isNotBlank()
            PairedDeviceUi(
                deviceId = dev.deviceId,
                displayName = dev.displayName,
                role = DeviceRoleUi.fromFfi(dev.deviceRole),
                publicKeyFingerprint = dev.staticX25519Pubkey.take(12),
                lastBleAddress = dev.lastBleAddress,
                pairedAt = dev.pairedAt,
                autoAcceptFiles = dev.autoAcceptFiles,
                lastEndpointId = dev.lastEndpointId,
                lastLanAddress = dev.lastLanAddress,
                isOnline = online,
                rssi = if (online) -52 else -76,
            )
        }
        val shortcutSnapshot = paired.map { it.deviceId to it.displayName }
        appContext?.let { ctx ->
            if (lastSyncedShortcutSnapshot != shortcutSnapshot) {
                lastSyncedShortcutSnapshot = shortcutSnapshot
                DirectShareManager.syncShortcuts(ctx, paired)
            }
        }
        val transfers = orionNode?.let { node ->
            runCatching { node.listTransfers() }.getOrDefault(emptyList())
                .map { mapTransferSessionToUi(it, peerNames) }
        } ?: emptyList()

        val pending = cachedPendingPairing?.let { prompt ->
            if (prompt.remainingSecs <= 1L) null else prompt.copy(remainingSecs = prompt.remainingSecs - 1L)
        }
        if (cachedPendingPairing != null) {
            cachedPendingPairing = pending
        }
        val activeConfig = configPath?.let { path ->
            runCatching {
                DaemonConfigUi.fromFfi(handle.loadDaemonConfig(path), cachedDarkTheme)
            }.getOrNull()
        }?.also { cachedDaemonConfig = it } ?: cachedDaemonConfig.copy(darkTheme = cachedDarkTheme)

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
        if (paired.isEmpty() && cachedNearby.isEmpty() && cachedPendingPairing == null && transfers.isEmpty() && queue.isEmpty()) {
            CoreState.Empty(snapshot)
        } else {
            CoreState.Ready(snapshot)
        }
    } catch (t: Throwable) {
        CoreState.Error(t.message ?: "Unable to initialize keynear core")
    }

    private fun decodeEndpointIdHex(hex: String): ByteArray? = EndpointHexCodec.decode32(hex)

    private fun encodeHex(bytes: ByteArray): String = EndpointHexCodec.encode(bytes)

    companion object {
        private const val TAG = "UniffiCoreClient"
        private const val UI_PREFS_NAME = "keynear_ui_prefs"
        private const val PREF_DARK_THEME = "dark_theme"
    }
}
