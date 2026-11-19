package org.keynear.app.ui.receive

import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.VisibilityState
import org.keynear.app.ui.navigation.OrbitPinUi

object ReceiveStateHolder {
    fun buildMapPins(snapshot: CoreSnapshot): List<OrbitPinUi> {
        val result = mutableListOf<OrbitPinUi>()
        snapshot.pairedDevices.forEach { dev ->
            result.add(
                OrbitPinUi(
                    id = dev.deviceId,
                    displayName = dev.displayName,
                    statusLabel = if (dev.isOnline) "Wi-Fi" else "Standby",
                    role = dev.role,
                    rssi = dev.rssi,
                    isPaired = true,
                    isOnline = dev.isOnline,
                    peerAddress = dev.lastBleAddress,
                ),
            )
        }
        if (snapshot.visibility != VisibilityState.HIDDEN) {
            snapshot.nearbyPeers.filter { !it.isPaired }.forEach { peer ->
                result.add(
                    OrbitPinUi(
                        id = peer.address,
                        displayName = peer.displayName,
                        statusLabel = "Tap to pair",
                        role = peer.role,
                        rssi = peer.rssi,
                        isPaired = false,
                        isOnline = true,
                        peerAddress = peer.address,
                    ),
                )
            }
        }
        return result
    }
}

