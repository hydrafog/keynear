package org.keynear.app.service

import android.app.job.JobInfo
import android.app.job.JobParameters
import android.app.job.JobScheduler
import android.app.job.JobService
import android.content.ComponentName
import android.content.Context
import android.os.Build
import android.util.Log
import org.keynear.app.ble.PairingNotificationManager
import org.keynear.app.core.CoreClientProvider
import org.keynear.app.core.CoreState
import org.keynear.app.core.TransferDirectionUi
import org.keynear.app.core.TransferStateUi
import org.keynear.app.core.TransferUi
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledExecutorService
import java.util.concurrent.TimeUnit

class KeynearTransferJobService : JobService() {
    @Volatile
    private var monitor: ScheduledExecutorService? = null

    override fun onStartJob(params: JobParameters): Boolean {
        val app = applicationContext
        val client = CoreClientProvider.init(app)
        val initialTransfer = activeTransfersFromState(client.loadState()).firstOrNull()
            ?: placeholderTransfer()

        val initialNotification = PairingNotificationManager.buildTransferProgressNotification(
            context = app,
            transfer = initialTransfer,
            sdkInt = Build.VERSION.SDK_INT,
        )
        val initialNotificationId = PairingNotificationManager.progressNotificationId(initialTransfer.transferId)

        if (Build.VERSION.SDK_INT >= 34) {
            runCatching {
                setNotification(
                    params,
                    initialNotificationId,
                    initialNotification,
                    JOB_END_NOTIFICATION_POLICY_DETACH,
                )
            }.onFailure { err ->
                Log.w(TAG, "job_set_notification_failed: ${err.message}")
            }
        }

        if (activeTransfersFromState(client.loadState()).isEmpty()) {
            return false
        }

        val executor = Executors.newSingleThreadScheduledExecutor()
        monitor = executor
        executor.scheduleWithFixedDelay(
            {
                runCatching {
                    client.syncTransferStates(app)
                    val active = activeTransfersFromState(client.loadState())
                    val current = active.firstOrNull()
                    if (current == null) {
                        stopMonitor()
                        jobFinished(params, false)
                        return@runCatching
                    }
                    if (Build.VERSION.SDK_INT >= 34) {
                        val updated = PairingNotificationManager.buildTransferProgressNotification(
                            context = app,
                            transfer = current,
                            sdkInt = Build.VERSION.SDK_INT,
                        )
                        setNotification(
                            params,
                            PairingNotificationManager.progressNotificationId(current.transferId),
                            updated,
                            JOB_END_NOTIFICATION_POLICY_DETACH,
                        )
                    } else {
                        PairingNotificationManager.showOrUpdateTransferProgress(app, current)
                    }
                }.onFailure { err ->
                    Log.w(TAG, "job_poll_failed: ${err.message}")
                }
            },
            POLL_INTERVAL_MS,
            POLL_INTERVAL_MS,
            TimeUnit.MILLISECONDS,
        )
        return true
    }

    override fun onStopJob(params: JobParameters): Boolean {
        stopMonitor()
        return false
    }

    private fun stopMonitor() {
        monitor?.shutdownNow()
        monitor = null
    }

    private fun activeTransfersFromState(state: CoreState): List<TransferUi> {
        val snapshot = when (state) {
            is CoreState.Empty -> state.snapshot
            is CoreState.Ready -> state.snapshot
            is CoreState.Error -> state.previous
            CoreState.Loading -> null
        }
        return snapshot?.activeTransfers.orEmpty().filter { it.state == TransferStateUi.ACTIVE }
    }

    private fun placeholderTransfer(): TransferUi = TransferUi(
        transferId = "keynear_job_active",
        peerDeviceId = "",
        peerDisplayName = "Nearby peer",
        summaryName = "File transfer",
        mimeType = "application/octet-stream",
        isDirectory = false,
        direction = TransferDirectionUi.SEND,
        totalBytes = 0L,
        transferredBytes = 0L,
        state = TransferStateUi.ACTIVE,
        startedAt = "",
    )

    companion object {
        private const val TAG = "KeynearTransferJobSvc"
        private const val POLL_INTERVAL_MS = 500L
        const val TRANSFER_JOB_ID = 4101

        fun syncActiveTransfers(
            context: Context,
            hasActiveTransfers: Boolean,
        ) {
            val app = context.applicationContext
            val scheduler = app.getSystemService(JobScheduler::class.java) ?: return
            if (!hasActiveTransfers) {
                runCatching { scheduler.cancel(TRANSFER_JOB_ID) }
                return
            }
            val pending = runCatching { scheduler.getPendingJob(TRANSFER_JOB_ID) }.getOrNull()
            if (pending != null) {
                return
            }
            val builder = JobInfo.Builder(
                TRANSFER_JOB_ID,
                ComponentName(app, KeynearTransferJobService::class.java),
            )
                .setRequiredNetworkType(JobInfo.NETWORK_TYPE_ANY)
            if (Build.VERSION.SDK_INT >= 34) {
                builder.setUserInitiated(true)
            }
            runCatching {
                scheduler.schedule(builder.build())
            }.onFailure { err ->
                Log.w(TAG, "job_schedule_skipped: ${err.message}")
            }
        }
    }
}

