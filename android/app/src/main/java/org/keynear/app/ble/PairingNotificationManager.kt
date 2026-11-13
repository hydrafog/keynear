package org.keynear.app.ble

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import org.keynear.app.MainActivity
import org.keynear.app.R
import org.keynear.app.service.KeynearForegroundService

object PairingNotificationManager {
    const val CHANNEL_ID = "keynear_pairing"
    const val NOTIFICATION_ID = 1002
    const val PAIRING_TIMEOUT_MS = 120_000L

    fun formatCode(rawSixDigits: String): String {
        val digits = rawSixDigits.filter { it.isDigit() }
        if (digits.length != 6) return rawSixDigits
        return "${digits.substring(0, 3)} ${digits.substring(3, 6)}"
    }

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

    fun showVerificationPrompt(
        context: Context,
        peerName: String,
        verificationCode: String,
    ) {
        if (Build.VERSION.SDK_INT >= 33 &&
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) !=
            PackageManager.PERMISSION_GRANTED
        ) {
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

    fun cancel(context: Context) {
        val manager = context.getSystemService(NotificationManager::class.java) ?: return
        manager.cancel(NOTIFICATION_ID)
    }
}

