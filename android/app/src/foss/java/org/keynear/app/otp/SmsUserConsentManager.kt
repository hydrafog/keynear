package org.keynear.app.otp

import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.util.Log
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
        appContext.getSystemService(NotificationManager::class.java)
            ?.cancel(PLAY_SERVICES_UNAVAILABLE_NOTIF_ID)
        if (!otpPassthroughEnabled) {
            return
        }
        if (unavailableWarned.compareAndSet(false, true)) {
            Log.i(TAG, "sms_user_consent_unavailable_foss_build")
        }
    }

    fun requestNextSmsConsent() = Unit

    fun stop() = Unit

    fun shutdown() = Unit

    companion object {
        private const val TAG = "SmsUserConsentMgr"
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

