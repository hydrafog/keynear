package org.keynear.app.data

import android.content.Context
import android.content.pm.PackageManager
import android.net.nsd.NsdManager
import android.os.Build
import androidx.core.content.ContextCompat
import org.keynear.app.bridge.EndpointHexCodec
import org.keynear.app.ui.model.LocalNetworkStatus
import java.util.concurrent.Executor

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
