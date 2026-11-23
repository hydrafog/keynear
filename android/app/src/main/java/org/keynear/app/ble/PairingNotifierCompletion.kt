package org.keynear.app.ble

import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.Settings
import androidx.core.app.NotificationCompat
import org.keynear.app.MainActivity
import org.keynear.app.R
import org.keynear.app.bridge.resolveDownloadMimeType
import org.keynear.app.ui.model.TransferUi

internal fun PairingNotifier.showTransferCompleted(
    context: Context,
    transfer: TransferUi,
    autoAccepted: Boolean,
    completedUri: Uri? = null,
) {
    if (!hasPostNotificationsPermission(context)) return
    ensureTransferChannel(context)
    val manager = context.getSystemService(NotificationManager::class.java) ?: return

    val mimeType = resolveDownloadMimeType(transfer.summaryName, transfer.mimeType)
    val viewIntent = completedUri?.let { uri ->
        Intent(Intent.ACTION_VIEW).apply {
            setDataAndType(uri, mimeType)
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION
        }.takeIf { it.resolveActivity(context.packageManager) != null }
    }
    val openIntent = viewIntent ?: Intent(context, MainActivity::class.java).apply {
        flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP
    }
    val openPending = PendingIntent.getActivity(
        context,
        301,
        openIntent,
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
    )

    val suffix = if (autoAccepted) " (auto-accepted)" else ""
    val notification = NotificationCompat.Builder(context, TRANSFER_CHANNEL_ID)
        .setSmallIcon(android.R.drawable.stat_sys_download_done)
        .setContentTitle(context.getString(R.string.transfer_complete_title))
        .setContentText("${transfer.peerDisplayName}: ${transfer.summaryName}$suffix")
        .setPriority(NotificationCompat.PRIORITY_DEFAULT)
        .setContentIntent(openPending)
        .setAutoCancel(true)
        .build()

    manager.notify(TRANSFER_COMPLETE_ID, notification)
}

internal fun PairingNotifier.showTransferError(
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

internal fun PairingNotifier.showLnpBlockedNotification(context: Context) {
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
