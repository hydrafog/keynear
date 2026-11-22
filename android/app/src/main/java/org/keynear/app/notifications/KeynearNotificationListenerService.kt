package org.keynear.app.notifications

import android.app.Notification
import android.content.SharedPreferences
import android.service.notification.NotificationListenerService
import android.service.notification.StatusBarNotification
import uniffi.keynear_core.FfiCapabilityPair
import uniffi.keynear_core.FfiNotificationDescriptor
import uniffi.keynear_core.carinaCapabilityFlags
import uniffi.keynear_core.carinaMirrorNotification

class KeynearNotificationListenerService : NotificationListenerService() {

    private var lastSequence: Long = 0L

    override fun onListenerConnected() {
        super.onListenerConnected()
        android.util.Log.i(TAG, "notification listener connected")
    }

    override fun onListenerDisconnected() {
        super.onListenerDisconnected()
        android.util.Log.i(TAG, "notification listener disconnected")
    }

    override fun onNotificationPosted(sbn: StatusBarNotification?) {
        val notification = sbn ?: return
        if (!isOptedIn()) return
        val descriptor = descriptorOf(notification) ?: return
        val flags = FfiCapabilityPair(carinaCapabilityFlags(), carinaCapabilityFlags())
        val mirrored = carinaMirrorNotification(
            descriptor,
            excludedPackages().toList(),
            flags,
            lastSequence,
        ) ?: return
        lastSequence = maxOf(lastSequence, mirrored.sequence)
        android.util.Log.i(TAG, "mirrored notification key=${mirrored.notification_key.length}")
    }

    private fun descriptorOf(sbn: StatusBarNotification): FfiNotificationDescriptor? {
        val notification: Notification = sbn.notification ?: return null
        val title = notification.extras?.getCharSequence(Notification.EXTRA_TITLE)?.toString() ?: ""
        val body = notification.extras?.getCharSequence(Notification.EXTRA_TEXT)?.toString() ?: ""
        val descriptor = NotificationMirror.Descriptor(
            key = sbn.key,
            packageName = sbn.packageName,
            title = title,
            body = body,
            sequence = sbn.postTime,
            supportsReply = true,
            supportsMarkRead = true,
            supportsDismiss = true,
        )
        if (!NotificationMirror.isValidText(descriptor.key, NotificationMirror.MAX_KEY_LENGTH)) {
            return null
        }
        if (!NotificationMirror.isValidText(
                descriptor.packageName,
                NotificationMirror.MAX_PACKAGE_LENGTH,
            )
        ) {
            return null
        }
        return FfiNotificationDescriptor(
            key = descriptor.key,
            package = descriptor.packageName,
            title = descriptor.title,
            body = descriptor.body,
            category = notification.category ?: "",
            sequence = descriptor.sequence,
            supportsReply = descriptor.supportsReply,
            supportsMarkRead = descriptor.supportsMarkRead,
            supportsDismiss = descriptor.supportsDismiss,
        )
    }

    private fun preferences(): SharedPreferences {
        return getSharedPreferences(PREFERENCES_NAME, MODE_PRIVATE)
    }

    private fun isOptedIn(): Boolean {
        return preferences().getBoolean(KEY_OPT_IN, false)
    }

    private fun excludedPackages(): Set<String> {
        val stored = preferences().getString(KEY_EXCLUSIONS, "") ?: ""
        return stored.split(SEPARATOR).filter { it.isNotEmpty() }.toSet()
    }

    companion object {
        private const val TAG: String = "KeynearMirror"
        private const val PREFERENCES_NAME: String = "org.keynear.app.carina"
        private const val KEY_OPT_IN: String = "notification_mirror_enabled"
        private const val KEY_EXCLUSIONS: String = "notification_exclusions"
        private const val SEPARATOR: String = ","
    }
}
