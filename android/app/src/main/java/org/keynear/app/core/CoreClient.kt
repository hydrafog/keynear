package org.keynear.app.core

import android.content.Context
import uniffi.keynear_core.CoreHandle
import uniffi.keynear_core.FfiDeviceRole
import uniffi.keynear_core.FfiPairedDevice
import uniffi.keynear_core.FfiVisibilityMode
import uniffi.keynear_core.defaultConfigSummary
import uniffi.keynear_core.everyoneNearbyTimeoutSecs
import java.io.File
import java.util.UUID

enum class VisibilityState {
    HIDDEN,
    PAIRED_ONLY,
    EVERYONE_NEARBY;

    fun next(): VisibilityState = when (this) {
        HIDDEN -> PAIRED_ONLY
        PAIRED_ONLY -> EVERYONE_NEARBY
        EVERYONE_NEARBY -> HIDDEN
    }

    fun label(): String = when (this) {
        HIDDEN -> "Hidden"
        PAIRED_ONLY -> "Paired devices only"
        EVERYONE_NEARBY -> "Everyone nearby"
    }

    fun formattedStatus(remainingSecs: Long): String = when (this) {
        HIDDEN -> "Hidden"
        PAIRED_ONLY -> "Paired devices only"
        EVERYONE_NEARBY -> {
            val clamped = remainingSecs.coerceAtLeast(0L)
            val mins = clamped / 60L
            val secs = clamped % 60L
            "Everyone (%02d:%02d)".format(mins, secs)
        }
    }

    fun toFfi(): FfiVisibilityMode = when (this) {
        HIDDEN -> FfiVisibilityMode.HIDDEN
        PAIRED_ONLY -> FfiVisibilityMode.PAIRED_ONLY
        EVERYONE_NEARBY -> FfiVisibilityMode.EVERYONE_NEARBY
    }

    companion object {
        fun fromFfi(mode: FfiVisibilityMode): VisibilityState = when (mode) {
            FfiVisibilityMode.HIDDEN -> HIDDEN
            FfiVisibilityMode.PAIRED_ONLY -> PAIRED_ONLY
            FfiVisibilityMode.EVERYONE_NEARBY -> EVERYONE_NEARBY
        }
    }
}

enum class BluetoothStatus {
    READY,
    MISSING_PERMISSION,
    ADAPTER_DISABLED,
    UNAVAILABLE,
}

enum class DeviceRoleUi {
    DESKTOP,
    MOBILE;

    fun label(): String = when (this) {
        DESKTOP -> "Desktop"
        MOBILE -> "Mobile"
    }

    companion object {
        fun fromFfi(role: FfiDeviceRole): DeviceRoleUi = when (role) {
            FfiDeviceRole.DESKTOP -> DESKTOP
            FfiDeviceRole.MOBILE -> MOBILE
        }
    }
}

data class DiscoveredPeerUi(
    val address: String,
    val displayName: String,
    val role: DeviceRoleUi,
    val rssi: Int,
    val capabilityFlags: Long,
    val isPaired: Boolean,
    val batteryLevel: Int? = null,
)

data class PairedDeviceUi(
    val deviceId: String,
    val displayName: String,
    val role: DeviceRoleUi,
    val publicKeyFingerprint: String,
    val lastBleAddress: String,
    val pairedAt: String,
)

data class PendingPairingUi(
    val peerAddress: String,
    val peerName: String,
    val verificationCode: String,
    val remainingSecs: Long,
)

data class CoreSnapshot(
    val version: String,
    val maxControlFrameBytes: Long,
    val ready: Boolean,
    val visibility: VisibilityState,
    val configSummary: String,
    val everyoneRemainingSecs: Long = 0L,
    val pairedDevices: List<PairedDeviceUi> = emptyList(),
    val nearbyPeers: List<DiscoveredPeerUi> = emptyList(),
    val pendingPairing: PendingPairingUi? = null,
    val bluetoothStatus: BluetoothStatus = BluetoothStatus.READY,
    val warningMessage: String? = null,
)

sealed interface CoreState {
    data object Loading : CoreState
    data class Empty(val snapshot: CoreSnapshot) : CoreState
    data class Ready(val snapshot: CoreSnapshot) : CoreState
    data class Error(val message: String, val previous: CoreSnapshot? = null) : CoreState
}

sealed interface ShareValidationResult {
    data class Valid(val itemCount: Int, val hasText: Boolean) : ShareValidationResult
    data class Invalid(val reason: String) : ShareValidationResult
}

interface CoreBridge {
    fun loadState(): CoreState
    fun cycleVisibility(): CoreState
    fun tick(): CoreState = loadState()
    fun updateNearbyPeers(peers: List<DiscoveredPeerUi>) {}
    fun updateBluetoothStatus(status: BluetoothStatus, warning: String? = null) {}
    fun setPendingPairing(prompt: PendingPairingUi?) {}
    fun unpairDevice(deviceId: String): CoreState = loadState()
}

class VisibilityCountdownTracker(
    private val timeoutSecs: Long = 600L,
) {
    private var mode: VisibilityState = VisibilityState.PAIRED_ONLY
    private var remainingSecs: Long = 0L

    @Synchronized
    fun currentMode(): VisibilityState = mode

    @Synchronized
    fun remainingSecs(): Long = remainingSecs

    @Synchronized
    fun setMode(next: VisibilityState) {
        mode = next
        remainingSecs = if (next == VisibilityState.EVERYONE_NEARBY) timeoutSecs else 0L
    }

    @Synchronized
    fun cycle(): VisibilityState {
        setMode(mode.next())
        return mode
    }

    @Synchronized
    fun tick(): VisibilityState {
        if (mode == VisibilityState.EVERYONE_NEARBY) {
            if (remainingSecs <= 1L) {
                mode = VisibilityState.PAIRED_ONLY
                remainingSecs = 0L
            } else {
                remainingSecs -= 1L
            }
        }
        return mode
    }
}

private val ALLOWED_SHARE_SCHEMES = setOf("content", "file")

fun validateShareIntent(
    action: String?,
    streamUris: List<String>,
    sharedText: String?,
): ShareValidationResult {
    if (action != "android.intent.action.SEND" && action != "android.intent.action.SEND_MULTIPLE") {
        return ShareValidationResult.Invalid("unsupported_action")
    }

    val normalizedText = sharedText?.trim()?.takeIf { it.isNotEmpty() }
    if (streamUris.isEmpty() && normalizedText == null) {
        return ShareValidationResult.Invalid("missing_payload")
    }

    for (rawUri in streamUris) {
        val trimmed = rawUri.trim()
        val colonIndex = trimmed.indexOf(':')
        if (colonIndex <= 0) {
            return ShareValidationResult.Invalid("missing_uri_scheme")
        }
        val scheme = trimmed.substring(0, colonIndex).lowercase()
        if (scheme !in ALLOWED_SHARE_SCHEMES) {
            return ShareValidationResult.Invalid("disallowed_uri_scheme")
        }
    }

    return ShareValidationResult.Valid(
        itemCount = streamUris.size,
        hasText = normalizedText != null,
    )
}

class UniffiCoreClient(
    private val appContext: Context? = null,
) : CoreBridge {
    private val handle: CoreHandle by lazy { CoreHandle() }
    private var devicesPath: String? = appContext?.let { File(it.filesDir, "devices.toml").absolutePath }
    private var secretStore: KeystoreSecretStore? = appContext?.let { runCatching { KeystoreSecretStore.open(it) }.getOrNull() }
    private var localDeviceId: String = appContext?.let { resolveLocalDeviceId(it) } ?: "android-device"

    @Volatile
    private var cachedNearby: List<DiscoveredPeerUi> = emptyList()

    @Volatile
    private var cachedPendingPairing: PendingPairingUi? = null

    @Volatile
    private var cachedBtStatus: BluetoothStatus = BluetoothStatus.READY

    @Volatile
    private var cachedWarning: String? = null

    @Synchronized
    fun attachContext(context: Context) {
        if (devicesPath == null) {
            devicesPath = File(context.filesDir, "devices.toml").absolutePath
        }
        if (secretStore == null) {
            secretStore = runCatching { KeystoreSecretStore.open(context) }.getOrNull()
        }
        localDeviceId = resolveLocalDeviceId(context)
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

    override fun setPendingPairing(prompt: PendingPairingUi?) {
        cachedPendingPairing = prompt
    }

    override fun loadState(): CoreState = buildState()

    override fun cycleVisibility(): CoreState {
        return try {
            handle.cycleVisibilityMode()
            buildState()
        } catch (t: Throwable) {
            CoreState.Error(t.message ?: "Unable to cycle visibility mode")
        }
    }

    override fun tick(): CoreState = buildState()

    override fun unpairDevice(deviceId: String): CoreState {
        return try {
            devicesPath?.let { handle.removePairedDevice(it, deviceId) }
            secretStore?.deletePeerSecrets(deviceId)
            buildState()
        } catch (t: Throwable) {
            CoreState.Error(t.message ?: "Unable to unpair device")
        }
    }

    fun getPairedDevices(): List<FfiPairedDevice> {
        val path = devicesPath ?: return emptyList()
        return runCatching { handle.loadPairedDevices(path) }.getOrDefault(emptyList())
    }

    fun timeoutSecs(): Long = runCatching { everyoneNearbyTimeoutSecs().toLong() }.getOrDefault(600L)

    private fun buildState(): CoreState = try {
        val status = handle.status()
        val visibility = VisibilityState.fromFfi(handle.visibilityMode())
        val remaining = handle.everyoneNearbyRemainingSecs().toLong()
        val paired = getPairedDevices().map { dev ->
            PairedDeviceUi(
                deviceId = dev.deviceId,
                displayName = dev.displayName,
                role = DeviceRoleUi.fromFfi(dev.deviceRole),
                publicKeyFingerprint = dev.staticX25519Pubkey.take(12),
                lastBleAddress = dev.lastBleAddress,
                pairedAt = dev.pairedAt,
            )
        }
        val pending = cachedPendingPairing?.let { prompt ->
            if (prompt.remainingSecs <= 1L) null else prompt.copy(remainingSecs = prompt.remainingSecs - 1L)
        }
        if (cachedPendingPairing != null) {
            cachedPendingPairing = pending
        }
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
        )
        if (paired.isEmpty() && cachedNearby.isEmpty() && cachedPendingPairing == null) {
            CoreState.Empty(snapshot)
        } else {
            CoreState.Ready(snapshot)
        }
    } catch (t: Throwable) {
        CoreState.Error(t.message ?: "Unable to initialize keynear core")
    }

    private fun resolveLocalDeviceId(context: Context): String {
        val prefs = context.getSharedPreferences("keynear_device_identity", Context.MODE_PRIVATE)
        val existing = prefs.getString("device_id", null)
        if (!existing.isNullOrBlank()) {
            return existing
        }
        val generated = "android-${UUID.randomUUID().toString().take(12)}"
        prefs.edit().putString("device_id", generated).apply()
        return generated
    }
}

object CoreClientProvider {
    @Volatile
    private var instance: UniffiCoreClient? = null

    fun init(context: Context): UniffiCoreClient {
        val client = getInternal()
        client.attachContext(context.applicationContext)
        return client
    }

    fun get(): CoreBridge = getInternal()

    fun getClient(): UniffiCoreClient = getInternal()

    private fun getInternal(): UniffiCoreClient {
        val current = instance
        if (current != null) return current
        return synchronized(this) {
            instance ?: UniffiCoreClient().also { instance = it }
        }
    }
}
