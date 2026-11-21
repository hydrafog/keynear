package org.keynear.app.ble

import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import androidx.core.app.NotificationCompat
import org.keynear.app.MainActivity
import org.keynear.app.R
import org.keynear.app.service.KeynearForegroundService

internal fun PairingNotifier.showVerificationPrompt(
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

internal fun PairingNotifier.showIncomingTransferConsent(
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
