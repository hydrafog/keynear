package org.keynear.app.ble

import android.Manifest
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.drawable.Icon
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.annotation.RequiresApi
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import org.keynear.app.MainActivity
import org.keynear.app.R
import org.keynear.app.core.TransferDirectionUi
import org.keynear.app.core.TransferProgressModel
import org.keynear.app.core.TransferProgressStage
import org.keynear.app.core.TransferUi
import org.keynear.app.core.computeTransferProgressModel
import org.keynear.app.core.formatByteCount
import org.keynear.app.service.KeynearForegroundService
import org.keynear.app.ui.theme.KeynearTokenHex

object PairingNotificationManager {
    const val CHANNEL_ID = "keynear_pairing"
    const val TRANSFER_CHANNEL_ID = "keynear_transfers"
    const val TRANSFER_PROGRESS_CHANNEL_ID = "keynear_transfer_progress"
    const val NOTIFICATION_ID = 1002
    const val TRANSFER_COMPLETE_ID = 1003
    const val TRANSFER_ERROR_ID = 1004
    const val LNP_BLOCKED_ID = 1005
    const val TRANSFER_CONSENT_BASE_ID = 2000
    const val TRANSFER_PROGRESS_BASE_ID = 3000
    const val PAIRING_TIMEOUT_MS = 120_000L

    fun formatCode(rawSixDigits: String): String {
        val digits = rawSixDigits.filter { it.isDigit() }
        if (digits.length != 6) return rawSixDigits
        return "${digits.substring(0, 3)} ${digits.substring(3, 6)}"
    }

    private fun hasPostNotificationsPermission(context: Context): Boolean =
        Build.VERSION.SDK_INT < 33 ||
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED

    fun ensureChannel(context: Context) {
        val manager = context.getSystemService(NotificationManager::class.java) ?: return
        val channel = NotificationChannel(
            CHANNEL_ID,
            context.getString(R.string.pairing_channel_name),
            NotificationManager.IMPORTANCE_HIGH,
        ).apply {
            description = context.getString(R.string.pairing_channel_description)
        }
        manager.createNotificationChannel(channel)
    }

    fun ensureTransferChannel(context: Context) {
        val manager = context.getSystemService(NotificationManager::class.java) ?: return
        val channel = NotificationChannel(
            TRANSFER_CHANNEL_ID,
            context.getString(R.string.transfer_channel_name),
            NotificationManager.IMPORTANCE_HIGH,
        ).apply {
            description = context.getString(R.string.transfer_channel_description)
        }
        manager.createNotificationChannel(channel)
    }

    fun ensureTransferProgressChannel(context: Context) {
        val manager = context.getSystemService(NotificationManager::class.java) ?: return
        val channel = NotificationChannel(
            TRANSFER_PROGRESS_CHANNEL_ID,
            context.getString(R.string.transfer_progress_channel_name),
            NotificationManager.IMPORTANCE_LOW,
        ).apply {
            description = context.getString(R.string.transfer_progress_channel_description)
            setSound(null, null)
            enableVibration(false)
        }
        manager.createNotificationChannel(channel)
    }

    fun transferNotificationKey(transferId: String): Int = transferId.hashCode() and 0x0FFF

    fun progressNotificationId(transferId: String): Int =
        TRANSFER_PROGRESS_BASE_ID + transferNotificationKey(transferId)

    fun buildTransferProgressNotification(
        context: Context,
        transfer: TransferUi,
        sdkInt: Int = Build.VERSION.SDK_INT,
    ): Notification {
        ensureTransferProgressChannel(context)
        val model = computeTransferProgressModel(
            state = transfer.state,
            transferredBytes = transfer.transferredBytes,
            totalBytes = transfer.totalBytes,
            sdkInt = sdkInt,
        )
        val key = transferNotificationKey(transfer.transferId)
        val openPending = PendingIntent.getActivity(
            context,
            310 + key,
            Intent(context, MainActivity::class.java).apply {
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP
            },
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val cancelIntent = Intent(context, KeynearForegroundService::class.java).apply {
            action = KeynearForegroundService.ACTION_TRANSFER_CANCEL
            putExtra(KeynearForegroundService.EXTRA_TRANSFER_ID, transfer.transferId)
        }
        val cancelPending = PendingIntent.getService(
            context,
            5000 + key,
            cancelIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val stageTitle = when (model.stage) {
            TransferProgressStage.PREPARE -> context.getString(R.string.transfer_stage_prepare)
            TransferProgressStage.TRANSFER -> context.getString(R.string.transfer_stage_transfer)
            TransferProgressStage.COMPLETE -> context.getString(R.string.transfer_stage_complete)
        }
        val progressBody = if (transfer.totalBytes > 0L) {
            "${transfer.summaryName} (${formatByteCount(transfer.transferredBytes)} / ${formatByteCount(transfer.totalBytes)} · ${model.overallPercent}%)"
        } else {
            transfer.summaryName
        }
        val smallIcon = if (transfer.direction == TransferDirectionUi.SEND) {
            android.R.drawable.stat_sys_upload
        } else {
            android.R.drawable.stat_sys_download
        }

        return if (model.usesProgressStyleApi36 && Build.VERSION.SDK_INT >= 36) {
            buildApi36ProgressNotification(
                context = context,
                smallIcon = smallIcon,
                title = "$stageTitle · ${transfer.peerDisplayName}",
                body = progressBody,
                model = model,
                openPending = openPending,
                cancelPending = cancelPending,
            )
        } else {
            NotificationCompat.Builder(context, TRANSFER_PROGRESS_CHANNEL_ID)
                .setSmallIcon(smallIcon)
                .setContentTitle("$stageTitle · ${transfer.peerDisplayName}")
                .setContentText(progressBody)
                .setProgress(100, model.overallPercent, model.stage == TransferProgressStage.PREPARE)
                .setOngoing(model.stage != TransferProgressStage.COMPLETE)
                .setOnlyAlertOnce(true)
                .setSilent(true)
                .setCategory(NotificationCompat.CATEGORY_PROGRESS)
                .setContentIntent(openPending)
                .addAction(0, context.getString(R.string.action_cancel), cancelPending)
                .build()
        }
    }

    @RequiresApi(36)
    private fun buildApi36ProgressNotification(
        context: Context,
        smallIcon: Int,
        title: String,
        body: String,
        model: TransferProgressModel,
        openPending: PendingIntent,
        cancelPending: PendingIntent,
    ): Notification {
        val accentColor = (KeynearTokenHex.ACCENT_BLUE_600 and 0xFFFFFFFFL).toInt()
        val focusColor = (KeynearTokenHex.FOCUS_BLUE_500 and 0xFFFFFFFFL).toInt()
        val prepareEnd = model.segmentWeights[0]
        val transferEnd = prepareEnd + model.segmentWeights[1]
        val progressStyle = Notification.ProgressStyle()
            .setStyledByProgress(true)
            .setProgress(model.segmentedProgress)
            .addProgressSegment(Notification.ProgressStyle.Segment(model.segmentWeights[0]).setColor(accentColor))
            .addProgressSegment(Notification.ProgressStyle.Segment(model.segmentWeights[1]).setColor(focusColor))
            .addProgressSegment(Notification.ProgressStyle.Segment(model.segmentWeights[2]).setColor(accentColor))
            .addProgressPoint(Notification.ProgressStyle.Point(prepareEnd).setColor(accentColor))
            .addProgressPoint(Notification.ProgressStyle.Point(transferEnd).setColor(focusColor))
        val cancelAction = Notification.Action.Builder(
            Icon.createWithResource(context, android.R.drawable.ic_menu_close_clear_cancel),
            context.getString(R.string.action_cancel),
            cancelPending,
        ).build()

        return Notification.Builder(context, TRANSFER_PROGRESS_CHANNEL_ID)
            .setSmallIcon(smallIcon)
            .setContentTitle(title)
            .setContentText(body)
            .setOngoing(model.stage != TransferProgressStage.COMPLETE)
            .setOnlyAlertOnce(true)
            .setCategory(Notification.CATEGORY_PROGRESS)
            .setContentIntent(openPending)
            .setStyle(progressStyle)
            .addAction(cancelAction)
            .build()
    }

    fun showOrUpdateTransferProgress(
        context: Context,
        transfer: TransferUi,
    ) {
        if (!hasPostNotificationsPermission(context)) return
        val manager = context.getSystemService(NotificationManager::class.java) ?: return
        val notification = buildTransferProgressNotification(context, transfer, Build.VERSION.SDK_INT)
        manager.notify(progressNotificationId(transfer.transferId), notification)
    }

    fun showVerificationPrompt(
        context: Context,
        peerName: String,
        verificationCode: String,
    ) {
        if (!hasPostNotificationsPermission(context)) {
            return
        }
        ensureChannel(context)
        val manager = context.getSystemService(NotificationManager::class.java) ?: return

        val openIntent = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP
        }
        val openPending = PendingIntent.getActivity(
            context,
            200,
            openIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        val approveIntent = Intent(context, KeynearForegroundService::class.java).apply {
            action = KeynearForegroundService.ACTION_PAIRING_APPROVE
        }
        val approvePending = PendingIntent.getService(
            context,
            201,
            approveIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        val rejectIntent = Intent(context, KeynearForegroundService::class.java).apply {
            action = KeynearForegroundService.ACTION_PAIRING_REJECT
        }
        val rejectPending = PendingIntent.getService(
            context,
            202,
            rejectIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        val formatted = formatCode(verificationCode)
        val notification = NotificationCompat.Builder(context, CHANNEL_ID)
            .setSmallIcon(android.R.drawable.stat_sys_data_bluetooth)
            .setContentTitle(context.getString(R.string.pairing_prompt_title))
            .setContentText("$peerName • Code $formatted")
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .setCategory(NotificationCompat.CATEGORY_MESSAGE)
            .setContentIntent(openPending)
            .addAction(0, context.getString(R.string.action_confirm), approvePending)
            .addAction(0, context.getString(R.string.action_cancel), rejectPending)
            .setTimeoutAfter(PAIRING_TIMEOUT_MS)
            .setAutoCancel(true)
            .build()

        manager.notify(NOTIFICATION_ID, notification)
    }

    fun showIncomingTransferConsent(
        context: Context,
        transferId: String,
        peerName: String,
        summary: String,
        sizeLabel: String,
    ) {
        if (!hasPostNotificationsPermission(context)) return
        ensureTransferChannel(context)
        val manager = context.getSystemService(NotificationManager::class.java) ?: return

        val openPending = PendingIntent.getActivity(
            context,
            300,
            Intent(context, MainActivity::class.java).apply {
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP
            },
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        val key = transferId.hashCode() and 0x0FFF
        val acceptIntent = Intent(context, KeynearForegroundService::class.java).apply {
            action = KeynearForegroundService.ACTION_TRANSFER_ACCEPT
            putExtra(KeynearForegroundService.EXTRA_TRANSFER_ID, transferId)
        }
        val acceptPending = PendingIntent.getService(
            context,
            4000 + key * 2,
            acceptIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        val declineIntent = Intent(context, KeynearForegroundService::class.java).apply {
            action = KeynearForegroundService.ACTION_TRANSFER_DECLINE
            putExtra(KeynearForegroundService.EXTRA_TRANSFER_ID, transferId)
        }
        val declinePending = PendingIntent.getService(
            context,
            4000 + key * 2 + 1,
            declineIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        val notification = NotificationCompat.Builder(context, TRANSFER_CHANNEL_ID)
            .setSmallIcon(android.R.drawable.stat_sys_download)
            .setContentTitle(context.getString(R.string.transfer_consent_title))
            .setContentText("$peerName: $summary ($sizeLabel)")
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .setCategory(NotificationCompat.CATEGORY_MESSAGE)
            .setContentIntent(openPending)
            .addAction(0, context.getString(R.string.action_accept), acceptPending)
            .addAction(0, context.getString(R.string.action_decline), declinePending)
            .setTimeoutAfter(PAIRING_TIMEOUT_MS)
            .setAutoCancel(true)
            .build()

        manager.notify(TRANSFER_CONSENT_BASE_ID + key, notification)
    }

    fun cancelTransferNotification(context: Context, transferId: String) {
        val manager = context.getSystemService(NotificationManager::class.java) ?: return
        val key = transferNotificationKey(transferId)
        manager.cancel(TRANSFER_CONSENT_BASE_ID + key)
        manager.cancel(TRANSFER_PROGRESS_BASE_ID + key)
    }

    fun showTransferCompleted(
        context: Context,
        peerName: String,
        summary: String,
        autoAccepted: Boolean,
    ) {
        if (!hasPostNotificationsPermission(context)) return
        ensureTransferChannel(context)
        val manager = context.getSystemService(NotificationManager::class.java) ?: return

        val openPending = PendingIntent.getActivity(
            context,
            301,
            Intent(context, MainActivity::class.java).apply {
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP
            },
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        val suffix = if (autoAccepted) " (auto-accepted)" else ""
        val notification = NotificationCompat.Builder(context, TRANSFER_CHANNEL_ID)
            .setSmallIcon(android.R.drawable.stat_sys_download_done)
            .setContentTitle(context.getString(R.string.transfer_complete_title))
            .setContentText("$peerName: $summary$suffix")
            .setPriority(NotificationCompat.PRIORITY_DEFAULT)
            .setContentIntent(openPending)
            .setAutoCancel(true)
            .build()

        manager.notify(TRANSFER_COMPLETE_ID, notification)
    }

    fun showTransferError(
        context: Context,
        summary: String,
        reason: String,
    ) {
        if (!hasPostNotificationsPermission(context)) return
        ensureTransferChannel(context)
        val manager = context.getSystemService(NotificationManager::class.java) ?: return

        val notification = NotificationCompat.Builder(context, TRANSFER_CHANNEL_ID)
            .setSmallIcon(android.R.drawable.stat_notify_error)
            .setContentTitle(context.getString(R.string.transfer_error_title))
            .setContentText("$summary: $reason")
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .setAutoCancel(true)
            .build()

        manager.notify(TRANSFER_ERROR_ID, notification)
    }

    fun showLnpBlockedNotification(context: Context) {
        if (!hasPostNotificationsPermission(context)) return
        ensureTransferChannel(context)
        val manager = context.getSystemService(NotificationManager::class.java) ?: return

        val settingsIntent = Intent(
            Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
            Uri.fromParts("package", context.packageName, null),
        ).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK
        }
        val settingsPending = PendingIntent.getActivity(
            context,
            302,
            settingsIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        val openPending = PendingIntent.getActivity(
            context,
            303,
            Intent(context, MainActivity::class.java).apply {
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP
            },
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        val notification = NotificationCompat.Builder(context, TRANSFER_CHANNEL_ID)
            .setSmallIcon(android.R.drawable.stat_sys_warning)
            .setContentTitle(context.getString(R.string.lnp_blocked_title))
            .setContentText(context.getString(R.string.lnp_blocked_body))
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .setContentIntent(openPending)
            .addAction(0, context.getString(R.string.action_open_settings), settingsPending)
            .setAutoCancel(true)
            .build()

        manager.notify(LNP_BLOCKED_ID, notification)
    }

    fun cancelLnpBlockedNotification(context: Context) {
        val manager = context.getSystemService(NotificationManager::class.java) ?: return
        manager.cancel(LNP_BLOCKED_ID)
    }

    fun cancel(context: Context) {
        val manager = context.getSystemService(NotificationManager::class.java) ?: return
        manager.cancel(NOTIFICATION_ID)
    }
}

