package org.keynear.app.ble

import android.app.Notification
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.graphics.drawable.Icon
import android.os.Build
import androidx.annotation.RequiresApi
import androidx.core.app.NotificationCompat
import org.keynear.app.MainActivity
import org.keynear.app.R
import org.keynear.app.ui.model.TransferDirectionUi
import org.keynear.app.ui.model.TransferProgressModel
import org.keynear.app.ui.model.TransferProgressStage
import org.keynear.app.ui.model.TransferUi
import org.keynear.app.ui.model.computeTransferProgressModel
import org.keynear.app.bridge.formatByteCount
import org.keynear.app.service.KeynearForegroundService

internal fun PairingNotifier.buildTransferProgressNotification(
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
    val pending = createTransferPendingIntents(context, transfer, key)
    val openPending = pending.open
    val cancelPending = pending.cancel
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
        buildLegacyTransferProgressNotification(
            context, transfer, model, stageTitle, progressBody, smallIcon, openPending, cancelPending,
        )
    }
}

private fun PairingNotifier.buildLegacyTransferProgressNotification(
    context: Context,
    transfer: TransferUi,
    model: TransferProgressModel,
    stageTitle: String,
    progressBody: String,
    smallIcon: Int,
    openPending: PendingIntent,
    cancelPending: PendingIntent,
): Notification = NotificationCompat.Builder(context, TRANSFER_PROGRESS_CHANNEL_ID)
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


private data class TransferPendingIntents(val open: PendingIntent, val cancel: PendingIntent)

private fun PairingNotifier.createTransferPendingIntents(
    context: Context,
    transfer: TransferUi,
    key: Int,
): TransferPendingIntents {
    val open = PendingIntent.getActivity(
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
    val cancel = PendingIntent.getService(
        context,
        5000 + key,
        cancelIntent,
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
    )
    return TransferPendingIntents(open, cancel)
}

@RequiresApi(36)
private fun PairingNotifier.buildApi36ProgressNotification(
    context: Context,
    smallIcon: Int,
    title: String,
    body: String,
    model: TransferProgressModel,
    openPending: PendingIntent,
    cancelPending: PendingIntent,
): Notification {
    val accentColor = 0xFF004CED.toInt()
    val focusColor = 0xFF0172F9.toInt()
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

internal fun PairingNotifier.showOrUpdateTransferProgress(
    context: Context,
    transfer: TransferUi,
) {
    if (!hasPostNotificationsPermission(context)) return
    val manager = context.getSystemService(NotificationManager::class.java) ?: return
    val notification = buildTransferProgressNotification(context, transfer, Build.VERSION.SDK_INT)
    manager.notify(progressNotificationId(transfer.transferId), notification)
}
