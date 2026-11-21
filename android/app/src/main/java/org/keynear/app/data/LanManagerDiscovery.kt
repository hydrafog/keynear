package org.keynear.app.data

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.os.Build
import android.util.Log
import org.keynear.app.ble.PairingNotifier
import org.keynear.app.bridge.CoreClientProvider
import org.keynear.app.bridge.EventQueue
import org.keynear.app.ui.model.LocalNetworkStatus
import java.nio.charset.StandardCharsets


internal fun LanManager.handleMissingLnpPermission(
    appContext: Context,
    client: EventQueue,
    mgr: NsdManager,
    localDeviceId: String,
    showPicker: Boolean,
) {
    client.updateLocalNetworkStatus(
        LocalNetworkStatus.MISSING_PERMISSION,
        "Local network permission is required to discover paired devices on Wi-Fi.",
    )
    PairingNotifier.showLnpBlockedNotification(appContext)
    if (showPicker) {
        startDiscovery(appContext, mgr, localDeviceId, showPicker = true)
    }
}

internal fun LanManager.ensureNetworkCallback(context: Context, cm: ConnectivityManager) {
    if (networkCallback != null) return
    val callback = object : ConnectivityManager.NetworkCallback() {
        override fun onAvailable(network: Network) {
            val client = CoreClientProvider.init(context)
            val status = LocalNetworkConstants.evaluateLnpStatus(
                sdkInt = Build.VERSION.SDK_INT,
                hasPermission = LocalNetworkConstants.hasLocalNetworkPermission(context),
                networkBlocked = false,
            )
            val msg = if (status == LocalNetworkStatus.MISSING_PERMISSION) {
                "Local network permission is required to discover paired devices on Wi-Fi."
            } else {
                null
            }
            client.updateLocalNetworkStatus(status, msg)
            if (status == LocalNetworkStatus.READY) {
                PairingNotifier.cancelLnpBlockedNotification(context)
            }
        }

        override fun onBlockedStatusChanged(network: Network, blocked: Boolean) {
            val client = CoreClientProvider.init(context)
            val status = LocalNetworkConstants.evaluateLnpStatus(
                sdkInt = Build.VERSION.SDK_INT,
                hasPermission = LocalNetworkConstants.hasLocalNetworkPermission(context),
                networkBlocked = blocked,
            )
            when (status) {
                LocalNetworkStatus.BLOCKED_LNP -> {
                    client.updateLocalNetworkStatus(
                        LocalNetworkStatus.BLOCKED_LNP,
                        "Local network access is blocked. Allow local network access in system settings.",
                    )
                    PairingNotifier.showLnpBlockedNotification(context)
                }
                LocalNetworkStatus.MISSING_PERMISSION -> {
                    client.updateLocalNetworkStatus(
                        LocalNetworkStatus.MISSING_PERMISSION,
                        "Local network permission is required to discover paired devices on Wi-Fi.",
                    )
                    PairingNotifier.showLnpBlockedNotification(context)
                }
                LocalNetworkStatus.READY -> {
                    client.updateLocalNetworkStatus(LocalNetworkStatus.READY, null)
                    PairingNotifier.cancelLnpBlockedNotification(context)
                }
                LocalNetworkStatus.UNAVAILABLE -> Unit
            }
        }
    }
    val request = NetworkRequest.Builder()
        .addTransportType(NetworkCapabilities.TRANSPORT_WIFI)
        .addTransportType(NetworkCapabilities.TRANSPORT_ETHERNET)
        .build()
    runCatching {
        cm.registerNetworkCallback(request, callback)
        networkCallback = callback
    }.onFailure { err ->
        Log.w(LanManager.TAG, "network_callback_register_failed: ${err.message}")
    }
}

internal fun LanManager.registerLocalService(
    mgr: NsdManager,
    localDeviceId: String,
    endpointIdBytes: ByteArray,
    port: Int,
) {
    val info = NsdServiceInfo().apply {
        serviceName = "keynear-$localDeviceId"
        serviceType = LocalNetworkConstants.SERVICE_TYPE
        setPort(port)
        setAttribute(LocalNetworkConstants.TXT_DEVICE_ID, localDeviceId)
        setAttribute(
            LocalNetworkConstants.TXT_ENDPOINT_ID,
            LocalNetworkConstants.encodeHex(endpointIdBytes),
        )
    }
    val listener = object : NsdManager.RegistrationListener {
        override fun onServiceRegistered(serviceInfo: NsdServiceInfo) {
            Log.i(LanManager.TAG, "nsd_service_registered:${serviceInfo.serviceName}")
        }

        override fun onRegistrationFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
            logDegradedOnce("nsd_register_failed:$errorCode")
        }

        override fun onServiceUnregistered(serviceInfo: NsdServiceInfo) = Unit

        override fun onUnregistrationFailed(serviceInfo: NsdServiceInfo, errorCode: Int) = Unit
    }
    registrationListener = listener
    runCatching {
        mgr.registerService(info, NsdManager.PROTOCOL_DNS_SD, listener)
    }.onFailure { err ->
        registrationListener = null
        logDegradedOnce("nsd_register_exception:${err.message}")
    }
}

internal fun LanManager.startDiscovery(
    context: Context,
    mgr: NsdManager,
    localDeviceId: String,
    showPicker: Boolean,
) {
    discoveryListener?.let { prev ->
        runCatching { mgr.stopServiceDiscovery(prev) }
        discoveryListener = null
    }

    val listener = object : NsdManager.DiscoveryListener {
        override fun onDiscoveryStarted(regType: String) {
            Log.i(LanManager.TAG, "nsd_discovery_started")
        }

        override fun onServiceFound(service: NsdServiceInfo) {
            if (!service.serviceType.contains("_keynear._udp")) return
            if (service.serviceName == "keynear-$localDeviceId") return
            resolveFoundService(context, mgr, service, localDeviceId)
        }

        override fun onServiceLost(service: NsdServiceInfo) = Unit

        override fun onDiscoveryStopped(serviceType: String) = Unit

        override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
            logDegradedOnce("nsd_discovery_start_failed:$errorCode")
            discoveryListener = null
        }

        override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) = Unit
    }

    discoveryListener = listener
    runCatching {
        LocalNetworkConstants.discoverServicesWithPickerFallback(
            nsdManager = mgr,
            listener = listener,
            executor = nsdExecutor,
            showPicker = showPicker,
        )
    }.onFailure { err ->
        discoveryListener = null
        logDegradedOnce("nsd_discover_exception:${err.message}")
    }
}

@Suppress("DEPRECATION")
internal fun LanManager.resolveFoundService(
    context: Context,
    mgr: NsdManager,
    service: NsdServiceInfo,
    localDeviceId: String,
) {
    runCatching {
        mgr.resolveService(
            service,
            object : NsdManager.ResolveListener {
                override fun onResolveFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
                    Log.w(LanManager.TAG, "nsd_resolve_failed:$errorCode")
                }

                override fun onServiceResolved(serviceInfo: NsdServiceInfo) {
                    val attrs = serviceInfo.attributes
                    val peerIdBytes = attrs[LocalNetworkConstants.TXT_DEVICE_ID] ?: return
                    val peerEidBytes = attrs[LocalNetworkConstants.TXT_ENDPOINT_ID] ?: return
                    val peerId = String(peerIdBytes, StandardCharsets.UTF_8).trim()
                    if (peerId.isEmpty() || peerId == localDeviceId) return
                    val eidHex = String(peerEidBytes, StandardCharsets.UTF_8).trim()
                    val eid = LocalNetworkConstants.decodeHexEndpointId(eidHex) ?: return
                    val hostAddr = serviceInfo.host?.hostAddress ?: return
                    val port = serviceInfo.port
                    if (port <= 0) return
                    val formattedAddr = if (hostAddr.contains(':')) {
                        "[$hostAddr]:$port"
                    } else {
                        "$hostAddr:$port"
                    }
                    CoreClientProvider.init(context).onPeerLanEndpointDiscovered(
                        deviceId = peerId,
                        endpointId = eid,
                        socketAddrs = listOf(formattedAddr),
                    )
                }
            },
        )
    }.onFailure { err ->
        Log.w(LanManager.TAG, "nsd_resolve_exception:${err.message}")
    }
}

internal fun LanManager.logDegradedOnce(reason: String) {
    if (!loggedDegraded) {
        loggedDegraded = true
        Log.w(LanManager.TAG, "mdns_degraded:$reason")
    }
}
