package org.keynear.app.ui.model

import android.Manifest
import android.annotation.SuppressLint
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.os.PowerManager
import androidx.core.content.ContextCompat
import org.keynear.app.ble.KeynearBleConstants
import org.keynear.app.data.LocalNetworkConstants

data class PermissionsChecklistState(
    val nearbyGranted: Boolean,
    val notificationsGranted: Boolean,
    val batteryOptimizationIgnored: Boolean,
    val localNetworkGranted: Boolean = true,
) {
    val allRequiredGranted: Boolean
        get() = nearbyGranted && localNetworkGranted && notificationsGranted

    val allRecommendedComplete: Boolean
        get() = allRequiredGranted && batteryOptimizationIgnored

    val completedCount: Int
        get() = listOf(
            nearbyGranted,
            notificationsGranted,
            batteryOptimizationIgnored,
        ).count { it }

    val missingPermissionsCount: Int
        get() = listOf(
            nearbyGranted,
            localNetworkGranted,
            notificationsGranted,
            batteryOptimizationIgnored,
        ).count { !it }
}

@SuppressLint("NewApi")
fun evaluatePermissionsChecklist(
    context: Context,
    sdkInt: Int = Build.VERSION.SDK_INT,
): PermissionsChecklistState {
    val bleReady = KeynearBleConstants.hasScanPermission(context) &&
        KeynearBleConstants.hasConnectPermission(context) &&
        KeynearBleConstants.hasAdvertisePermission(context)
    val wifiNearbyReady = sdkInt < 33 ||
        ContextCompat.checkSelfPermission(context, Manifest.permission.NEARBY_WIFI_DEVICES) ==
        PackageManager.PERMISSION_GRANTED
    val lnpReady = LocalNetworkConstants.hasLocalNetworkPermission(context, sdkInt)
    val notificationsReady = sdkInt < 33 ||
        ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) ==
        PackageManager.PERMISSION_GRANTED
    val batteryIgnored = runCatching {
        context.getSystemService(PowerManager::class.java)
            ?.isIgnoringBatteryOptimizations(context.packageName) == true
    }.getOrDefault(false)
    return PermissionsChecklistState(
        nearbyGranted = bleReady && wifiNearbyReady,
        notificationsGranted = notificationsReady,
        batteryOptimizationIgnored = batteryIgnored,
        localNetworkGranted = lnpReady,
    )
}
