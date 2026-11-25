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
import uniffi.keynear_core.FfiRangingSample

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

    private val capabilitiesCallback = object : RangingManager.RangingCapabilitiesCallback {
        override fun onRangingCapabilities(capabilities: RangingCapabilities) {
            if (capabilities.uwbCapabilities?.isDistanceMeasurementSupported == true) {
                startUwbSession()
            } else {
                onFallback()
            }
        }
    }

    fun start() {
        if (Build.VERSION.SDK_INT < RANGING_SDK_FLOOR) {
            onFallback()
            return
        }
        val manager = context.getSystemService(RangingManager::class.java) ?: run {
            onFallback()
            return
        }
        manager.registerCapabilitiesCallback(executor, capabilitiesCallback)
    }

    fun stop() {
        session?.stop()
        session = null
        context.getSystemService(RangingManager::class.java)
            ?.unregisterCapabilitiesCallback(capabilitiesCallback)
        executor.shutdown()
    }

    private fun startUwbSession() {
        val manager = context.getSystemService(RangingManager::class.java) ?: return
        val callback = RangingCallback(peerDeviceId, onSample)
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
            onFallback()
            return
        }
        session?.start(preference)
    }

    companion object {
        private const val RANGING_SDK_FLOOR = 36
        private const val SESSION_ID = 0x4B4E

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
