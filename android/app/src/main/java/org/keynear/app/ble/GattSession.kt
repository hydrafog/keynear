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
import uniffi.keynear_core.FfiPairingState

object GattSession {
    internal const val TAG = "GattSession"
    internal val pairingLock = Any()

    internal enum class ApprovalAction { NO_SESSION, ALREADY_APPROVED, APPROVE_NOW }

    internal fun decideApprovalAction(state: FfiPairingState?): ApprovalAction = when (state) {
        null -> ApprovalAction.NO_SESSION
        FfiPairingState.APPROVED -> ApprovalAction.ALREADY_APPROVED
        FfiPairingState.AWAITING_CONFIRMATION -> ApprovalAction.APPROVE_NOW
        else -> ApprovalAction.NO_SESSION
    }

    internal fun shouldClearOnDisconnect(activeHandle: Any?, disconnectedHandle: Any?): Boolean =
        activeHandle != null && activeHandle === disconnectedHandle

    internal fun shouldIgnorePairingTap(activePeerAddress: String?, requestedAddress: String): Boolean =
        activePeerAddress?.equals(requestedAddress, ignoreCase = true) == true

    @Volatile internal var gattServer: BluetoothGattServer? = null
    @Volatile internal var activeClientGatt: BluetoothGatt? = null
    @Volatile internal var activeSession: FfiNoisePairingSession? = null
    @Volatile internal var activePeerAddress: String? = null
    @Volatile internal var activeDeviceForNotification: BluetoothDevice? = null
    @Volatile internal var rxNotifyCharacteristic: BluetoothGattCharacteristic? = null
    @Volatile internal var pendingLocalExport: ByteArray? = null
    @Volatile internal var pendingPeerExport: ByteArray? = null
    @Volatile internal var pendingLanExchangePeerId: String? = null

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
        val (clientGatt, server) = synchronized(pairingLock) {
            val handles = activeClientGatt to gattServer
            activeClientGatt = null
            gattServer = null
            clearActivePairingFieldsLocked()
            pendingLanExchangePeerId = null
            handles
        }
        if (KeynearBleConstants.hasConnectPermission(context)) {
            runCatching { clientGatt?.close() }
            runCatching { server?.close() }
        }
    }

    internal fun clearActivePairingFieldsLocked() {
        activeSession = null
        activePeerAddress = null
        activeDeviceForNotification = null
        pendingLocalExport = null
        pendingPeerExport = null
    }

    internal fun clearDisconnectedClientGatt(context: Context, gatt: BluetoothGatt) {
        val cleared = synchronized(pairingLock) {
            if (!shouldClearOnDisconnect(activeClientGatt, gatt)) return
            activeClientGatt = null
            clearActivePairingFieldsLocked()
            pendingLanExchangePeerId = null
            true
        }
        if (cleared) {
            CoreClientProvider.init(context).setPendingPairing(null)
            PairingNotifier.cancel(context)
            BleScanner.notifyUiRefresh()
        }
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
        val needsReject = synchronized(pairingLock) {
            if (activeSession == null) {
                false
            } else if (shouldIgnorePairingTap(activePeerAddress, peerAddress)) {
                return false
            } else {
                true
            }
        }
        if (needsReject) rejectActivePairing(context)
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
            val previousGatt = synchronized(pairingLock) {
                val old = activeClientGatt
                activeSession = session
                activePeerAddress = peerAddress
                activeDeviceForNotification = null
                pendingLocalExport = null
                pendingPeerExport = null
                pendingLanExchangePeerId = null
                activeClientGatt = device.connectGatt(
                    context,
                    false,
                    buildGattClientCallback(context.applicationContext, peerAddress),
                    BluetoothDevice.TRANSPORT_LE,
                )
                old
            }
            runCatching { previousGatt?.close() }
            return true
        } catch (t: Throwable) {
            Log.w(TAG, "initiate_pairing_failed", t)
            client.updateBluetoothStatus(BluetoothStatus.READY, t.message)
            return false
        } finally {
            privKey.fill(0)
        }
    }

    internal fun approveSessionStep(
        state: FfiPairingState?,
        hasBufferedPeerExport: Boolean,
        approveAndBuildExport: () -> ByteArray,
        sendExport: (ByteArray) -> Unit,
        clearPendingPrompt: () -> Unit,
        completeBuffered: () -> Boolean,
        onFailure: (Throwable) -> Unit,
    ): Boolean = when (decideApprovalAction(state)) {
        ApprovalAction.NO_SESSION -> false
        ApprovalAction.ALREADY_APPROVED -> {
            clearPendingPrompt()
            if (hasBufferedPeerExport) completeBuffered() else false
        }
        ApprovalAction.APPROVE_NOW -> try {
            val export = approveAndBuildExport()
            sendExport(export)
            clearPendingPrompt()
            if (hasBufferedPeerExport) completeBuffered() else false
        } catch (t: Throwable) {
            onFailure(t)
            false
        }
    }

    @SuppressLint("MissingPermission")
    fun approveActivePairing(context: Context): Boolean {
        val client = CoreClientProvider.init(context)
        var failureToReject: Throwable? = null
        var clearedPrompt = false
        val completed = synchronized(pairingLock) {
            val session = activeSession ?: return false
            val clientGatt = activeClientGatt
            val responderDevice = activeDeviceForNotification
            val rxChar = rxNotifyCharacteristic
            approveSessionStep(
                state = runCatching { session.state() }.getOrNull(),
                hasBufferedPeerExport = pendingPeerExport != null,
                approveAndBuildExport = {
                    val export = session.approveAndBuildExport()
                    pendingLocalExport = export
                    export
                },
                sendExport = { export ->
                    if (clientGatt != null && KeynearBleConstants.hasConnectPermission(context)) {
                        val txChar = clientGatt
                            .getService(KeynearBleConstants.SERVICE_UUID)
                            ?.getCharacteristic(KeynearBleConstants.PAIRING_TX_CHAR_UUID)
                        if (txChar != null) {
                            KeynearBleGattCompat.writeCharacteristicCompat(clientGatt, txChar, export)
                        }
                    }
                    if (responderDevice != null && rxChar != null && KeynearBleConstants.hasConnectPermission(context)) {
                        KeynearBleGattCompat.notifyCharacteristicCompat(gattServer, responderDevice, rxChar, export)
                    }
                },
                clearPendingPrompt = {
                    client.setPendingPairing(null)
                    clearedPrompt = true
                },
                completeBuffered = { completeBufferedPeerExportLocked(context, client, session, clientGatt) },
                onFailure = { failureToReject = it },
            )
        }
        failureToReject?.let { err ->
            Log.w(TAG, "approve_active_pairing_failed", err)
            rejectActivePairing(context)
            return false
        }
        if (clearedPrompt || completed) {
            PairingNotifier.cancel(context)
            BleScanner.notifyUiRefresh()
        }
        return completed
    }

    @SuppressLint("MissingPermission")
    private fun completeBufferedPeerExportLocked(
        context: Context,
        client: org.keynear.app.bridge.EventQueue,
        session: FfiNoisePairingSession,
        clientGatt: BluetoothGatt?,
    ): Boolean {
        val buffered = pendingPeerExport ?: return false
        val peerAddr = activePeerAddress ?: return false
        pendingPeerExport = null
        return if (clientGatt != null) {
            completeGattClientPairingLocked(
                context,
                client,
                session,
                clientGatt,
                peerAddr,
                buffered,
                deferLanExchange = true,
            )
        } else {
            completeGattServerPairingLocked(client, session, peerAddr, buffered)
        }
    }

    fun rejectActivePairing(context: Context) {
        synchronized(pairingLock) {
            runCatching { activeSession?.reject() }
            clearActivePairingFieldsLocked()
            pendingLanExchangePeerId = null
        }
        CoreClientProvider.init(context).setPendingPairing(null)
        PairingNotifier.cancel(context)
        BleScanner.notifyUiRefresh()
    }
}
