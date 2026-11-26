package org.keynear.app.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
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
import org.keynear.app.ble.BleScanner
import org.keynear.app.ble.GattSession
import org.keynear.app.clipboard.ClipboardReadActivity
import org.keynear.app.bridge.CoreClientProvider
import org.keynear.app.bridge.EventQueue
import org.keynear.app.ui.model.CoreState
import org.keynear.app.ui.model.TransferStateUi
import org.keynear.app.ui.model.VisibilityState
import org.keynear.app.data.LanManager
import org.keynear.app.otp.SmsUserConsentManager
import org.keynear.app.share.DirectShareManager
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledExecutorService
import java.util.concurrent.TimeUnit
import org.keynear.app.bridge.syncTransferStates
import org.keynear.app.bridge.syncClipboardStates
import org.keynear.app.bridge.acceptIncomingTransferOrSync
import org.keynear.app.bridge.localLanEndpoint
import org.keynear.app.proximity.ProximitySupervisor

class KeynearForegroundService : Service() {
    @Volatile
    private var transferMonitor: ScheduledExecutorService? = null

    @Volatile
    private var actionExecutor: ExecutorService? = null

    @Volatile
    private var smsUserConsentManager: SmsUserConsentManager? = null

    @Volatile
    private var proximitySupervisor: ProximitySupervisor? = null

    override fun onCreate() {
        super.onCreate()
        CoreClientProvider.init(applicationContext)
        ensureNotificationChannel()
        actionExecutor = Executors.newSingleThreadExecutor()
        smsUserConsentManager = SmsUserConsentManager(applicationContext)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val client = CoreClientProvider.init(applicationContext)
        ensureNotificationChannel()

        dispatchAction(intent, client)

        val (visibility, remainingSecs) = resolveVisibility(client)
        val notification = buildForegroundNotification(visibility, remainingSecs)
        if (!startForegroundService(startId, notification)) {
            return START_NOT_STICKY
        }

        startBackgroundWork(client, visibility)
        return START_STICKY
    }

    private fun dispatchAction(intent: Intent?, client: EventQueue) {
        val action = intent?.action ?: return
        when (action) {
            ACTION_PAIRING_APPROVE -> GattSession.approveActivePairing(applicationContext)
            ACTION_PAIRING_REJECT -> GattSession.rejectActivePairing(applicationContext)
            ACTION_TRANSFER_ACCEPT -> withTransferId(intent) { transferId ->
                actionExecutor?.execute {
                    client.acceptIncomingTransferOrSync(applicationContext, transferId)
                    BleScanner.notifyUiRefresh()
                }
            }
            ACTION_TRANSFER_DECLINE -> withTransferId(intent) { transferId ->
                actionExecutor?.execute {
                    client.declineTransfer(transferId)
                    BleScanner.notifyUiRefresh()
                }
            }
            ACTION_TRANSFER_CANCEL -> withTransferId(intent) { transferId ->
                actionExecutor?.execute {
                    client.cancelTransfer(transferId)
                    BleScanner.notifyUiRefresh()
                }
            }
        }
    }

    private fun withTransferId(intent: Intent, action: (String) -> Unit) {
        val transferId = intent.getStringExtra(EXTRA_TRANSFER_ID)
        if (!transferId.isNullOrBlank()) {
            action(transferId)
        }
    }

    private fun resolveVisibility(client: EventQueue): Pair<VisibilityState, Long> =
        when (val state = client.loadState()) {
            is CoreState.Empty -> {
                DirectShareManager.syncShortcuts(applicationContext, state.snapshot.pairedDevices)
                state.snapshot.visibility to state.snapshot.everyoneRemainingSecs
            }
            is CoreState.Ready -> {
                DirectShareManager.syncShortcuts(applicationContext, state.snapshot.pairedDevices)
                val hasActive = state.snapshot.activeTransfers.any { it.state == TransferStateUi.ACTIVE }
                KeynearTransferJobService.syncActiveTransfers(applicationContext, hasActive)
                state.snapshot.visibility to state.snapshot.everyoneRemainingSecs
            }
            else -> VisibilityState.PAIRED_ONLY to 0L
        }

    private fun buildForegroundNotification(visibility: VisibilityState, remainingSecs: Long): Notification {
        val copyClipboardIntent = Intent(this, ClipboardReadActivity::class.java).apply {
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        val copyClipboardPendingIntent = PendingIntent.getActivity(
            this,
            CLIPBOARD_ACTION_REQUEST_CODE,
            copyClipboardIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        val statusText = visibility.formattedStatus(remainingSecs)
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(android.R.drawable.stat_sys_data_bluetooth)
            .setContentTitle(getString(R.string.notification_title))
            .setContentText(statusText)
            .setOngoing(true)
            .setSilent(true)
            .addAction(
                android.R.drawable.ic_menu_edit,
                getString(R.string.action_copy_phone_clipboard),
                copyClipboardPendingIntent,
            )
            .build()
    }

    private fun startForegroundService(startId: Int, notification: Notification): Boolean {
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
        }
        return foregroundStarted
    }

    private fun startBackgroundWork(client: EventQueue, visibility: VisibilityState) {
        BleScanner.startOrSync(applicationContext, visibility)
        val node = client.ensureOrionNode(applicationContext)
        val ep = node?.localLanEndpoint()
        if (ep != null) {
            val port = ep.socketAddrs.firstOrNull()?.substringAfterLast(':')?.toIntOrNull() ?: 0
            LanManager.startOrSync(
                context = applicationContext,
                localDeviceId = ep.deviceId,
                endpointIdBytes = ep.endpointId,
                quicPort = port,
            )
        }
        ensureTransferMonitor()
        startProximitySupervisor(client)
        smsUserConsentManager?.startIfEnabled(client.isOtpPassthroughEnabled())
    }

    private fun startProximitySupervisor(client: EventQueue) {
        if (proximitySupervisor != null) return
        val snapshot = when (val state = client.loadState()) {
            is CoreState.Empty -> state.snapshot
            is CoreState.Ready -> state.snapshot
            else -> return
        }
        val paired = snapshot.pairedDevices.firstOrNull() ?: return
        val peerDeviceId = paired.deviceId
        proximitySupervisor = ProximitySupervisor.startIfEnabled(
            context = applicationContext,
            onRangingSample = { sample ->
                runCatching {
                    val node = client.ensureOrionNode(applicationContext) ?: return@runCatching
                    node.sendRangingReport(peerDeviceId, sample)
                }.onFailure { err -> Log.w(TAG, "ranging_report_failed: ${err.message}") }
            },
            measuredRssiDbm = {
                ProximitySupervisor.measuredRssi(peerDeviceId, paired.lastBleAddress)
            },
        )
    }

    private fun ensureTransferMonitor() {
        if (transferMonitor != null) return
        val scheduler = Executors.newSingleThreadScheduledExecutor()
        scheduler.scheduleWithFixedDelay(
            {
                runCatching {
                    val client = CoreClientProvider.getClient()
                    if (client.syncTransferStates(applicationContext)) {
                        BleScanner.notifyUiRefresh()
                    }
                    client.syncClipboardStates(applicationContext)
                    smsUserConsentManager?.startIfEnabled(client.isOtpPassthroughEnabled())
                    proximitySupervisor?.refreshBleRssi()
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
        smsUserConsentManager?.shutdown()
        smsUserConsentManager = null
        proximitySupervisor?.stop()
        proximitySupervisor = null
        LanManager.stop(applicationContext)
        CoreClientProvider.getClient().shutdownOrion()
        BleScanner.stopAdvertisingAndScanning(applicationContext)
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun ensureNotificationChannel() {
        val manager = getSystemService(NotificationManager::class.java) ?: return
        val channel = NotificationChannel(
            CHANNEL_ID,
            getString(R.string.notification_channel_name),
            NotificationManager.IMPORTANCE_DEFAULT,
        ).apply {
            description = getString(R.string.notification_channel_description)
        }
        manager.createNotificationChannel(channel)
        manager.deleteNotificationChannel(LEGACY_CHANNEL_ID)
    }

    companion object {
        private const val TAG = "KeynearForegroundSvc"
        private const val TRANSFER_POLL_INTERVAL_MS = 1000L
        const val CHANNEL_ID = "keynear_continuity_v2"
        private const val LEGACY_CHANNEL_ID = "keynear_continuity"
        const val NOTIFICATION_ID = 1001
        private const val CLIPBOARD_ACTION_REQUEST_CODE = 1002
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
