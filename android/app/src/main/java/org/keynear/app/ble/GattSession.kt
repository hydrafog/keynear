package org.keynear.app.ble

import android.annotation.SuppressLint
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
import android.content.Context
import android.util.Log
import org.keynear.app.bridge.CoreClientProvider
import org.keynear.app.ui.model.BluetoothStatus
import org.keynear.app.ui.model.PendingPairingUi
import uniffi.keynear_core.FfiDeviceRole
import uniffi.keynear_core.FfiNoisePairingSession
import uniffi.keynear_core.FfiPairingState

object GattSession {
    private const val TAG = "GattSession"

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

    @SuppressLint("MissingPermission")
    fun ensureGattServer(context: Context, manager: BluetoothManager) {
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
        val lanChar = BluetoothGattCharacteristic(
            KeynearBleConstants.LAN_ENDPOINT_CHAR_UUID,
            BluetoothGattCharacteristic.PROPERTY_READ or BluetoothGattCharacteristic.PROPERTY_WRITE,
            BluetoothGattCharacteristic.PERMISSION_READ or BluetoothGattCharacteristic.PERMISSION_WRITE,
        )
        service.addCharacteristic(lanChar)
        rxNotifyCharacteristic = rxChar
        server.addService(service)
        gattServer = server
    }

    @SuppressLint("MissingPermission")
    fun close(context: Context) {
        if (KeynearBleConstants.hasConnectPermission(context)) {
            runCatching { activeClientGatt?.close() }
            runCatching { gattServer?.close() }
        }
        activeClientGatt = null
        gattServer = null
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
                    KeynearBleGattCompat.writeCharacteristicCompat(clientGatt, txChar, export)
                }
            }
            val responderDevice = activeDeviceForNotification
            val rxChar = rxNotifyCharacteristic
            if (responderDevice != null && rxChar != null && KeynearBleConstants.hasConnectPermission(context)) {
                KeynearBleGattCompat.notifyCharacteristicCompat(gattServer, responderDevice, rxChar, export)
            }
            PairingNotifier.cancel(context)
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
        PairingNotifier.cancel(context)
        BleScanner.notifyUiRefresh()
    }

    private fun buildGattServerCallback(context: Context): BluetoothGattServerCallback =
        object : BluetoothGattServerCallback() {
            @SuppressLint("MissingPermission")
            override fun onCharacteristicReadRequest(
                device: BluetoothDevice,
                requestId: Int,
                offset: Int,
                characteristic: BluetoothGattCharacteristic,
            ) {
                if (characteristic.uuid != KeynearBleConstants.LAN_ENDPOINT_CHAR_UUID) {
                    gattServer?.sendResponse(device, requestId, BluetoothGatt.GATT_REQUEST_NOT_SUPPORTED, offset, null)
                    return
                }
                val client = CoreClientProvider.init(context)
                val encoded = client.encodeLocalLanEndpointForBleAddress(device.address)
                if (encoded == null) {
                    gattServer?.sendResponse(device, requestId, BluetoothGatt.GATT_READ_NOT_PERMITTED, offset, null)
                    return
                }
                if (offset < 0 || offset > encoded.size) {
                    gattServer?.sendResponse(device, requestId, BluetoothGatt.GATT_INVALID_OFFSET, offset, null)
                    return
                }
                val slice = encoded.copyOfRange(offset, encoded.size)
                gattServer?.sendResponse(device, requestId, BluetoothGatt.GATT_SUCCESS, offset, slice)
            }

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
                if (characteristic.uuid == KeynearBleConstants.LAN_ENDPOINT_CHAR_UUID) {
                    handleLanEndpointWrite(context, device, requestId, responseNeeded, offset, value)
                    return
                }
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
                handlePairingTxWrite(context, device, value)
            }
        }

    @SuppressLint("MissingPermission")
    private fun handleLanEndpointWrite(
        context: Context,
        device: BluetoothDevice,
        requestId: Int,
        responseNeeded: Boolean,
        offset: Int,
        value: ByteArray,
    ) {
        if (value.isEmpty() || value.size > KeynearBleConstants.MAX_LAN_ENDPOINT_GATT_BYTES) {
            if (responseNeeded) {
                gattServer?.sendResponse(device, requestId, BluetoothGatt.GATT_INVALID_ATTRIBUTE_LENGTH, offset, null)
            }
            return
        }
        val decoded = CoreClientProvider.init(context).onEncryptedLanEndpointFromBle(value, device.address)
        if (responseNeeded) {
            val status = if (decoded != null) BluetoothGatt.GATT_SUCCESS else BluetoothGatt.GATT_WRITE_NOT_PERMITTED
            gattServer?.sendResponse(device, requestId, status, offset, null)
        }
        if (decoded != null) {
            BleScanner.notifyUiRefresh()
        }
    }

    @SuppressLint("MissingPermission")
    private fun handlePairingTxWrite(
        context: Context,
        device: BluetoothDevice,
        value: ByteArray,
    ) {
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
                        KeynearBleGattCompat.notifyCharacteristicCompat(gattServer, device, rx, m2)
                    }
                }
                if (step.state == FfiPairingState.AWAITING_CONFIRMATION) {
                    val code = step.verificationCode ?: return
                    val peerName = BleScanner.discoveredDisplayName(device.address) ?: device.address
                    client.setPendingPairing(
                        PendingPairingUi(
                            peerAddress = device.address,
                            peerName = peerName,
                            verificationCode = code,
                            remainingSecs = step.remainingSecs.toLong(),
                        ),
                    )
                    PairingNotifier.showVerificationPrompt(context, peerName, code)
                    BleScanner.notifyUiRefresh()
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
                BleScanner.notifyUiRefresh()
            }
            else -> Unit
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
                    KeynearBleGattCompat.writeDescriptorCompat(gatt, cccd, BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE)
                }
                val session = activeSession ?: return
                val m1 = runCatching { session.startHandshake().outboundFrame }.getOrNull() ?: return
                KeynearBleGattCompat.writeCharacteristicCompat(gatt, txChar, m1)
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

            override fun onCharacteristicRead(
                gatt: BluetoothGatt,
                characteristic: BluetoothGattCharacteristic,
                value: ByteArray,
                status: Int,
            ) {
                if (status == BluetoothGatt.GATT_SUCCESS &&
                    characteristic.uuid == KeynearBleConstants.LAN_ENDPOINT_CHAR_UUID
                ) {
                    CoreClientProvider.init(context).onEncryptedLanEndpointFromBle(value, peerAddress)
                    BleScanner.notifyUiRefresh()
                }
            }

            @Suppress("OVERRIDE_DEPRECATION", "DEPRECATION")
            override fun onCharacteristicRead(
                gatt: BluetoothGatt,
                characteristic: BluetoothGattCharacteristic,
                status: Int,
            ) {
                val value = characteristic.value ?: return
                if (status == BluetoothGatt.GATT_SUCCESS &&
                    characteristic.uuid == KeynearBleConstants.LAN_ENDPOINT_CHAR_UUID
                ) {
                    CoreClientProvider.init(context).onEncryptedLanEndpointFromBle(value, peerAddress)
                    BleScanner.notifyUiRefresh()
                }
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
                step.outboundFrame?.let { m3 -> KeynearBleGattCompat.writeCharacteristicCompat(gatt, txChar, m3) }
                if (step.state == FfiPairingState.AWAITING_CONFIRMATION) {
                    val code = step.verificationCode ?: return
                    val peerName = BleScanner.discoveredDisplayName(peerAddress) ?: peerAddress
                    client.setPendingPairing(
                        PendingPairingUi(
                            peerAddress = peerAddress,
                            peerName = peerName,
                            verificationCode = code,
                            remainingSecs = step.remainingSecs.toLong(),
                        ),
                    )
                    PairingNotifier.showVerificationPrompt(context, peerName, code)
                    BleScanner.notifyUiRefresh()
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
                exchangeLanEndpointWithGatt(context, gatt, completed.peerDeviceId)
                BleScanner.notifyUiRefresh()
            }
            else -> Unit
        }
    }

    @SuppressLint("MissingPermission")
    private fun exchangeLanEndpointWithGatt(
        context: Context,
        gatt: BluetoothGatt,
        peerDeviceId: String,
    ) {
        if (!KeynearBleConstants.hasConnectPermission(context)) return
        val lanChar = gatt
            .getService(KeynearBleConstants.SERVICE_UUID)
            ?.getCharacteristic(KeynearBleConstants.LAN_ENDPOINT_CHAR_UUID) ?: return
        val client = CoreClientProvider.init(context)
        val encoded = client.encodeLocalLanEndpointForPeer(peerDeviceId)
        if (encoded != null) {
            KeynearBleGattCompat.writeCharacteristicCompat(gatt, lanChar, encoded)
        }
        runCatching { gatt.readCharacteristic(lanChar) }
    }
}

