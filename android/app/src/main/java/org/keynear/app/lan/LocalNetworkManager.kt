package org.keynear.app.lan

import android.content.Context
import android.content.pm.PackageManager
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.os.Build
import android.util.Log
import androidx.core.content.ContextCompat
import org.keynear.app.ble.PairingNotificationManager
import org.keynear.app.core.CoreClientProvider
import org.keynear.app.core.EndpointHexCodec
import org.keynear.app.core.LocalNetworkStatus
import java.nio.charset.StandardCharsets
import java.util.concurrent.Executor
import java.util.concurrent.Executors

object LocalNetworkConstants {
    const val PERMISSION_ACCESS_LOCAL_NETWORK = "android.permission.ACCESS_LOCAL_NETWORK"
    const val SERVICE_TYPE = "_keynear._udp."
    const val ANDROID_NETWORK_BLOCKED_REASON_LNP = 1024
    const val SDK_ANDROID_17 = 37
    const val TXT_DEVICE_ID = "id"
    const val TXT_ENDPOINT_ID = "eid"
    const val TXT_CAPABILITY_FLAGS = "cap"

    fun requiresLocalNetworkPermission(sdkInt: Int): Boolean = sdkInt >= SDK_ANDROID_17

    fun hasLocalNetworkPermission(context: Context, sdkInt: Int = Build.VERSION.SDK_INT): Boolean {
        if (!requiresLocalNetworkPermission(sdkInt)) {
            return true
        }
        return ContextCompat.checkSelfPermission(context, PERMISSION_ACCESS_LOCAL_NETWORK) ==
            PackageManager.PERMISSION_GRANTED
    }

    fun isLnpBlockedReason(blockedReasons: Int): Boolean =
        (blockedReasons and ANDROID_NETWORK_BLOCKED_REASON_LNP) != 0

    fun evaluateLnpStatus(
        sdkInt: Int,
        hasPermission: Boolean,
        networkBlocked: Boolean,
        blockedReasons: Int? = null,
    ): LocalNetworkStatus {
        if (requiresLocalNetworkPermission(sdkInt) && !hasPermission) {
            return LocalNetworkStatus.MISSING_PERMISSION
        }
        if (blockedReasons != null && isLnpBlockedReason(blockedReasons)) {
            return LocalNetworkStatus.BLOCKED_LNP
        }
        if (networkBlocked && requiresLocalNetworkPermission(sdkInt)) {
            return LocalNetworkStatus.BLOCKED_LNP
        }
        return LocalNetworkStatus.READY
    }

    fun buildDiscoveryRequestOrNull(
        serviceType: String = SERVICE_TYPE,
        showPicker: Boolean = true,
        sdkInt: Int = Build.VERSION.SDK_INT,
    ): Any? {
        if (sdkInt < SDK_ANDROID_17) {
            return null
        }
        return runCatching {
            val builderClass = Class.forName("android.net.nsd.NsdManager\$DiscoveryRequest\$Builder")
            val ctor = builderClass.getConstructor(String::class.java)
            val builder = ctor.newInstance(serviceType)
            if (showPicker) {
                val flagValue = runCatching {
                    NsdManager::class.java.getField("FLAG_SHOW_PICKER").getInt(null)
                }.getOrElse {
                    Class.forName("android.net.nsd.NsdManager\$DiscoveryRequest")
                        .getField("FLAG_SHOW_PICKER")
                        .getInt(null)
                }
                val setFlagsMethod = builderClass.getMethod("setFlags", Int::class.javaPrimitiveType)
                setFlagsMethod.invoke(builder, flagValue)
            }
            val buildMethod = builderClass.getMethod("build")
            buildMethod.invoke(builder)
        }.getOrNull()
    }

    fun discoverServicesWithPickerFallback(
        nsdManager: NsdManager,
        listener: NsdManager.DiscoveryListener,
        executor: Executor,
        showPicker: Boolean = true,
        sdkInt: Int = Build.VERSION.SDK_INT,
    ): Boolean {
        val request = buildDiscoveryRequestOrNull(SERVICE_TYPE, showPicker, sdkInt)
        if (request != null) {
            val reflected = runCatching {
                val requestClass = Class.forName("android.net.nsd.NsdManager\$DiscoveryRequest")
                val method = NsdManager::class.java.getMethod(
                    "discoverServices",
                    requestClass,
                    Executor::class.java,
                    NsdManager.DiscoveryListener::class.java,
                )
                method.invoke(nsdManager, request, executor, listener)
                true
            }.getOrDefault(false)
            if (reflected) {
                return true
            }
        }
        nsdManager.discoverServices(SERVICE_TYPE, NsdManager.PROTOCOL_DNS_SD, listener)
        return false
    }

    fun decodeHexEndpointId(hex: String): ByteArray? = EndpointHexCodec.decode32(hex)

    fun encodeHex(bytes: ByteArray): String = EndpointHexCodec.encode(bytes)
}

object LocalNetworkManager {
    private const val TAG = "LocalNetworkManager"
    private val nsdExecutor: Executor = Executors.newSingleThreadExecutor()

    @Volatile
    private var nsdManager: NsdManager? = null

    @Volatile
    private var registrationListener: NsdManager.RegistrationListener? = null

    @Volatile
    private var discoveryListener: NsdManager.DiscoveryListener? = null

    @Volatile
    private var networkCallback: ConnectivityManager.NetworkCallback? = null

    @Volatile
    private var started = false

    @Volatile
    private var loggedDegraded = false

    @Synchronized
    fun startOrSync(
        context: Context,
        localDeviceId: String,
        endpointIdBytes: ByteArray,
        quicPort: Int,
        showPicker: Boolean = false,
    ) {
        val appContext = context.applicationContext
        val client = CoreClientProvider.init(appContext)
        val hasPerm = LocalNetworkConstants.hasLocalNetworkPermission(appContext)
        val cm = appContext.getSystemService(ConnectivityManager::class.java)
        val mgr = appContext.getSystemService(NsdManager::class.java)

        if (mgr == null || cm == null) {
            client.updateLocalNetworkStatus(
                LocalNetworkStatus.UNAVAILABLE,
                "Local network discovery service is unavailable.",
            )
            return
        }

        ensureNetworkCallback(appContext, cm)

        if (!hasPerm) {
            client.updateLocalNetworkStatus(
                LocalNetworkStatus.MISSING_PERMISSION,
                "Local network permission is required to discover paired devices on Wi-Fi.",
            )
            PairingNotificationManager.showLnpBlockedNotification(appContext)
            if (showPicker) {
                startDiscovery(appContext, mgr, localDeviceId, showPicker = true)
            }
            return
        }

        if (client.currentLocalNetworkStatus() != LocalNetworkStatus.BLOCKED_LNP) {
            client.updateLocalNetworkStatus(LocalNetworkStatus.READY, null)
            PairingNotificationManager.cancelLnpBlockedNotification(appContext)
        }

        nsdManager = mgr
        if (quicPort > 0 && endpointIdBytes.size == 32 && registrationListener == null) {
            registerLocalService(mgr, localDeviceId, endpointIdBytes, quicPort)
        }
        if (discoveryListener == null || showPicker) {
            startDiscovery(appContext, mgr, localDeviceId, showPicker = showPicker)
        }
        started = true
    }

    @Synchronized
    fun requestPickerDiscovery(context: Context) {
        val appContext = context.applicationContext
        val client = CoreClientProvider.init(appContext)
        val ep = client.localLanEndpoint() ?: return
        val port = ep.socketAddrs.firstOrNull()?.substringAfterLast(':')?.toIntOrNull() ?: 0
        startOrSync(
            context = appContext,
            localDeviceId = ep.deviceId,
            endpointIdBytes = ep.endpointId,
            quicPort = port,
            showPicker = true,
        )
    }

    @Synchronized
    fun stop(context: Context) {
        val appContext = context.applicationContext
        val mgr = nsdManager ?: appContext.getSystemService(NsdManager::class.java)
        registrationListener?.let { listener ->
            runCatching { mgr?.unregisterService(listener) }
        }
        discoveryListener?.let { listener ->
            runCatching { mgr?.stopServiceDiscovery(listener) }
        }
        networkCallback?.let { cb ->
            val cm = appContext.getSystemService(ConnectivityManager::class.java)
            runCatching { cm?.unregisterNetworkCallback(cb) }
        }
        registrationListener = null
        discoveryListener = null
        networkCallback = null
        started = false
    }

    private fun ensureNetworkCallback(context: Context, cm: ConnectivityManager) {
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
                    PairingNotificationManager.cancelLnpBlockedNotification(context)
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
                        PairingNotificationManager.showLnpBlockedNotification(context)
                    }
                    LocalNetworkStatus.MISSING_PERMISSION -> {
                        client.updateLocalNetworkStatus(
                            LocalNetworkStatus.MISSING_PERMISSION,
                            "Local network permission is required to discover paired devices on Wi-Fi.",
                        )
                        PairingNotificationManager.showLnpBlockedNotification(context)
                    }
                    LocalNetworkStatus.READY -> {
                        client.updateLocalNetworkStatus(LocalNetworkStatus.READY, null)
                        PairingNotificationManager.cancelLnpBlockedNotification(context)
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
            Log.w(TAG, "network_callback_register_failed: ${err.message}")
        }
    }

    private fun registerLocalService(
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
                Log.i(TAG, "nsd_service_registered:${serviceInfo.serviceName}")
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

    private fun startDiscovery(
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
                Log.i(TAG, "nsd_discovery_started")
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
    private fun resolveFoundService(
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
                        Log.w(TAG, "nsd_resolve_failed:$errorCode")
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
            Log.w(TAG, "nsd_resolve_exception:${err.message}")
        }
    }

    private fun logDegradedOnce(reason: String) {
        if (!loggedDegraded) {
            loggedDegraded = true
            Log.w(TAG, "mdns_degraded:$reason")
        }
    }
}
