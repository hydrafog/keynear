package org.keynear.app.ble

import android.annotation.SuppressLint
import android.app.PendingIntent
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothManager
import android.bluetooth.le.BluetoothLeScanner
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.content.Context
import android.content.Intent
import android.os.ParcelUuid
import android.util.Log
import org.keynear.app.bridge.CoreClientProvider
import org.keynear.app.bridge.EventQueue
import org.keynear.app.bridge.KeystoreSecretStore
import org.keynear.app.ui.model.BluetoothStatus
import org.keynear.app.ui.model.DeviceRoleUi
import org.keynear.app.ui.model.DiscoveredPeerUi
import org.keynear.app.ui.model.VisibilityState
import uniffi.keynear_core.CoreHandle
import uniffi.keynear_core.FfiPairedBeaconState
import uniffi.keynear_core.FfiPairedDevice
import java.util.concurrent.ConcurrentHashMap
import org.keynear.app.bridge.getPairedDevices

object BleScanner {
    private const val TAG = "BleScanner"
    private const val MIN_COUNTER_BYTES = 10
    private const val BYTE_MASK = 0xFFu
    private const val PENDING_SCAN_REQUEST_CODE = 101

    private val discoveredByAddress = ConcurrentHashMap<String, DiscoveredPeerUi>()
    private val lastPairedCounters = ConcurrentHashMap<String, UInt>()

    @Volatile
    private var scanner: BluetoothLeScanner? = null

    @Volatile
    private var activeScanCallback: ScanCallback? = null

    @Volatile
    private var pendingScanIntent: PendingIntent? = null

    @Volatile
    private var lastStateUpdateListener: (() -> Unit)? = null

    fun setUiRefreshListener(listener: (() -> Unit)?) {
        lastStateUpdateListener = listener
    }

    fun notifyUiRefresh() {
        lastStateUpdateListener?.invoke()
    }

    fun discoveredDisplayName(address: String): String? =
        discoveredByAddress[address]?.displayName

    fun evaluateStatus(context: Context): BluetoothStatus =
        BleAdvertiser.evaluateStatus(context)

    @SuppressLint("MissingPermission")
    fun startOrSync(context: Context, visibility: VisibilityState) {
        val client = CoreClientProvider.init(context)
        val status = evaluateStatus(context)
        val warning = when (status) {
            BluetoothStatus.READY -> null
            BluetoothStatus.MISSING_PERMISSION -> "Bluetooth permissions required for nearby discovery."
            BluetoothStatus.ADAPTER_DISABLED -> "Turn on Bluetooth to discover nearby devices."
            BluetoothStatus.UNAVAILABLE -> "Bluetooth Low Energy hardware is unavailable."
        }
        client.updateBluetoothStatus(status, warning)
        if (status != BluetoothStatus.READY) {
            stopAdvertisingAndScanning(context)
            return
        }

        val manager = context.getSystemService(BluetoothManager::class.java) ?: return
        val adapter = manager.adapter ?: return
        GattSession.ensureGattServer(context, manager)

        if (visibility == VisibilityState.HIDDEN) {
            BleAdvertiser.stopAdvertising(adapter)
            stopScanning(context, adapter)
            discoveredByAddress.clear()
            client.updateNearbyPeers(emptyList())
            return
        }

        BleAdvertiser.startAdvertising(context, adapter, visibility)
        startScanning(context, adapter)
    }

    @SuppressLint("MissingPermission")
    fun stopAdvertisingAndScanning(context: Context) {
        val manager = context.getSystemService(BluetoothManager::class.java) ?: return
        val adapter = manager.adapter ?: return
        BleAdvertiser.stopAdvertising(adapter)
        stopScanning(context, adapter)
        GattSession.close(context)
    }

    fun onCompanionDeviceAppeared(context: Context) {
        Log.i(TAG, "companion_presence_awake")
        val client = CoreClientProvider.init(context)
        val mode = VisibilityState.fromFfi(client.coreHandle().visibilityMode())
        startOrSync(context, mode)
        lastStateUpdateListener?.invoke()
    }

    fun onCompanionDeviceDisappeared(address: String) {
        discoveredByAddress.remove(address)
        CoreClientProvider.getClient().updateNearbyPeers(discoveredByAddress.values.toList())
        lastStateUpdateListener?.invoke()
    }

    fun onScanPayload(context: Context, address: String, rssi: Int, serviceData: ByteArray) {
        if (serviceData.isEmpty() || serviceData.size > KeynearBleConstants.MAX_BEACON_DATA_BYTES) {
            return
        }
        val client = CoreClientProvider.init(context)
        val handle = client.coreHandle()

        val everyone = runCatching { handle.decodeEveryoneNearbyBeacon(serviceData) }.getOrNull()
        if (everyone != null) {
            publishPeer(
                client,
                DiscoveredPeerUi(
                    address = address,
                    displayName = everyone.displayName,
                    role = DeviceRoleUi.fromFfi(everyone.role),
                    rssi = rssi,
                    capabilityFlags = everyone.capabilityFlags.toLong(),
                    isPaired = false,
                    batteryLevel = null,
                ),
            )
            return
        }

        val secretStore = client.keystoreStore() ?: return
        for (device in client.getPairedDevices()) {
            val verified = verifyPairedDevice(handle, secretStore, device, serviceData) ?: continue
            lastPairedCounters[device.deviceId] = verified.counter
            publishPeer(
                client,
                DiscoveredPeerUi(
                    address = address,
                    displayName = device.displayName,
                    role = DeviceRoleUi.fromFfi(device.deviceRole),
                    rssi = rssi,
                    capabilityFlags = device.capabilityFlags.toLong(),
                    isPaired = true,
                    batteryLevel = verified.batteryLevel?.toInt(),
                ),
            )
            return
        }
    }

    private fun publishPeer(client: EventQueue, peer: DiscoveredPeerUi) {
        discoveredByAddress[peer.address] = peer
        client.updateNearbyPeers(discoveredByAddress.values.sortedByDescending { it.rssi })
        lastStateUpdateListener?.invoke()
    }

    private fun verifyPairedDevice(
        handle: CoreHandle,
        secretStore: KeystoreSecretStore,
        device: FfiPairedDevice,
        serviceData: ByteArray,
    ): FfiPairedBeaconState? {
        val secrets = secretStore.getPeerSecrets(device.deviceId) ?: return null
        val hint = counterHint(serviceData)
        return try {
            handle.decodePairedBeacon(
                serviceData = serviceData,
                deviceId = device.deviceId,
                expectedCounter = hint,
                lastAcceptedCounter = lastPairedCounters[device.deviceId],
                reconnectKey = secrets.reconnectSecret,
            )
        } catch (_: Throwable) {
            null
        } finally {
            secrets.zeroize()
        }
    }

    private fun counterHint(serviceData: ByteArray): UInt =
        if (serviceData.size >= MIN_COUNTER_BYTES) {
            ((serviceData[6].toUInt() and BYTE_MASK) shl 24) or
                ((serviceData[7].toUInt() and BYTE_MASK) shl 16) or
                ((serviceData[8].toUInt() and BYTE_MASK) shl 8) or
                (serviceData[9].toUInt() and BYTE_MASK)
        } else {
            0u
        }

    @SuppressLint("MissingPermission")
    private fun startScanning(context: Context, adapter: BluetoothAdapter) {
        if (!KeynearBleConstants.hasScanPermission(context)) return
        val leScanner = adapter.bluetoothLeScanner ?: return
        val filters = KeynearBleConstants.buildScanFilterList()
        val settings = ScanSettings.Builder()
            .setScanMode(ScanSettings.SCAN_MODE_LOW_POWER)
            .build()

        if (activeScanCallback == null) {
            val cb = object : ScanCallback() {
                override fun onScanResult(callbackType: Int, result: ScanResult) {
                    val data = result.scanRecord?.getServiceData(ParcelUuid(KeynearBleConstants.SERVICE_UUID))
                        ?: return
                    val addr = result.device?.address ?: return
                    onScanPayload(context.applicationContext, addr, result.rssi, data)
                }
            }
            activeScanCallback = cb
            scanner = leScanner
            runCatching { leScanner.startScan(filters, settings, cb) }
        }

        if (pendingScanIntent == null) {
            val intent = Intent(context, BleScanReceiver::class.java)
            val pi = PendingIntent.getBroadcast(
                context,
                PENDING_SCAN_REQUEST_CODE,
                intent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_MUTABLE,
            )
            pendingScanIntent = pi
            runCatching { leScanner.startScan(filters, settings, pi) }
        }
    }

    @SuppressLint("MissingPermission")
    private fun stopScanning(context: Context, adapter: BluetoothAdapter) {
        val leScanner = scanner ?: adapter.bluetoothLeScanner
        if (KeynearBleConstants.hasScanPermission(context)) {
            activeScanCallback?.let { cb -> runCatching { leScanner?.stopScan(cb) } }
            pendingScanIntent?.let { pi -> runCatching { leScanner?.stopScan(pi) } }
        }
        activeScanCallback = null
        pendingScanIntent = null
    }
}

