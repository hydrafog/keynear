package org.keynear.app.ble

import android.Manifest
import android.annotation.SuppressLint
import android.app.PendingIntent
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattDescriptor
import android.bluetooth.BluetoothGattServer
import android.bluetooth.BluetoothGattServerCallback
import android.bluetooth.BluetoothGattService
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.bluetooth.le.AdvertiseCallback
import android.bluetooth.le.AdvertiseData
import android.bluetooth.le.AdvertiseSettings
import android.bluetooth.le.BluetoothLeAdvertiser
import android.bluetooth.le.BluetoothLeScanner
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanFilter
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.ParcelUuid
import android.util.Log
import androidx.core.content.ContextCompat
import org.keynear.app.core.BluetoothStatus
import org.keynear.app.core.CoreClientProvider
import org.keynear.app.core.DeviceRoleUi
import org.keynear.app.core.DiscoveredPeerUi
import org.keynear.app.core.PendingPairingUi
import org.keynear.app.core.VisibilityState
import uniffi.keynear_core.FfiDeviceRole
import uniffi.keynear_core.FfiNoisePairingSession
import uniffi.keynear_core.FfiPairingState
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

object KeynearBleConstants {
    val SERVICE_UUID: UUID = UUID.fromString("0000FE6D-0000-1000-8000-00805F9B34FB")
    val PAIRING_TX_CHAR_UUID: UUID = UUID.fromString("0000FE6E-0000-1000-8000-00805F9B34FB")
    val PAIRING_RX_CHAR_UUID: UUID = UUID.fromString("0000FE6F-0000-1000-8000-00805F9B34FB")
    val CLIENT_CONFIG_DESCRIPTOR_UUID: UUID = UUID.fromString("00002902-0000-1000-8000-00805F9B34FB")

    const val MAX_BEACON_DATA_BYTES = 24
    const val MAX_GATT_PAIRING_FRAME_BYTES = 512
    const val ANDROID_CAPABILITY_FLAGS = 0x03u

    fun requiredPermissions(sdkInt: Int): List<String> = buildList {
        add(Manifest.permission.BLUETOOTH_SCAN)
        add(Manifest.permission.BLUETOOTH_CONNECT)
        add(Manifest.permission.BLUETOOTH_ADVERTISE)
        if (sdkInt >= 33) {
            add(Manifest.permission.POST_NOTIFICATIONS)
        }
    }

    fun missingPermissions(context: Context, sdkInt: Int): List<String> =
        requiredPermissions(sdkInt).filter { perm ->
            ContextCompat.checkSelfPermission(context, perm) != PackageManager.PERMISSION_GRANTED
        }

    fun hasScanPermission(context: Context): Boolean =
        ContextCompat.checkSelfPermission(context, Manifest.permission.BLUETOOTH_SCAN) ==
            PackageManager.PERMISSION_GRANTED

    fun hasConnectPermission(context: Context): Boolean =
        ContextCompat.checkSelfPermission(context, Manifest.permission.BLUETOOTH_CONNECT) ==
            PackageManager.PERMISSION_GRANTED

    fun hasAdvertisePermission(context: Context): Boolean =
        ContextCompat.checkSelfPermission(context, Manifest.permission.BLUETOOTH_ADVERTISE) ==
            PackageManager.PERMISSION_GRANTED

    fun buildScanFilterList(): List<ScanFilter> = listOf(
        ScanFilter.Builder()
            .setServiceUuid(ParcelUuid(SERVICE_UUID))
            .build(),
    )

    fun isValidGattFrame(bytes: ByteArray): Boolean =
        bytes.isNotEmpty() && bytes.size <= MAX_GATT_PAIRING_FRAME_BYTES
}

object KeynearBleManager {
    private const val TAG = "KeynearBleManager"

    private val discoveredByAddress = ConcurrentHashMap<String, DiscoveredPeerUi>()
    private val lastPairedCounters = ConcurrentHashMap<String, UInt>()

    @Volatile
    private var advertiser: BluetoothLeAdvertiser? = null

    @Volatile
    private var advertiseCallback: AdvertiseCallback? = null

    @Volatile
    private var scanner: BluetoothLeScanner? = null

    @Volatile
    private var activeScanCallback: ScanCallback? = null

    @Volatile
    private var pendingScanIntent: PendingIntent? = null

    @Volatile
    private var gattServer: BluetoothGattServer? = null

    @Volatile
    private var activeClientGatt: BluetoothGatt? = null

    @Volatile
    private var activeSession: FfiNoisePairingSession? = null

    @Volatile
    private var activePeerAddress: String? = null

    @Volatile
    private var activeDeviceForNotification: BluetoothDevice? = null

    @Volatile
    private var rxNotifyCharacteristic: BluetoothGattCharacteristic? = null

    @Volatile
    private var pendingLocalExport: ByteArray? = null

    @Volatile
    private var lastStateUpdateListener: (() -> Unit)? = null

    fun setUiRefreshListener(listener: (() -> Unit)?) {
        lastStateUpdateListener = listener
    }

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
        ensureGattServer(context, manager)

        if (visibility == VisibilityState.HIDDEN) {
            stopAdvertising(adapter)
            stopScanning(context, adapter)
            discoveredByAddress.clear()
            client.updateNearbyPeers(emptyList())
            return
        }

        startAdvertising(context, adapter, visibility)
        startScanning(context, adapter)
    }

    @SuppressLint("MissingPermission")
    fun stopAdvertisingAndScanning(context: Context) {
        val manager = context.getSystemService(BluetoothManager::class.java) ?: return
        val adapter = manager.adapter ?: return
        stopAdvertising(adapter)
        stopScanning(context, adapter)
        if (KeynearBleConstants.hasConnectPermission(context)) {
            runCatching { activeClientGatt?.close() }
            runCatching { gattServer?.close() }
        }
        activeClientGatt = null
        gattServer = null
    }

    fun onCompanionDeviceAppeared(context: Context, address: String) {
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
            discoveredByAddress[address] = DiscoveredPeerUi(
                address = address,
                displayName = everyone.displayName,
                role = DeviceRoleUi.fromFfi(everyone.role),
                rssi = rssi,
                capabilityFlags = everyone.capabilityFlags.toLong(),
                isPaired = false,
                batteryLevel = null,
            )
            client.updateNearbyPeers(discoveredByAddress.values.sortedByDescending { it.rssi })
            lastStateUpdateListener?.invoke()
            return
        }

        val secretStore = client.keystoreStore() ?: return
        val paired = client.getPairedDevices()
        val counterHint = if (serviceData.size >= 10) {
            ((serviceData[6].toUInt() and 0xFFu) shl 24) or
                ((serviceData[7].toUInt() and 0xFFu) shl 16) or
                ((serviceData[8].toUInt() and 0xFFu) shl 8) or
                (serviceData[9].toUInt() and 0xFFu)
        } else {
            0u
        }

        for (device in paired) {
            val secrets = secretStore.getPeerSecrets(device.deviceId) ?: continue
            val verified = try {
                handle.decodePairedBeacon(
                    serviceData = serviceData,
                    deviceId = device.deviceId,
                    expectedCounter = counterHint,
                    lastAcceptedCounter = lastPairedCounters[device.deviceId],
                    reconnectKey = secrets.reconnectSecret,
                )
            } catch (_: Throwable) {
                null
            } finally {
                secrets.zeroize()
            }
            if (verified != null) {
                lastPairedCounters[device.deviceId] = verified.counter
                discoveredByAddress[address] = DiscoveredPeerUi(
                    address = address,
                    displayName = device.displayName,
                    role = DeviceRoleUi.fromFfi(device.deviceRole),
                    rssi = rssi,
                    capabilityFlags = device.capabilityFlags.toLong(),
                    isPaired = true,
                    batteryLevel = verified.batteryLevel?.toInt(),
                )
                client.updateNearbyPeers(discoveredByAddress.values.sortedByDescending { it.rssi })
                lastStateUpdateListener?.invoke()
                return
            }
        }
    }

    @SuppressLint("MissingPermission")
    fun initiatePairing(context: Context, peerAddress: String) {
        if (!KeynearBleConstants.hasConnectPermission(context)) {
            CoreClientProvider.init(context).updateBluetoothStatus(
                BluetoothStatus.MISSING_PERMISSION,
                "Bluetooth connect permission is required to pair.",
            )
            return
        }
        val manager = context.getSystemService(BluetoothManager::class.java) ?: return
        val adapter = manager.adapter ?: return
        val device = runCatching { adapter.getRemoteDevice(peerAddress) }.getOrNull() ?: return
        val client = CoreClientProvider.init(context)
        val privKey = client.getOrCreateStaticPrivateKey()
        try {
            val session = FfiNoisePairingSession.newInitiator(
                localStaticPrivate = privKey,
                localDeviceId = client.localDeviceId(),
                localDisplayName = client.localDisplayName(),
                localRole = FfiDeviceRole.MOBILE,
                capabilityFlags = KeynearBleConstants.ANDROID_CAPABILITY_FLAGS,
            )
            activeSession = session
            activePeerAddress = peerAddress
            pendingLocalExport = null
            activeClientGatt?.close()
            activeClientGatt = device.connectGatt(
                context,
                false,
                buildGattClientCallback(context.applicationContext, peerAddress),
                BluetoothDevice.TRANSPORT_LE,
            )
        } catch (t: Throwable) {
            Log.w(TAG, "initiate_pairing_failed")
            client.updateBluetoothStatus(BluetoothStatus.READY, t.message)
        } finally {
            privKey.fill(0)
        }
    }

    @SuppressLint("MissingPermission")
    fun approveActivePairing(context: Context) {
        val session = activeSession ?: return
        val client = CoreClientProvider.init(context)
        try {
            val export = session.approveAndBuildExport()
            pendingLocalExport = export
            val clientGatt = activeClientGatt
            if (clientGatt != null && KeynearBleConstants.hasConnectPermission(context)) {
                val txChar = clientGatt
                    .getService(KeynearBleConstants.SERVICE_UUID)
                    ?.getCharacteristic(KeynearBleConstants.PAIRING_TX_CHAR_UUID)
                if (txChar != null) {
                    writeCharacteristicCompat(clientGatt, txChar, export)
                }
            }
            val responderDevice = activeDeviceForNotification
            val rxChar = rxNotifyCharacteristic
            if (responderDevice != null && rxChar != null && KeynearBleConstants.hasConnectPermission(context)) {
                notifyCharacteristicCompat(gattServer, responderDevice, rxChar, export)
            }
            PairingNotificationManager.cancel(context)
        } catch (_: Throwable) {
            rejectActivePairing(context)
        }
    }

    fun rejectActivePairing(context: Context) {
        runCatching { activeSession?.reject() }
        activeSession = null
        activePeerAddress = null
        pendingLocalExport = null
        CoreClientProvider.init(context).setPendingPairing(null)
        PairingNotificationManager.cancel(context)
        lastStateUpdateListener?.invoke()
    }

    @SuppressLint("MissingPermission")
    private fun startAdvertising(
        context: Context,
        adapter: BluetoothAdapter,
        visibility: VisibilityState,
    ) {
        if (!KeynearBleConstants.hasAdvertisePermission(context)) return
        val client = CoreClientProvider.init(context)
        val handle = client.coreHandle()

        val payload: ByteArray? = when (visibility) {
            VisibilityState.HIDDEN -> null
            VisibilityState.EVERYONE_NEARBY -> {
                runCatching {
                    handle.encodeEveryoneNearbyBeacon(
                        role = FfiDeviceRole.MOBILE,
                        capabilityFlags = KeynearBleConstants.ANDROID_CAPABILITY_FLAGS,
                        displayName = client.localDisplayName(),
                    )
                }.getOrNull()
            }
            VisibilityState.PAIRED_ONLY -> {
                val store = client.keystoreStore()
                val firstPeer = client.getPairedDevices().firstOrNull()
                if (store != null && firstPeer != null) {
                    val secrets = store.getPeerSecrets(firstPeer.deviceId)
                    if (secrets != null) {
                        try {
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
                    } else {
                        null
                    }
                } else {
                    null
                }
            }
        }

        stopAdvertising(adapter)
        if (payload == null) return

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
    private fun stopAdvertising(adapter: BluetoothAdapter) {
        val cb = advertiseCallback ?: return
        val leAdv = advertiser ?: adapter.bluetoothLeAdvertiser
        runCatching { leAdv?.stopAdvertising(cb) }
        advertiseCallback = null
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
                101,
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

    @SuppressLint("MissingPermission")
    private fun ensureGattServer(context: Context, manager: BluetoothManager) {
        if (gattServer != null || !KeynearBleConstants.hasConnectPermission(context)) return
        val server = manager.openGattServer(context, buildGattServerCallback(context.applicationContext))
            ?: return
        val service = BluetoothGattService(
            KeynearBleConstants.SERVICE_UUID,
            BluetoothGattService.SERVICE_TYPE_PRIMARY,
        )
        val txChar = BluetoothGattCharacteristic(
            KeynearBleConstants.PAIRING_TX_CHAR_UUID,
            BluetoothGattCharacteristic.PROPERTY_WRITE,
            BluetoothGattCharacteristic.PERMISSION_WRITE,
        )
        val rxChar = BluetoothGattCharacteristic(
            KeynearBleConstants.PAIRING_RX_CHAR_UUID,
            BluetoothGattCharacteristic.PROPERTY_NOTIFY,
            BluetoothGattCharacteristic.PERMISSION_READ,
        )
        rxChar.addDescriptor(
            BluetoothGattDescriptor(
                KeynearBleConstants.CLIENT_CONFIG_DESCRIPTOR_UUID,
                BluetoothGattDescriptor.PERMISSION_READ or BluetoothGattDescriptor.PERMISSION_WRITE,
            ),
        )
        service.addCharacteristic(txChar)
        service.addCharacteristic(rxChar)
        rxNotifyCharacteristic = rxChar
        server.addService(service)
        gattServer = server
    }

    private fun buildGattServerCallback(context: Context): BluetoothGattServerCallback =
        object : BluetoothGattServerCallback() {
            @SuppressLint("MissingPermission")
            override fun onCharacteristicWriteRequest(
                device: BluetoothDevice,
                requestId: Int,
                characteristic: BluetoothGattCharacteristic,
                preparedWrite: Boolean,
                responseNeeded: Boolean,
                offset: Int,
                value: ByteArray,
            ) {
                if (characteristic.uuid != KeynearBleConstants.PAIRING_TX_CHAR_UUID ||
                    !KeynearBleConstants.isValidGattFrame(value)
                ) {
                    if (responseNeeded) {
                        gattServer?.sendResponse(device, requestId, BluetoothGatt.GATT_INVALID_ATTRIBUTE_LENGTH, offset, null)
                    }
                    return
                }
                if (responseNeeded) {
                    gattServer?.sendResponse(device, requestId, BluetoothGatt.GATT_SUCCESS, offset, null)
                }

                val client = CoreClientProvider.init(context)
                var session = activeSession
                if (session == null) {
                    val priv = client.getOrCreateStaticPrivateKey()
                    session = try {
                        FfiNoisePairingSession.newResponder(
                            localStaticPrivate = priv,
                            localDeviceId = client.localDeviceId(),
                            localDisplayName = client.localDisplayName(),
                            localRole = FfiDeviceRole.MOBILE,
                            capabilityFlags = KeynearBleConstants.ANDROID_CAPABILITY_FLAGS,
                        )
                    } finally {
                        priv.fill(0)
                    }
                    activeSession = session
                    activePeerAddress = device.address
                    activeDeviceForNotification = device
                }

                when (session.state()) {
                    FfiPairingState.HANDSHAKE_M1, FfiPairingState.HANDSHAKE_M3 -> {
                        val step = runCatching { session.step(value) }.getOrNull() ?: return
                        step.outboundFrame?.let { m2 ->
                            rxNotifyCharacteristic?.let { rx ->
                                notifyCharacteristicCompat(gattServer, device, rx, m2)
                            }
                        }
                        if (step.state == FfiPairingState.AWAITING_CONFIRMATION) {
                            val code = step.verificationCode ?: return
                            val peerName = discoveredByAddress[device.address]?.displayName ?: device.address
                            client.setPendingPairing(
                                PendingPairingUi(
                                    peerAddress = device.address,
                                    peerName = peerName,
                                    verificationCode = code,
                                    remainingSecs = step.remainingSecs.toLong(),
                                ),
                            )
                            PairingNotificationManager.showVerificationPrompt(context, peerName, code)
                            lastStateUpdateListener?.invoke()
                        }
                    }
                    FfiPairingState.APPROVED -> {
                        val completed = runCatching { session.completeWithPeerExport(value) }.getOrNull() ?: return
                        try {
                            client.saveCompletedPairing(
                                peerDeviceId = completed.peerDeviceId,
                                peerDisplayName = completed.peerDisplayName,
                                peerRole = completed.peerRole,
                                peerStaticPubkeyHex = completed.peerStaticPubkeyHex,
                                peerCapabilityFlags = completed.peerCapabilityFlags,
                                bleAddress = device.address,
                                reconnectSecret = completed.reconnectSecret,
                            )
                        } finally {
                            completed.reconnectSecret.fill(0)
                        }
                        activeSession = null
                        client.setPendingPairing(null)
                        lastStateUpdateListener?.invoke()
                    }
                    else -> Unit
                }
            }
        }

    private fun buildGattClientCallback(context: Context, peerAddress: String): BluetoothGattCallback =
        object : BluetoothGattCallback() {
            @SuppressLint("MissingPermission")
            override fun onConnectionStateChange(gatt: BluetoothGatt, status: Int, newState: Int) {
                if (newState == BluetoothProfile.STATE_CONNECTED && status == BluetoothGatt.GATT_SUCCESS) {
                    gatt.discoverServices()
                } else if (newState == BluetoothProfile.STATE_DISCONNECTED) {
                    gatt.close()
                }
            }

            @SuppressLint("MissingPermission")
            override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) {
                if (status != BluetoothGatt.GATT_SUCCESS) return
                val service = gatt.getService(KeynearBleConstants.SERVICE_UUID) ?: return
                val rxChar = service.getCharacteristic(KeynearBleConstants.PAIRING_RX_CHAR_UUID) ?: return
                val txChar = service.getCharacteristic(KeynearBleConstants.PAIRING_TX_CHAR_UUID) ?: return
                gatt.setCharacteristicNotification(rxChar, true)
                rxChar.getDescriptor(KeynearBleConstants.CLIENT_CONFIG_DESCRIPTOR_UUID)?.let { cccd ->
                    writeDescriptorCompat(gatt, cccd, BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE)
                }
                val session = activeSession ?: return
                val m1 = runCatching { session.startHandshake().outboundFrame }.getOrNull() ?: return
                writeCharacteristicCompat(gatt, txChar, m1)
            }

            override fun onCharacteristicChanged(
                gatt: BluetoothGatt,
                characteristic: BluetoothGattCharacteristic,
                value: ByteArray,
            ) {
                handleClientNotification(context, gatt, peerAddress, characteristic, value)
            }

            @Suppress("OVERRIDE_DEPRECATION", "DEPRECATION")
            override fun onCharacteristicChanged(
                gatt: BluetoothGatt,
                characteristic: BluetoothGattCharacteristic,
            ) {
                val value = characteristic.value ?: return
                handleClientNotification(context, gatt, peerAddress, characteristic, value)
            }
        }

    @SuppressLint("MissingPermission")
    private fun handleClientNotification(
        context: Context,
        gatt: BluetoothGatt,
        peerAddress: String,
        characteristic: BluetoothGattCharacteristic,
        value: ByteArray,
    ) {
        if (characteristic.uuid != KeynearBleConstants.PAIRING_RX_CHAR_UUID ||
            !KeynearBleConstants.isValidGattFrame(value)
        ) {
            return
        }
        val session = activeSession ?: return
        val client = CoreClientProvider.init(context)
        when (session.state()) {
            FfiPairingState.HANDSHAKE_M2 -> {
                val step = runCatching { session.step(value) }.getOrNull() ?: return
                val txChar = gatt.getService(KeynearBleConstants.SERVICE_UUID)
                    ?.getCharacteristic(KeynearBleConstants.PAIRING_TX_CHAR_UUID) ?: return
                step.outboundFrame?.let { m3 -> writeCharacteristicCompat(gatt, txChar, m3) }
                if (step.state == FfiPairingState.AWAITING_CONFIRMATION) {
                    val code = step.verificationCode ?: return
                    val peerName = discoveredByAddress[peerAddress]?.displayName ?: peerAddress
                    client.setPendingPairing(
                        PendingPairingUi(
                            peerAddress = peerAddress,
                            peerName = peerName,
                            verificationCode = code,
                            remainingSecs = step.remainingSecs.toLong(),
                        ),
                    )
                    PairingNotificationManager.showVerificationPrompt(context, peerName, code)
                    lastStateUpdateListener?.invoke()
                }
            }
            FfiPairingState.APPROVED -> {
                val completed = runCatching { session.completeWithPeerExport(value) }.getOrNull() ?: return
                try {
                    client.saveCompletedPairing(
                        peerDeviceId = completed.peerDeviceId,
                        peerDisplayName = completed.peerDisplayName,
                        peerRole = completed.peerRole,
                        peerStaticPubkeyHex = completed.peerStaticPubkeyHex,
                        peerCapabilityFlags = completed.peerCapabilityFlags,
                        bleAddress = peerAddress,
                        reconnectSecret = completed.reconnectSecret,
                    )
                } finally {
                    completed.reconnectSecret.fill(0)
                }
                activeSession = null
                client.setPendingPairing(null)
                lastStateUpdateListener?.invoke()
            }
            else -> Unit
        }
    }

    @SuppressLint("MissingPermission")
    private fun writeCharacteristicCompat(
        gatt: BluetoothGatt,
        characteristic: BluetoothGattCharacteristic,
        data: ByteArray,
    ) {
        if (android.os.Build.VERSION.SDK_INT >= 33) {
            gatt.writeCharacteristic(characteristic, data, BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT)
        } else {
            @Suppress("DEPRECATION")
            characteristic.value = data
            @Suppress("DEPRECATION")
            characteristic.writeType = BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT
            @Suppress("DEPRECATION")
            gatt.writeCharacteristic(characteristic)
        }
    }

    @SuppressLint("MissingPermission")
    private fun writeDescriptorCompat(
        gatt: BluetoothGatt,
        descriptor: BluetoothGattDescriptor,
        data: ByteArray,
    ) {
        if (android.os.Build.VERSION.SDK_INT >= 33) {
            gatt.writeDescriptor(descriptor, data)
        } else {
            @Suppress("DEPRECATION")
            descriptor.value = data
            @Suppress("DEPRECATION")
            gatt.writeDescriptor(descriptor)
        }
    }

    @SuppressLint("MissingPermission")
    private fun notifyCharacteristicCompat(
        server: BluetoothGattServer?,
        device: BluetoothDevice,
        characteristic: BluetoothGattCharacteristic,
        data: ByteArray,
    ) {
        if (server == null) return
        if (android.os.Build.VERSION.SDK_INT >= 33) {
            server.notifyCharacteristicChanged(device, characteristic, false, data)
        } else {
            @Suppress("DEPRECATION")
            characteristic.value = data
            @Suppress("DEPRECATION")
            server.notifyCharacteristicChanged(device, characteristic, false)
        }
    }
}

