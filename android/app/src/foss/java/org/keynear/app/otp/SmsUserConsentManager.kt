package org.keynear.app.otp

import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.util.Log
import androidx.core.app.NotificationCompat
import org.keynear.app.R
import org.keynear.app.bridge.CoreClientProvider
import org.keynear.app.bridge.UniffiCoreClient
import java.util.concurrent.ExecutorService
import java.util.concurrent.atomic.AtomicBoolean

class SmsUserConsentManager(
    context: Context,
    @Suppress("UNUSED_PARAMETER")
    onConsentIntentReady: ((Intent) -> Unit)? = null,
) {
    private val appContext = context.applicationContext
    private val unavailableWarned = AtomicBoolean(false)

    fun startIfEnabled(otpPassthroughEnabled: Boolean) {
        if (!otpPassthroughEnabled) {
            return
        }
        if (unavailableWarned.compareAndSet(false, true)) {
            Log.i(TAG, "sms_user_consent_unavailable_foss_build")
            postUnavailableNotification()
        }
    }

    fun requestNextSmsConsent() = Unit

    fun stop() = Unit

    fun shutdown() = Unit

    private fun postUnavailableNotification() {
        val manager = appContext.getSystemService(NotificationManager::class.java) ?: return
        val channel = NotificationChannel(
            OTP_CHANNEL_ID,
            appContext.getString(R.string.otp_consent_channel_name),
            NotificationManager.IMPORTANCE_LOW,
        ).apply {
            description = appContext.getString(R.string.otp_consent_channel_description)
        }
        manager.createNotificationChannel(channel)
        val notification = NotificationCompat.Builder(appContext, OTP_CHANNEL_ID)
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setContentTitle(appContext.getString(R.string.otp_play_services_unavailable_title))
            .setContentText(appContext.getString(R.string.otp_play_services_unavailable_body))
            .setAutoCancel(true)
            .build()
        manager.notify(PLAY_SERVICES_UNAVAILABLE_NOTIF_ID, notification)
    }

    companion object {
        private const val TAG = "SmsUserConsentMgr"
        private const val OTP_CHANNEL_ID = "keynear_otp_consent"
        private const val PLAY_SERVICES_UNAVAILABLE_NOTIF_ID = 2201
        const val ACTION_HANDLE_SMS_CONSENT = "org.keynear.app.action.HANDLE_SMS_CONSENT"
        const val EXTRA_PENDING_CONSENT_INTENT = "extra_pending_consent_intent"

        fun setForegroundConsentLauncher(@Suppress("UNUSED_PARAMETER") launcher: ((Intent) -> Unit)?) = Unit

        fun handleConsentResult(
            @Suppress("UNUSED_PARAMETER") context: Context,
            @Suppress("UNUSED_PARAMETER") resultCode: Int,
            @Suppress("UNUSED_PARAMETER") data: Intent?,
            @Suppress("UNUSED_PARAMETER") client: UniffiCoreClient = CoreClientProvider.init(context),
            @Suppress("UNUSED_PARAMETER") executor: ExecutorService? = null,
        ): Boolean = false
    }
}

