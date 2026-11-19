package org.keynear.app.data

import org.keynear.app.bridge.EndpointHexCodec
import org.keynear.app.bridge.KeystoreSecretStore
import org.keynear.app.bridge.PeerSecretRecord
import uniffi.keynear_core.CoreHandle
import uniffi.keynear_core.FfiDeviceRole
import uniffi.keynear_core.FfiLanEndpoint
import uniffi.keynear_core.FfiPairedDevice
import uniffi.keynear_core.OrionNodeHandle
import java.security.SecureRandom

object PairingRepo {
    private const val QUIC_SECRET_BYTES = 32
    private const val ENDPOINT_ID_BYTES = 32
    private const val MAX_LAN_ENDPOINT_FRAME_BYTES = 65_536
    private const val MILLIS_PER_SECOND = 1000L

    fun getOrCreateStaticPrivateKey(handle: CoreHandle, secretStore: KeystoreSecretStore?): ByteArray =
        if (secretStore != null) {
            secretStore.getOrCreateStaticPrivateKey { handle.generateStaticKeypair().privateKey }
        } else {
            handle.generateStaticKeypair().privateKey
        }

    fun getOrCreateQuicSecretKey(secretStore: KeystoreSecretStore?): ByteArray {
        val randomGen = { ByteArray(QUIC_SECRET_BYTES).also { SecureRandom().nextBytes(it) } }
        return secretStore?.getOrCreateQuicSecretKey(randomGen) ?: randomGen()
    }

    fun saveCompletedPairing(
        handle: CoreHandle,
        devicesPath: String?,
        secretStore: KeystoreSecretStore?,
        peerDeviceId: String,
        peerDisplayName: String,
        peerRole: FfiDeviceRole,
        peerStaticPubkeyHex: String,
        peerCapabilityFlags: UInt,
        bleAddress: String,
        reconnectSecret: ByteArray,
    ) {
        if (secretStore != null) {
            secretStore.putPeerSecrets(
                deviceId = peerDeviceId,
                record = PeerSecretRecord(
                    irk = reconnectSecret.copyOf(),
                    reconnectSecret = reconnectSecret.copyOf(),
                ),
            )
        }
        if (devicesPath != null) {
            try {
                handle.savePairedDevice(
                    devicesTomlPath = devicesPath,
                    device = FfiPairedDevice(
                        deviceId = peerDeviceId,
                        displayName = peerDisplayName,
                        deviceRole = peerRole,
                        staticX25519Pubkey = peerStaticPubkeyHex,
                        lastBleAddress = bleAddress,
                        lastLanAddress = "",
                        lastEndpointId = "",
                        capabilityFlags = peerCapabilityFlags,
                        autoAcceptFiles = false,
                        pairedAt = (System.currentTimeMillis() / MILLIS_PER_SECOND).toString(),
                    ),
                )
            } catch (t: Throwable) {
                secretStore?.deletePeerSecrets(peerDeviceId)
                throw t
            }
        }
    }

    fun encodeLocalLanEndpointForPeer(
        handle: CoreHandle,
        orionNode: OrionNodeHandle?,
        store: KeystoreSecretStore?,
        deviceId: String,
    ): ByteArray? {
        val endpoint = orionNode?.localLanEndpoint() ?: return null
        val activeStore = store ?: return null
        val secrets = activeStore.getPeerSecrets(deviceId) ?: return null
        return try {
            handle.encodeLanEndpoint(endpoint, secrets.reconnectSecret)
        } catch (_: Throwable) {
            null
        } finally {
            secrets.zeroize()
        }
    }

    fun decodeEncryptedLanEndpointFromBle(
        handle: CoreHandle,
        store: KeystoreSecretStore,
        devices: List<FfiPairedDevice>,
        encoded: ByteArray,
        bleAddress: String?,
    ): FfiLanEndpoint? {
        if (encoded.isEmpty() || encoded.size > MAX_LAN_ENDPOINT_FRAME_BYTES) return null
        val ordered = if (!bleAddress.isNullOrBlank()) {
            val exact = devices.filter { it.lastBleAddress.equals(bleAddress, ignoreCase = true) }
            val rest = devices.filterNot { it.lastBleAddress.equals(bleAddress, ignoreCase = true) }
            exact + rest
        } else {
            devices
        }
        for (dev in ordered) {
            val secrets = store.getPeerSecrets(dev.deviceId) ?: continue
            val decoded = try {
                handle.decodeLanEndpoint(encoded, dev.deviceId, secrets.reconnectSecret)
            } catch (_: Throwable) {
                null
            } finally {
                secrets.zeroize()
            }
            if (decoded != null) return decoded
        }
        return null
    }

    fun onPeerLanEndpointDiscovered(
        handle: CoreHandle,
        devicesPath: String?,
        orionNode: OrionNodeHandle?,
        store: KeystoreSecretStore?,
        devices: List<FfiPairedDevice>,
        deviceId: String,
        endpointId: ByteArray,
        socketAddrs: List<String>,
    ) {
        if (endpointId.size != ENDPOINT_ID_BYTES) return
        val dev = devices.firstOrNull { it.deviceId == deviceId } ?: return
        val activeStore = store ?: return
        val secrets = activeStore.getPeerSecrets(deviceId) ?: return
        try {
            orionNode?.syncPeer(
                deviceId = deviceId,
                endpointId = endpointId,
                socketAddrs = socketAddrs,
                reconnectSecret = secrets.reconnectSecret,
                autoAcceptFiles = dev.autoAcceptFiles,
            )
            val eidHex = EndpointHexCodec.encode(endpointId)
            val firstAddr = socketAddrs.firstOrNull().orEmpty()
            if (devicesPath != null &&
                (dev.lastEndpointId != eidHex || (firstAddr.isNotEmpty() && dev.lastLanAddress != firstAddr))
            ) {
                runCatching {
                    handle.savePairedDevice(
                        devicesTomlPath = devicesPath,
                        device = dev.copy(
                            lastEndpointId = eidHex,
                            lastLanAddress = if (firstAddr.isNotEmpty()) firstAddr else dev.lastLanAddress,
                        ),
                    )
                }
            }
        } finally {
            secrets.zeroize()
        }
    }
}

