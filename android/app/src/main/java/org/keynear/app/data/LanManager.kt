package org.keynear.app.data

import android.content.Context
import android.net.ConnectivityManager
import android.net.nsd.NsdManager
import org.keynear.app.ble.PairingNotifier
import org.keynear.app.bridge.CoreClientProvider
import org.keynear.app.ui.model.LocalNetworkStatus
import java.util.concurrent.Executor
import java.util.concurrent.Executors

object LanManager {
    internal const val TAG = "LanManager"
    internal val nsdExecutor: Executor = Executors.newSingleThreadExecutor()

    @Volatile
    internal var nsdManager: NsdManager? = null

    @Volatile
    internal var registrationListener: NsdManager.RegistrationListener? = null

    @Volatile
    internal var discoveryListener: NsdManager.DiscoveryListener? = null

    @Volatile
    internal var networkCallback: ConnectivityManager.NetworkCallback? = null

    @Volatile
    internal var started = false

    @Volatile
    internal var loggedDegraded = false

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
            handleMissingLnpPermission(appContext, client, mgr, localDeviceId, showPicker)
            return
        }

        if (client.currentLocalNetworkStatus() != LocalNetworkStatus.BLOCKED_LNP) {
            client.updateLocalNetworkStatus(LocalNetworkStatus.READY, null)
            PairingNotifier.cancelLnpBlockedNotification(appContext)
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
}
