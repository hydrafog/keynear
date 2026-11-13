package org.keynear.app.ble

import android.bluetooth.le.BluetoothLeScanner
import android.bluetooth.le.ScanResult
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.ParcelUuid
import android.util.Log
import androidx.core.content.IntentCompat

class BleScanReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (!KeynearBleConstants.hasScanPermission(context)) {
            Log.w(TAG, "background_ble_scan_missing_permission")
            return
        }

        val results = IntentCompat.getParcelableArrayListExtra(
            intent,
            BluetoothLeScanner.EXTRA_LIST_SCAN_RESULT,
            ScanResult::class.java,
        ) ?: return

        val serviceUuid = ParcelUuid(KeynearBleConstants.SERVICE_UUID)
        for (result in results) {
            val payload = result.scanRecord?.getServiceData(serviceUuid) ?: continue
            if (payload.isEmpty() || payload.size > KeynearBleConstants.MAX_BEACON_DATA_BYTES) {
                continue
            }
            val address = result.device?.address ?: continue
            KeynearBleManager.onScanPayload(context.applicationContext, address, result.rssi, payload)
        }
    }

    companion object {
        private const val TAG = "BleScanReceiver"
    }
}

