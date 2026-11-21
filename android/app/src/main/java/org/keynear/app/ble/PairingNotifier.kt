package org.keynear.app.ble

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.content.ContextCompat
import org.keynear.app.R

object PairingNotifier {
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

    internal fun hasPostNotificationsPermission(context: Context): Boolean =
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
    fun cancelTransferNotification(context: Context, transferId: String) {
        val manager = context.getSystemService(NotificationManager::class.java) ?: return
        val key = transferNotificationKey(transferId)
        manager.cancel(TRANSFER_CONSENT_BASE_ID + key)
        manager.cancel(TRANSFER_PROGRESS_BASE_ID + key)
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
