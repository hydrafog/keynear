package org.keynear.app.service

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.IBinder
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat
import androidx.core.content.ContextCompat
import org.keynear.app.R
import org.keynear.app.ble.KeynearBleConstants
import org.keynear.app.ble.KeynearBleManager
import org.keynear.app.core.CoreClientProvider
import org.keynear.app.core.CoreState
import org.keynear.app.core.VisibilityState

class KeynearForegroundService : Service() {
    override fun onCreate() {
        super.onCreate()
        CoreClientProvider.init(applicationContext)
        ensureNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val client = CoreClientProvider.init(applicationContext)
        ensureNotificationChannel()

        when (intent?.action) {
            ACTION_PAIRING_APPROVE -> KeynearBleManager.approveActivePairing(applicationContext)
            ACTION_PAIRING_REJECT -> KeynearBleManager.rejectActivePairing(applicationContext)
        }

        val (visibility, remainingSecs) = when (val state = client.loadState()) {
            is CoreState.Empty -> state.snapshot.visibility to state.snapshot.everyoneRemainingSecs
            is CoreState.Ready -> state.snapshot.visibility to state.snapshot.everyoneRemainingSecs
            else -> VisibilityState.PAIRED_ONLY to 0L
        }

        val statusText = visibility.formattedStatus(remainingSecs)
        val notification = NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(android.R.drawable.stat_sys_data_bluetooth)
            .setContentTitle(getString(R.string.notification_title))
            .setContentText(statusText)
            .setOngoing(true)
            .setSilent(true)
            .build()

        val foregroundStarted = runCatching {
            ServiceCompat.startForeground(
                this,
                NOTIFICATION_ID,
                notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE,
            )
        }.onFailure { err ->
            Log.w(TAG, "foreground_start_failed: ${err.message}")
        }.isSuccess

        if (!foregroundStarted) {
            stopSelf(startId)
            return START_NOT_STICKY
        }

        KeynearBleManager.startOrSync(applicationContext, visibility)
        return START_STICKY
    }

    override fun onDestroy() {
        KeynearBleManager.stopAdvertisingAndScanning(applicationContext)
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun ensureNotificationChannel() {
        val manager = getSystemService(NotificationManager::class.java) ?: return
        val channel = NotificationChannel(
            CHANNEL_ID,
            getString(R.string.notification_channel_name),
            NotificationManager.IMPORTANCE_LOW,
        ).apply {
            description = getString(R.string.notification_channel_description)
        }
        manager.createNotificationChannel(channel)
    }

    companion object {
        private const val TAG = "KeynearForegroundSvc"
        const val CHANNEL_ID = "keynear_continuity"
        const val NOTIFICATION_ID = 1001
        const val ACTION_SYNC_VISIBILITY = "org.keynear.app.action.SYNC_VISIBILITY"
        const val ACTION_PAIRING_APPROVE = "org.keynear.app.action.PAIRING_APPROVE"
        const val ACTION_PAIRING_REJECT = "org.keynear.app.action.PAIRING_REJECT"

        fun syncIfPermitted(context: Context) {
            if (!KeynearBleConstants.hasScanPermission(context) ||
                !KeynearBleConstants.hasConnectPermission(context) ||
                !KeynearBleConstants.hasAdvertisePermission(context)
            ) {
                return
            }
            val intent = Intent(context, KeynearForegroundService::class.java).apply {
                action = ACTION_SYNC_VISIBILITY
            }
            runCatching {
                ContextCompat.startForegroundService(context, intent)
            }.onFailure { err ->
                Log.w(TAG, "sync_service_failed: ${err.message}")
            }
        }
    }
}
