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
import org.keynear.app.lan.LocalNetworkManager
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledExecutorService
import java.util.concurrent.TimeUnit

class KeynearForegroundService : Service() {
    @Volatile
    private var transferMonitor: ScheduledExecutorService? = null

    @Volatile
    private var actionExecutor: ExecutorService? = null

    override fun onCreate() {
        super.onCreate()
        CoreClientProvider.init(applicationContext)
        ensureNotificationChannel()
        actionExecutor = Executors.newSingleThreadExecutor()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val client = CoreClientProvider.init(applicationContext)
        ensureNotificationChannel()

        when (intent?.action) {
            ACTION_PAIRING_APPROVE -> KeynearBleManager.approveActivePairing(applicationContext)
            ACTION_PAIRING_REJECT -> KeynearBleManager.rejectActivePairing(applicationContext)
            ACTION_TRANSFER_ACCEPT -> {
                val transferId = intent.getStringExtra(EXTRA_TRANSFER_ID)
                if (!transferId.isNullOrBlank()) {
                    actionExecutor?.execute {
                        client.acceptIncomingTransferOrSync(applicationContext, transferId)
                        KeynearBleManager.notifyUiRefresh()
                    }
                }
            }
            ACTION_TRANSFER_DECLINE -> {
                val transferId = intent.getStringExtra(EXTRA_TRANSFER_ID)
                if (!transferId.isNullOrBlank()) {
                    actionExecutor?.execute {
                        client.declineTransfer(transferId)
                        KeynearBleManager.notifyUiRefresh()
                    }
                }
            }
            ACTION_TRANSFER_CANCEL -> {
                val transferId = intent.getStringExtra(EXTRA_TRANSFER_ID)
                if (!transferId.isNullOrBlank()) {
                    actionExecutor?.execute {
                        client.cancelTransfer(transferId)
                        KeynearBleManager.notifyUiRefresh()
                    }
                }
            }
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
        val node = client.ensureOrionNode(applicationContext)
        val ep = node?.localLanEndpoint()
        if (ep != null) {
            val port = ep.socketAddrs.firstOrNull()?.substringAfterLast(':')?.toIntOrNull() ?: 0
            LocalNetworkManager.startOrSync(
                context = applicationContext,
                localDeviceId = ep.deviceId,
                endpointIdBytes = ep.endpointId,
                quicPort = port,
            )
        }
        ensureTransferMonitor()
        return START_STICKY
    }

    private fun ensureTransferMonitor() {
        if (transferMonitor != null) return
        val scheduler = Executors.newSingleThreadScheduledExecutor()
        scheduler.scheduleWithFixedDelay(
            {
                runCatching {
                    val client = CoreClientProvider.getClient()
                    if (client.syncTransferStates(applicationContext)) {
                        KeynearBleManager.notifyUiRefresh()
                    }
                }
            },
            TRANSFER_POLL_INTERVAL_MS,
            TRANSFER_POLL_INTERVAL_MS,
            TimeUnit.MILLISECONDS,
        )
        transferMonitor = scheduler
    }

    override fun onDestroy() {
        transferMonitor?.shutdownNow()
        transferMonitor = null
        actionExecutor?.shutdownNow()
        actionExecutor = null
        LocalNetworkManager.stop(applicationContext)
        CoreClientProvider.getClient().shutdownOrion()
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
        private const val TRANSFER_POLL_INTERVAL_MS = 1000L
        const val CHANNEL_ID = "keynear_continuity"
        const val NOTIFICATION_ID = 1001
        const val ACTION_SYNC_VISIBILITY = "org.keynear.app.action.SYNC_VISIBILITY"
        const val ACTION_PAIRING_APPROVE = "org.keynear.app.action.PAIRING_APPROVE"
        const val ACTION_PAIRING_REJECT = "org.keynear.app.action.PAIRING_REJECT"
        const val ACTION_TRANSFER_ACCEPT = "org.keynear.app.action.TRANSFER_ACCEPT"
        const val ACTION_TRANSFER_DECLINE = "org.keynear.app.action.TRANSFER_DECLINE"
        const val ACTION_TRANSFER_CANCEL = "org.keynear.app.action.TRANSFER_CANCEL"
        const val EXTRA_TRANSFER_ID = "extra_transfer_id"

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
