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
import org.keynear.app.ui.model.TransferUi
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

typealias UniffiCoreClient = EventQueue

class EventQueue(
    internal var appContext: Context? = null,
) : CoreBridge {
    internal val handle: CoreHandle by lazy { CoreHandle() }
    internal var devicesPath: String? = appContext?.let { File(it.filesDir, "devices.toml").absolutePath }
    internal var configPath: String? = appContext?.let { File(it.filesDir, "config.toml").absolutePath }
    internal var secretStore: KeystoreSecretStore? = appContext?.let { runCatching { KeystoreSecretStore.open(it) }.getOrNull() }
    internal var localDeviceId: String = appContext?.let { DeviceRepo.resolveStableLocalDeviceId(it) } ?: "android-device"
    internal var mediaStoreTracker: PendingMediaStoreTracker<Uri>? =
        appContext?.let { PendingMediaStoreTracker(AndroidMediaStoreOps(it.applicationContext)) }
    internal val sendQueueStore = SendQueueStore()

    @Volatile
    internal var cachedDarkTheme: Boolean = appContext?.let { readDarkThemePref(it) } ?: false

    @Volatile
    internal var cachedDynamicAccent: Boolean = appContext?.let { readDynamicAccentPref(it) } ?: true

    @Volatile
    internal var cachedDaemonConfig: DaemonConfigUi = DaemonConfigUi(
        darkTheme = cachedDarkTheme,
        dynamicAccent = cachedDynamicAccent,
    )

    @Volatile
    internal var orionNode: OrionNodeHandle? = null

    @Volatile
    internal var cachedNearby: List<DiscoveredPeerUi> = emptyList()

    @Volatile
    internal var cachedPendingPairing: PendingPairingUi? = null

    @Volatile
    internal var cachedBtStatus: BluetoothStatus = BluetoothStatus.READY

    @Volatile
    internal var cachedWarning: String? = null

    @Volatile
    internal var cachedLnpStatus: LocalNetworkStatus = LocalNetworkStatus.READY

    @Volatile
    internal var cachedLnpMessage: String? = null

    internal val notifiedConsentIds = ConcurrentHashMap.newKeySet<String>()
    internal val notifiedTerminalIds = ConcurrentHashMap.newKeySet<String>()
    internal val fdHandedOffIds = ConcurrentHashMap.newKeySet<String>()

    @Volatile
    internal var lastSyncedShortcutSnapshot: List<Pair<String, String>>? = null

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
        localDeviceId = DeviceRepo.resolveStableLocalDeviceId(app)
        cachedDarkTheme = readDarkThemePref(app)
        cachedDynamicAccent = readDynamicAccentPref(app)
        refreshConfigFromDisk()
    }

    internal fun refreshConfigFromDisk() {
        val path = configPath ?: return
        runCatching {
            val loaded = handle.loadDaemonConfig(path)
            handle.setVisibilityMode(loaded.visibilityMode)
            cachedDaemonConfig = DaemonConfigUi.fromFfi(loaded, cachedDarkTheme, cachedDynamicAccent)
        }
    }

    fun coreHandle(): CoreHandle = handle

    fun keystoreStore(): KeystoreSecretStore? = secretStore

    fun devicesTomlPath(): String? = devicesPath

    fun localDeviceId(): String = localDeviceId

    fun localDisplayName(): String = android.os.Build.MODEL?.takeIf { it.isNotBlank() } ?: "Android"

    fun getOrCreateStaticPrivateKey(): ByteArray =
        PairingRepo.getOrCreateStaticPrivateKey(handle, secretStore)

    @Synchronized
    fun ensureOrionNode(context: Context): OrionNodeHandle? {
        attachContext(context)
        orionNode?.let { return it }
        val app = context.applicationContext
        val incomingDir = File(app.filesDir, "incoming").apply { mkdirs() }
        val quicKey = PairingRepo.getOrCreateQuicSecretKey(secretStore)
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
            syncClipboardPolicyToNode(node)
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

    fun isOtpPassthroughEnabled(): Boolean = cachedDaemonConfig.otpPassthroughEnabled

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

    override fun cycleVisibility(): CoreState = try {
        handle.cycleVisibilityMode()
        val mode = VisibilityState.fromFfi(handle.visibilityMode())
        persistCurrentConfig(mode)
        buildState()
    } catch (t: Throwable) {
        CoreState.Error(t.message ?: "Unable to cycle visibility mode")
    }

    override fun setVisibility(mode: VisibilityState): CoreState = try {
        handle.setVisibilityMode(mode.toFfi())
        persistCurrentConfig(mode)
        buildState()
    } catch (t: Throwable) {
        CoreState.Error(t.message ?: "Unable to set visibility mode", snapshotFromCurrent())
    }

    internal fun persistCurrentConfig(visibility: VisibilityState) {
        val path = configPath ?: return
        runCatching {
            val saved = handle.saveDaemonConfig(path, cachedDaemonConfig.toFfi(visibility))
            cachedDaemonConfig = DaemonConfigUi.fromFfi(saved, cachedDarkTheme, cachedDynamicAccent)
        }
    }

    override fun resetBleBond(deviceId: String): CoreState = try {
        secretStore?.deletePeerSecrets(deviceId)
        buildState()
    } catch (t: Throwable) {
        CoreState.Error(t.message ?: "Unable to reset Bluetooth bond", snapshotFromCurrent())
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

    override fun tick(): CoreState = buildState()

    fun timeoutSecs(): Long = runCatching { everyoneNearbyTimeoutSecs().toLong() }.getOrDefault(DEFAULT_TIMEOUT_SECS)

    override fun sendPhoneClipboardText(text: String): Result<Unit> = sendPhoneClipboardTextImpl(text)
    override fun sendPhoneClipboardBlob(mimeType: String, data: ByteArray): Result<Unit> = sendPhoneClipboardBlobImpl(mimeType, data)
    override fun sendVerifiedOtp(code: String): Result<Unit> = sendVerifiedOtpImpl(code)
    override fun stageFileUris(uris: List<String>): CoreState = stageFileUrisImpl(uris)
    override fun stageFolderUri(
        folderUri: String,
        displayName: String,
        fileUris: List<String>,
    ): CoreState = stageFolderUriImpl(folderUri, displayName, fileUris)
    override fun sendQueueToPeer(peerDeviceId: String): Result<TransferUi> = sendQueueToPeerImpl(peerDeviceId)
    override fun updateDaemonConfig(transform: (DaemonConfigUi) -> DaemonConfigUi): CoreState = updateDaemonConfigImpl(transform)
    override fun renamePairedDevice(deviceId: String, newDisplayName: String): CoreState = renamePairedDeviceImpl(deviceId, newDisplayName)
    override fun setAutoAcceptFiles(deviceId: String, enabled: Boolean): CoreState = setAutoAcceptFilesImpl(deviceId, enabled)
    override fun acceptTransfer(transferId: String): CoreState = acceptTransferImpl(transferId)
    override fun declineTransfer(transferId: String): CoreState = declineTransferImpl(transferId)
    override fun cancelTransfer(transferId: String): CoreState = cancelTransferImpl(transferId)
    override fun sendContentUris(
        peerDeviceId: String,
        uris: List<String>,
        sharedText: String?,
    ): Result<TransferUi> = sendContentUrisImpl(peerDeviceId, uris, sharedText)

    companion object {
        private const val TAG = "EventQueue"
        private const val DEFAULT_TIMEOUT_SECS = 600L
    }
}
