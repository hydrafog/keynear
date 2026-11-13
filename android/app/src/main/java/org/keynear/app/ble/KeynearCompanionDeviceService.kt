package org.keynear.app.ble

import android.companion.AssociationInfo
import android.companion.CompanionDeviceManager
import android.companion.CompanionDeviceService
import android.companion.ObservingDevicePresenceRequest
import android.os.Build
import android.util.Log
import androidx.annotation.RequiresApi

class KeynearCompanionDeviceService : CompanionDeviceService() {
    @Suppress("OVERRIDE_DEPRECATION")
    override fun onDeviceAppeared(associationInfo: AssociationInfo) {
        val address = associationInfo.deviceMacAddress?.toString()
        Log.i(TAG, "companion_device_appeared_v33")
        if (!address.isNullOrBlank()) {
            KeynearBleManager.onCompanionDeviceAppeared(applicationContext, address)
        }
    }

    @Suppress("OVERRIDE_DEPRECATION")
    override fun onDeviceDisappeared(associationInfo: AssociationInfo) {
        val address = associationInfo.deviceMacAddress?.toString()
        Log.i(TAG, "companion_device_disappeared_v33")
        if (!address.isNullOrBlank()) {
            KeynearBleManager.onCompanionDeviceDisappeared(address)
        }
    }

    @Suppress("OVERRIDE_DEPRECATION")
    override fun onDeviceAppeared(address: String) {
        Log.i(TAG, "companion_device_appeared_legacy")
        KeynearBleManager.onCompanionDeviceAppeared(applicationContext, address)
    }

    @Suppress("OVERRIDE_DEPRECATION")
    override fun onDeviceDisappeared(address: String) {
        Log.i(TAG, "companion_device_disappeared_legacy")
        KeynearBleManager.onCompanionDeviceDisappeared(address)
    }

    companion object {
        private const val TAG = "KeynearCdmService"
    }
}

object CompanionPresenceHelper {
    fun startObserving(
        manager: CompanionDeviceManager,
        associationId: Int,
        address: String,
    ) {
        if (Build.VERSION.SDK_INT >= 36) {
            startObservingApi36(manager, associationId)
        } else {
            @Suppress("DEPRECATION")
            manager.startObservingDevicePresence(address)
        }
    }

    @RequiresApi(36)
    private fun startObservingApi36(
        manager: CompanionDeviceManager,
        associationId: Int,
    ) {
        val request = ObservingDevicePresenceRequest.Builder()
            .setAssociationId(associationId)
            .build()
        manager.startObservingDevicePresence(request)
    }
}

