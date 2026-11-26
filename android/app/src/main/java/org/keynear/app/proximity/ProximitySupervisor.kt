package org.keynear.app.proximity

import androidx.annotation.RequiresApi

import android.content.Context
import android.os.Build
import android.ranging.RangingCapabilities
import android.ranging.RangingManager
import android.ranging.RangingPreference
import android.ranging.RangingSession
import android.ranging.raw.RawInitiatorRangingConfig
import android.ranging.raw.RawRangingDevice
import android.ranging.uwb.UwbAddress
import android.ranging.uwb.UwbRangingParams
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicLong
import org.keynear.app.bridge.CoreClientProvider
import org.keynear.app.data.PairingRepo
import org.keynear.app.ui.model.CoreState
import uniffi.keynear_core.FfiRangingSample
import uniffi.keynear_core.FfiRangingTechnology

@RequiresApi(36)
class ProximitySupervisor(
    private val context: Context,
    private val peerDeviceId: String,
    private val peerBluetoothAddress: String,
    private val sessionKeyInfo: ByteArray,
    private val onSample: (FfiRangingSample) -> Unit,
    private val onFallback: () -> Unit,
) {
    private val executor = Executors.newSingleThreadExecutor()
    private var session: RangingSession? = null
    private val bleSequence = AtomicLong(BLE_SEQUENCE_BASE)
    @Volatile
    private var fallbackActive = false
    @Volatile
    private var rssiSupplier: (() -> Int?)? = null

    private val capabilitiesCallback = object : RangingManager.RangingCapabilitiesCallback {
        override fun onRangingCapabilities(capabilities: RangingCapabilities) {
            val uwbAvailable = capabilities.uwbCapabilities?.isDistanceMeasurementSupported == true
            when (RangingDecision.selectTechnology(uwbAvailable)) {
                FfiRangingTechnology.UWB -> startUwbSession()
                else -> activateFallback()
            }
        }
    }

    fun start() {
        if (Build.VERSION.SDK_INT < RANGING_SDK_FLOOR) {
            activateFallback()
            return
        }
        val manager = context.getSystemService(RangingManager::class.java) ?: run {
            activateFallback()
            return
        }
        manager.registerCapabilitiesCallback(executor, capabilitiesCallback)
    }

    fun acceptBleRssi(rssiDbm: Int) {
        if (!fallbackActive) return
        onSample(
            FfiRangingSample(
                deviceId = peerDeviceId,
                technology = FfiRangingTechnology.BLE_RSSI,
                distanceMeters = null,
                rssiDbm = rssiDbm,
                sequence = bleSequence.incrementAndGet().toULong(),
            )
        )
    }

    fun refreshBleRssi() {
        rssiSupplier?.invoke()?.let(::acceptBleRssi)
    }

    private fun activateFallback() {
        if (fallbackActive) return
        fallbackActive = true
        onFallback()
        refreshBleRssi()
    }

    fun stop() {
        session?.stop()
        session = null
        fallbackActive = false
        rssiSupplier = null
        context.getSystemService(RangingManager::class.java)
            ?.unregisterCapabilitiesCallback(capabilitiesCallback)
        executor.shutdown()
    }

    private fun startUwbSession() {
        val manager = context.getSystemService(RangingManager::class.java) ?: return
        val callback = RangingCallback(peerDeviceId, onSample, ::activateFallback)
        val address = UwbAddress.fromBytes(parseMac(peerBluetoothAddress).copyOf(8))
        val params = UwbRangingParams.Builder(
            SESSION_ID,
            UwbRangingParams.CONFIG_UNICAST_DS_TWR,
            address,
            address,
        )
            .setSessionKeyInfo(sessionKeyInfo)
            .build()
        val device = RawRangingDevice.Builder()
            .setRangingDevice(android.ranging.RangingDevice.Builder().build())
            .setUwbRangingParams(params)
            .build()
        val config = RawInitiatorRangingConfig.Builder()
            .addRawRangingDevice(device)
            .build()
        val preference = RangingPreference.Builder(
            RangingPreference.DEVICE_ROLE_INITIATOR,
            config,
        ).build()
        session = manager.createRangingSession(executor, callback)
        if (session == null) {
            activateFallback()
            return
        }
        session?.start(preference)
    }

    companion object {
        private const val RANGING_SDK_FLOOR = 36
        private const val SESSION_ID = 0x4B4E
        private const val BLE_SEQUENCE_BASE = 1_000_000L

        fun startIfEnabled(
            context: Context,
            onRangingSample: (FfiRangingSample) -> Unit,
            measuredRssiDbm: () -> Int? = { null },
        ): ProximitySupervisor? {
            val client = CoreClientProvider.getClient()
            val snapshot = when (val state = client.loadState()) {
                is CoreState.Empty -> state.snapshot
                is CoreState.Ready -> state.snapshot
                else -> return null
            }
            if (!snapshot.daemonConfig.proximityLockEnabled) return null
            val paired = snapshot.pairedDevices.firstOrNull() ?: return null
            if (paired.deviceId.isBlank()) return null
            val secretStore = client.keystoreStore()
            val sessionKey = secretStore?.getPeerSecrets(paired.deviceId)?.reconnectSecret?.copyOf()
                ?: PairingRepo.getOrCreateQuicSecretKey(secretStore).copyOf()
            val supervisor = ProximitySupervisor(
                context = context.applicationContext,
                peerDeviceId = paired.deviceId,
                peerBluetoothAddress = paired.lastBleAddress,
                sessionKeyInfo = sessionKey,
                onSample = onRangingSample,
                onFallback = {},
            ).also { it.rssiSupplier = measuredRssiDbm }
            if (paired.lastBleAddress.isBlank()) {
                supervisor.activateFallback()
            } else {
                supervisor.start()
            }
            return supervisor
        }

        fun measuredRssi(peerDeviceId: String, bleAddress: String): Int? {
            val snapshot = when (val state = CoreClientProvider.getClient().loadState()) {
                is CoreState.Empty -> state.snapshot
                is CoreState.Ready -> state.snapshot
                else -> return null
            }
            snapshot.pairedDevices.firstOrNull { it.deviceId == peerDeviceId }?.rssi?.let { return it }
            val normalized = bleAddress.replace(":", "").replace("-", "").lowercase()
            if (normalized.isBlank()) return null
            return snapshot.nearbyPeers.firstOrNull {
                it.address.replace(":", "").replace("-", "").lowercase() == normalized
            }?.rssi
        }

        fun parseMac(value: String): ByteArray {
            val cleaned = value.replace(":", "").replace("-", "")
            require(cleaned.length == 12) { "peer Bluetooth address must be 6 bytes" }
            return ByteArray(6) { index ->
                val byte = cleaned.substring(index * 2, index * 2 + 2).toInt(16)
                (byte and 0xFF).toByte()
            }
        }
    }
}
