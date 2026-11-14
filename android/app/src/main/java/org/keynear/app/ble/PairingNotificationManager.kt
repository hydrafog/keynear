package org.keynear.app.ble

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import org.keynear.app.MainActivity
import org.keynear.app.R
import org.keynear.app.service.KeynearForegroundService

object PairingNotificationManager {
    const val CHANNEL_ID = "keynear_pairing"
    const val TRANSFER_CHANNEL_ID = "keynear_transfers"
    const val NOTIFICATION_ID = 1002
    const val TRANSFER_COMPLETE_ID = 1003
    const val TRANSFER_ERROR_ID = 1004
    const val LNP_BLOCKED_ID = 1005
    const val TRANSFER_CONSENT_BASE_ID = 2000
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
        val key = transferId.hashCode() and 0x0FFF
        manager.cancel(TRANSFER_CONSENT_BASE_ID + key)
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

