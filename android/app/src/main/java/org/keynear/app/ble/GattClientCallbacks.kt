package org.keynear.app.ble

import android.annotation.SuppressLint
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattDescriptor
import android.bluetooth.BluetoothProfile
import android.content.Context
import android.util.Log
import org.keynear.app.bridge.CoreClientProvider
import org.keynear.app.ui.model.PendingPairingUi
import uniffi.keynear_core.FfiPairingState
import org.keynear.app.bridge.EventQueue
import org.keynear.app.bridge.onEncryptedLanEndpointFromBle
import org.keynear.app.bridge.encodeLocalLanEndpointForPeer
import org.keynear.app.bridge.saveCompletedPairing
import uniffi.keynear_core.FfiNoisePairingSession

internal fun GattSession.buildGattClientCallback(context: Context, peerAddress: String): BluetoothGattCallback =
    object : BluetoothGattCallback() {
        @SuppressLint("MissingPermission")
        override fun onConnectionStateChange(gatt: BluetoothGatt, status: Int, newState: Int) {
            if (newState == BluetoothProfile.STATE_CONNECTED && status == BluetoothGatt.GATT_SUCCESS) {
                if (!gatt.requestMtu(KeynearBleConstants.MAX_GATT_PAIRING_FRAME_BYTES)) {
                    gatt.discoverServices()
                }
            } else if (newState == BluetoothProfile.STATE_DISCONNECTED) {
                gatt.close()
                clearDisconnectedClientGatt(context, gatt)
            }
        }

        @SuppressLint("MissingPermission")
        override fun onMtuChanged(gatt: BluetoothGatt, mtu: Int, status: Int) {
            gatt.discoverServices()
        }

        @SuppressLint("MissingPermission")
        override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) {
            if (status != BluetoothGatt.GATT_SUCCESS) return
            val service = gatt.getService(KeynearBleConstants.SERVICE_UUID) ?: return
            val rxChar = service.getCharacteristic(KeynearBleConstants.PAIRING_RX_CHAR_UUID) ?: return
            val txChar = service.getCharacteristic(KeynearBleConstants.PAIRING_TX_CHAR_UUID) ?: return
            gatt.setCharacteristicNotification(rxChar, true)
            val cccd = rxChar.getDescriptor(KeynearBleConstants.CLIENT_CONFIG_DESCRIPTOR_UUID)
            if (cccd != null) {
                KeynearBleGattCompat.writeDescriptorCompat(
                    gatt,
                    cccd,
                    BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE,
                )
            } else {
                sendInitialHandshakeFrame(gatt, txChar)
            }
        }

        @SuppressLint("MissingPermission")
        override fun onDescriptorWrite(
            gatt: BluetoothGatt,
            descriptor: BluetoothGattDescriptor,
            status: Int,
        ) {
            if (status != BluetoothGatt.GATT_SUCCESS) return
            if (descriptor.uuid != KeynearBleConstants.CLIENT_CONFIG_DESCRIPTOR_UUID) return
            val txChar = gatt.getService(KeynearBleConstants.SERVICE_UUID)
                ?.getCharacteristic(KeynearBleConstants.PAIRING_TX_CHAR_UUID) ?: return
            sendInitialHandshakeFrame(gatt, txChar)
        }

        @SuppressLint("MissingPermission")
        override fun onCharacteristicWrite(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic,
            status: Int,
        ) {
            if (status != BluetoothGatt.GATT_SUCCESS) return
            when (characteristic.uuid) {
                KeynearBleConstants.PAIRING_TX_CHAR_UUID -> {
                    val peerId = synchronized(pairingLock) {
                        val id = pendingLanExchangePeerId
                        pendingLanExchangePeerId = null
                        id
                    } ?: return
                    exchangeLanEndpointWithGatt(context, gatt, peerId)
                }
                KeynearBleConstants.LAN_ENDPOINT_CHAR_UUID -> {
                    runCatching { gatt.readCharacteristic(characteristic) }
                }
            }
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
internal fun GattSession.handleClientNotification(
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
    val client = CoreClientProvider.init(context)
    var shouldNotifyUi = false
    synchronized(pairingLock) {
        val session = activeSession ?: return
        when (session.state()) {
            FfiPairingState.HANDSHAKE_M2 ->
                shouldNotifyUi = handleClientHandshakeStep(context, client, gatt, session, peerAddress, value)
            FfiPairingState.AWAITING_CONFIRMATION -> pendingPeerExport = value
            FfiPairingState.APPROVED -> {
                completeGattClientPairingLocked(context, client, session, gatt, peerAddress, value)
                shouldNotifyUi = true
            }
            else -> Unit
        }
    }
    if (shouldNotifyUi) {
        BleScanner.notifyUiRefresh()
    }
}

@SuppressLint("MissingPermission")
private fun GattSession.sendInitialHandshakeFrame(
    gatt: BluetoothGatt,
    txChar: BluetoothGattCharacteristic,
) {
    val session = synchronized(pairingLock) { activeSession } ?: return
    val m1 = runCatching { session.startHandshake().outboundFrame }.getOrNull() ?: return
    KeynearBleGattCompat.writeCharacteristicCompat(gatt, txChar, m1)
}

@SuppressLint("MissingPermission")
private fun GattSession.handleClientHandshakeStep(
    context: Context,
    client: org.keynear.app.bridge.EventQueue,
    gatt: BluetoothGatt,
    session: FfiNoisePairingSession,
    peerAddress: String,
    value: ByteArray,
): Boolean {
    val step = runCatching { session.step(value) }.getOrNull() ?: return false
    val txChar = gatt.getService(KeynearBleConstants.SERVICE_UUID)
        ?.getCharacteristic(KeynearBleConstants.PAIRING_TX_CHAR_UUID) ?: return false
    step.outboundFrame?.let { m3 -> KeynearBleGattCompat.writeCharacteristicCompat(gatt, txChar, m3) }
    if (step.state == FfiPairingState.AWAITING_CONFIRMATION) {
        val code = step.verificationCode ?: return false
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
        return true
    }
    return false
}

@SuppressLint("MissingPermission")
internal fun GattSession.completeGattClientPairingLocked(
    context: Context,
    client: org.keynear.app.bridge.EventQueue,
    session: FfiNoisePairingSession,
    gatt: BluetoothGatt,
    peerAddress: String,
    value: ByteArray,
    deferLanExchange: Boolean = false,
): Boolean {
    val completed = try {
        session.completeWithPeerExport(value)
    } catch (t: Throwable) {
        Log.w(TAG, "client_pairing_complete_failed", t)
        clearActivePairingFieldsLocked()
        client.setPendingPairing(null)
        return false
    }
    val saved = try {
        client.saveCompletedPairing(
            peerDeviceId = completed.peerDeviceId,
            peerDisplayName = completed.peerDisplayName,
            peerRole = completed.peerRole,
            peerStaticPubkeyHex = completed.peerStaticPubkeyHex,
            peerCapabilityFlags = completed.peerCapabilityFlags,
            bleAddress = peerAddress,
            reconnectSecret = completed.reconnectSecret,
        )
        true
    } catch (t: Throwable) {
        Log.w(TAG, "client_pairing_save_failed", t)
        false
    } finally {
        completed.reconnectSecret.fill(0)
        clearActivePairingFieldsLocked()
        client.setPendingPairing(null)
    }
    if (!saved) return false
    if (deferLanExchange) {
        pendingLanExchangePeerId = completed.peerDeviceId
    } else {
        exchangeLanEndpointWithGatt(context, gatt, completed.peerDeviceId)
    }
    return true
}

@SuppressLint("MissingPermission")
internal fun GattSession.exchangeLanEndpointWithGatt(
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
    } else {
        runCatching { gatt.readCharacteristic(lanChar) }
    }
}
