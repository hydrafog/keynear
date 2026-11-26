package org.keynear.app.ble

import android.annotation.SuppressLint
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattDescriptor
import android.bluetooth.BluetoothGattServer
import android.bluetooth.BluetoothGattService
import android.bluetooth.BluetoothManager
import android.content.Context
import android.util.Log
import org.keynear.app.bridge.CoreClientProvider
import org.keynear.app.ui.model.BluetoothStatus
import uniffi.keynear_core.FfiDeviceRole
import uniffi.keynear_core.FfiNoisePairingSession

object GattSession {
    internal const val TAG = "GattSession"

    internal fun shouldIgnorePairingTap(activePeerAddress: String?, requestedAddress: String): Boolean =
        activePeerAddress?.equals(requestedAddress, ignoreCase = true) == true

    @Volatile
    internal var gattServer: BluetoothGattServer? = null

    @Volatile
    internal var activeClientGatt: BluetoothGatt? = null

    @Volatile
    internal var activeSession: FfiNoisePairingSession? = null

    @Volatile
    internal var activePeerAddress: String? = null

    @Volatile
    internal var activeDeviceForNotification: BluetoothDevice? = null

    @Volatile
    internal var rxNotifyCharacteristic: BluetoothGattCharacteristic? = null

    @Volatile
    internal var pendingLocalExport: ByteArray? = null

    @Volatile
    internal var pendingPeerExport: ByteArray? = null

    @Volatile
    internal var pendingLanExchangePeerId: String? = null

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
    fun initiatePairing(context: Context, peerAddress: String): Boolean {
        if (!KeynearBleConstants.hasConnectPermission(context)) {
            CoreClientProvider.init(context).updateBluetoothStatus(
                BluetoothStatus.MISSING_PERMISSION,
                "Bluetooth connect permission is required to pair.",
            )
            return false
        }
        val manager = context.getSystemService(BluetoothManager::class.java) ?: return false
        val adapter = manager.adapter ?: return false
        val device = runCatching { adapter.getRemoteDevice(peerAddress) }.getOrNull() ?: return false
        if (activeSession != null) {
            if (shouldIgnorePairingTap(activePeerAddress, peerAddress)) return false
            rejectActivePairing(context)
        }
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
            pendingPeerExport = null
            pendingLanExchangePeerId = null
            activeClientGatt?.close()
            activeClientGatt = device.connectGatt(
                context,
                false,
                buildGattClientCallback(context.applicationContext, peerAddress),
                BluetoothDevice.TRANSPORT_LE,
            )
            return true
        } catch (t: Throwable) {
            Log.w(TAG, "initiate_pairing_failed")
            client.updateBluetoothStatus(BluetoothStatus.READY, t.message)
            return false
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
            completeBufferedPeerExport(context, client, session, clientGatt)
        } catch (_: Throwable) {
            rejectActivePairing(context)
        }
    }

    @SuppressLint("MissingPermission")
    private fun completeBufferedPeerExport(
        context: Context,
        client: org.keynear.app.bridge.EventQueue,
        session: FfiNoisePairingSession,
        clientGatt: BluetoothGatt?,
    ) {
        val buffered = pendingPeerExport ?: return
        val peerAddr = activePeerAddress ?: return
        if (clientGatt == null) return
        pendingPeerExport = null
        completeGattClientPairing(
            context,
            client,
            session,
            clientGatt,
            peerAddr,
            buffered,
            deferLanExchange = true,
        )
    }

    fun rejectActivePairing(context: Context) {
        runCatching { activeSession?.reject() }
        activeSession = null
        activePeerAddress = null
        pendingLocalExport = null
        pendingPeerExport = null
        pendingLanExchangePeerId = null
        CoreClientProvider.init(context).setPendingPairing(null)
        PairingNotifier.cancel(context)
        BleScanner.notifyUiRefresh()
    }
}
