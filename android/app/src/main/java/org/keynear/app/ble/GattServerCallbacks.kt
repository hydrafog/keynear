package org.keynear.app.ble

import android.annotation.SuppressLint
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattServerCallback
import android.content.Context
import org.keynear.app.bridge.CoreClientProvider
import org.keynear.app.ui.model.PendingPairingUi
import uniffi.keynear_core.FfiDeviceRole
import uniffi.keynear_core.FfiNoisePairingSession
import uniffi.keynear_core.FfiPairingState
import org.keynear.app.bridge.EventQueue
import org.keynear.app.bridge.encodeLocalLanEndpointForBleAddress
import org.keynear.app.bridge.saveCompletedPairing
import org.keynear.app.bridge.onEncryptedLanEndpointFromBle

internal fun GattSession.buildGattServerCallback(context: Context): BluetoothGattServerCallback =
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


private fun GattSession.handleServerHandshakeStep(
    context: Context,
    client: org.keynear.app.bridge.EventQueue,
    session: FfiNoisePairingSession,
    device: BluetoothDevice,
    value: ByteArray,
) {
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

private fun GattSession.completeGattServerPairing(
    client: org.keynear.app.bridge.EventQueue,
    session: FfiNoisePairingSession,
    bleAddress: String,
    value: ByteArray,
) {
    val completed = runCatching { session.completeWithPeerExport(value) }.getOrNull() ?: return
    try {
        client.saveCompletedPairing(
            peerDeviceId = completed.peerDeviceId,
            peerDisplayName = completed.peerDisplayName,
            peerRole = completed.peerRole,
            peerStaticPubkeyHex = completed.peerStaticPubkeyHex,
            peerCapabilityFlags = completed.peerCapabilityFlags,
            bleAddress = bleAddress,
            reconnectSecret = completed.reconnectSecret,
        )
    } finally {
        completed.reconnectSecret.fill(0)
    }
    activeSession = null
    client.setPendingPairing(null)
    BleScanner.notifyUiRefresh()
}

@SuppressLint("MissingPermission")
internal fun GattSession.handleLanEndpointWrite(
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
internal fun GattSession.handlePairingTxWrite(
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
        FfiPairingState.HANDSHAKE_M1, FfiPairingState.HANDSHAKE_M3 ->
            handleServerHandshakeStep(context, client, session, device, value)
        FfiPairingState.APPROVED -> completeGattServerPairing(client, session, device.address, value)
        else -> Unit
    }
}
