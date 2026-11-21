package org.keynear.app.ble

import android.annotation.SuppressLint
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothManager
import android.bluetooth.le.AdvertiseCallback
import android.bluetooth.le.AdvertiseData
import android.bluetooth.le.AdvertiseSettings
import android.bluetooth.le.BluetoothLeAdvertiser
import android.content.Context
import android.os.ParcelUuid
import android.util.Log
import org.keynear.app.bridge.CoreClientProvider
import org.keynear.app.bridge.EventQueue
import org.keynear.app.ui.model.BluetoothStatus
import org.keynear.app.ui.model.VisibilityState
import uniffi.keynear_core.CoreHandle
import uniffi.keynear_core.FfiDeviceRole

object BleAdvertiser {
    private const val TAG = "BleAdvertiser"

    @Volatile
    private var advertiser: BluetoothLeAdvertiser? = null

    @Volatile
    private var advertiseCallback: AdvertiseCallback? = null

    fun evaluateStatus(context: Context): BluetoothStatus {
        val manager = context.getSystemService(BluetoothManager::class.java) ?: return BluetoothStatus.UNAVAILABLE
        val adapter = manager.adapter ?: return BluetoothStatus.UNAVAILABLE
        if (!KeynearBleConstants.hasScanPermission(context) ||
            !KeynearBleConstants.hasConnectPermission(context) ||
            !KeynearBleConstants.hasAdvertisePermission(context)
        ) {
            return BluetoothStatus.MISSING_PERMISSION
        }
        return if (adapter.isEnabled) BluetoothStatus.READY else BluetoothStatus.ADAPTER_DISABLED
    }

    @SuppressLint("MissingPermission")
    fun startAdvertising(
        context: Context,
        adapter: BluetoothAdapter,
        visibility: VisibilityState,
    ) {
        if (!KeynearBleConstants.hasAdvertisePermission(context)) return
        val client = CoreClientProvider.init(context)
        val handle = client.coreHandle()
        val payload = buildBeaconPayload(client, handle, visibility)
        stopAdvertising(adapter)
        if (payload == null) return
        startLeAdvertising(adapter, payload)
    }

    private fun buildBeaconPayload(
        client: EventQueue,
        handle: CoreHandle,
        visibility: VisibilityState,
    ): ByteArray? = when (visibility) {
        VisibilityState.HIDDEN -> null
        VisibilityState.EVERYONE_NEARBY -> runCatching {
            handle.encodeEveryoneNearbyBeacon(
                role = FfiDeviceRole.MOBILE,
                capabilityFlags = KeynearBleConstants.ANDROID_CAPABILITY_FLAGS,
                displayName = client.localDisplayName(),
            )
        }.getOrNull()
        VisibilityState.PAIRED_ONLY -> pairedBeaconPayload(client, handle)
    }

    private fun pairedBeaconPayload(client: EventQueue, handle: CoreHandle): ByteArray? {
        val store = client.keystoreStore() ?: return null
        val firstPeer = client.getPairedDevices().firstOrNull() ?: return null
        val secrets = store.getPeerSecrets(firstPeer.deviceId) ?: return null
        return try {
            val counter = store.nextPresenceCounter().toUInt()
            handle.encodePairedBeacon(
                deviceId = client.localDeviceId(),
                counter = counter,
                reconnectKey = secrets.reconnectSecret,
                available = true,
                batteryLevel = null,
            )
        } catch (_: Throwable) {
            null
        } finally {
            secrets.zeroize()
        }
    }

    @SuppressLint("MissingPermission")
    private fun startLeAdvertising(adapter: BluetoothAdapter, payload: ByteArray) {
        val leAdv = adapter.bluetoothLeAdvertiser ?: return
        val settings = AdvertiseSettings.Builder()
            .setAdvertiseMode(AdvertiseSettings.ADVERTISE_MODE_BALANCED)
            .setConnectable(true)
            .setTimeout(0)
            .setTxPowerLevel(AdvertiseSettings.ADVERTISE_TX_POWER_MEDIUM)
            .build()
        val data = AdvertiseData.Builder()
            .addServiceUuid(ParcelUuid(KeynearBleConstants.SERVICE_UUID))
            .addServiceData(ParcelUuid(KeynearBleConstants.SERVICE_UUID), payload)
            .setIncludeDeviceName(false)
            .build()

        val callback = object : AdvertiseCallback() {
            override fun onStartFailure(errorCode: Int) {
                Log.w(TAG, "le_advertise_failed:$errorCode")
            }
        }
        advertiser = leAdv
        advertiseCallback = callback
        runCatching { leAdv.startAdvertising(settings, data, callback) }
    }

    @SuppressLint("MissingPermission")
    fun stopAdvertising(adapter: BluetoothAdapter) {
        val cb = advertiseCallback ?: return
        val leAdv = advertiser ?: adapter.bluetoothLeAdvertiser
        runCatching { leAdv?.stopAdvertising(cb) }
        advertiseCallback = null
    }
}
