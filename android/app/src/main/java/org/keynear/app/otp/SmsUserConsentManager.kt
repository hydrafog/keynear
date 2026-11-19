package org.keynear.app.otp

import android.app.Activity
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.os.Build
import android.util.Base64
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import com.google.android.gms.auth.api.phone.SmsRetriever
import com.google.android.gms.common.ConnectionResult
import com.google.android.gms.common.GoogleApiAvailability
import com.google.android.gms.common.api.CommonStatusCodes
import com.google.android.gms.common.api.Status
import org.keynear.app.MainActivity
import org.keynear.app.R
import org.keynear.app.bridge.CoreClientProvider
import org.keynear.app.bridge.UniffiCoreClient
import java.security.MessageDigest
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

object OtpConsentVerifier {
    const val GOOGLE_PLAY_SERVICES_PACKAGE = "com.google.android.gms"
    private val OTP_TOKEN_REGEX = Regex("""\b(\d{4,10}|[A-Z0-9]{6,10})\b""")
    private val APP_HASH_REGEX = Regex("""^<#>\s+.*?\s([A-Za-z0-9+/]{11})\s*$""", RegexOption.DOT_MATCHES_ALL)

    fun isConsentIntentTrusted(resolvedPackage: String?): Boolean =
        resolvedPackage == GOOGLE_PLAY_SERVICES_PACKAGE

    fun verifySmsAppHash(smsBody: String, expectedAppHashes: Set<String>): Boolean {
        if (expectedAppHashes.isEmpty()) return true
        val match = APP_HASH_REGEX.find(smsBody.trim())?.groupValues?.getOrNull(1) ?: return true
        return match in expectedAppHashes
    }

    fun extractVerifiedOtpCode(
        smsBody: String,
        expectedAppHashes: Set<String> = emptySet(),
    ): String? {
        val trimmed = smsBody.trim()
        if (trimmed.isEmpty()) return null
        if (!verifySmsAppHash(trimmed, expectedAppHashes)) {
            return null
        }
        val candidates = OTP_TOKEN_REGEX.findAll(trimmed)
            .map { it.groupValues[1] }
            .filter { token -> token.any { it.isDigit() } }
            .toList()
        return candidates.firstOrNull { it.all { ch -> ch.isDigit() } } ?: candidates.firstOrNull()
    }

    fun computeAppSignatureHash(packageName: String, signatureBytes: ByteArray): String {
        val info = "$packageName ${signatureBytes.joinToString("") { "%02x".format(it) }}"
        val digest = MessageDigest.getInstance("SHA-256").digest(info.toByteArray(Charsets.UTF_8))
        return Base64.encodeToString(digest.copyOf(9), Base64.NO_PADDING or Base64.NO_WRAP).take(11)
    }
}

class SmsUserConsentManager(
    private val context: Context,
    private val onConsentIntentReady: ((Intent) -> Unit)? = null,
) {
    private val appContext = context.applicationContext
    private val registered = AtomicBoolean(false)
    private val playServicesWarned = AtomicBoolean(false)
    private val executor: ExecutorService = Executors.newSingleThreadExecutor()

    private val smsReceiver = object : BroadcastReceiver() {
        override fun onReceive(ctx: Context, intent: Intent) {
            if (intent.action != SmsRetriever.SMS_RETRIEVED_ACTION) return
            val client = CoreClientProvider.init(appContext)
            if (!client.isOtpPassthroughEnabled()) return

            val extras = intent.extras ?: return
            @Suppress("DEPRECATION")
            val status = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                extras.getParcelable(SmsRetriever.EXTRA_STATUS, Status::class.java)
            } else {
                extras.getParcelable(SmsRetriever.EXTRA_STATUS)
            } ?: return

            if (status.statusCode != CommonStatusCodes.SUCCESS) {
                if (status.statusCode == CommonStatusCodes.TIMEOUT) {
                    requestNextSmsConsent()
                }
                return
            }

            @Suppress("DEPRECATION")
            val consentIntent = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                extras.getParcelable(SmsRetriever.EXTRA_CONSENT_INTENT, Intent::class.java)
            } else {
                extras.getParcelable(SmsRetriever.EXTRA_CONSENT_INTENT)
            } ?: return

            val resolvedPkg = consentIntent.resolveActivity(appContext.packageManager)?.packageName
            if (!OtpConsentVerifier.isConsentIntentTrusted(resolvedPkg)) {
                Log.w(TAG, "rejected_untrusted_consent_intent: $resolvedPkg")
                return
            }

            val foregroundLauncher = foregroundConsentLauncher ?: onConsentIntentReady
            if (foregroundLauncher != null) {
                foregroundLauncher(consentIntent)
            } else {
                postConsentNotification(consentIntent)
            }
        }
    }

    fun startIfEnabled(otpPassthroughEnabled: Boolean) {
        if (!otpPassthroughEnabled) {
            stop()
            return
        }
        val gmsStatus = runCatching {
            GoogleApiAvailability.getInstance().isGooglePlayServicesAvailable(appContext)
        }.getOrDefault(ConnectionResult.SERVICE_MISSING)

        if (gmsStatus != ConnectionResult.SUCCESS) {
            if (playServicesWarned.compareAndSet(false, true)) {
                postPlayServicesUnavailableNotification()
            }
            return
        }

        if (registered.compareAndSet(false, true)) {
            val filter = IntentFilter(SmsRetriever.SMS_RETRIEVED_ACTION)
            ContextCompat.registerReceiver(
                appContext,
                smsReceiver,
                filter,
                SmsRetriever.SEND_PERMISSION,
                null,
                ContextCompat.RECEIVER_EXPORTED,
            )
        }
        requestNextSmsConsent()
    }

    fun requestNextSmsConsent() {
        if (!registered.get()) return
        runCatching {
            SmsRetriever.getClient(appContext).startSmsUserConsent(null)
        }.onFailure { err ->
            Log.w(TAG, "sms_user_consent_start_failed: ${err.message}")
        }
    }

    fun stop() {
        if (registered.compareAndSet(true, false)) {
            runCatching { appContext.unregisterReceiver(smsReceiver) }
        }
    }

    fun shutdown() {
        stop()
        executor.shutdown()
    }

    private fun postPlayServicesUnavailableNotification() {
        val manager = appContext.getSystemService(NotificationManager::class.java) ?: return
        ensureOtpChannel(manager)
        val notification = NotificationCompat.Builder(appContext, OTP_CHANNEL_ID)
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setContentTitle(appContext.getString(R.string.otp_play_services_unavailable_title))
            .setContentText(appContext.getString(R.string.otp_play_services_unavailable_body))
            .setAutoCancel(true)
            .build()
        manager.notify(PLAY_SERVICES_UNAVAILABLE_NOTIF_ID, notification)
    }

    private fun postConsentNotification(consentIntent: Intent) {
        val manager = appContext.getSystemService(NotificationManager::class.java) ?: return
        ensureOtpChannel(manager)
        val launchIntent = Intent(appContext, MainActivity::class.java).apply {
            action = ACTION_HANDLE_SMS_CONSENT
            putExtra(EXTRA_PENDING_CONSENT_INTENT, consentIntent)
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP)
        }
        val pendingIntent = PendingIntent.getActivity(
            appContext,
            CONSENT_REQUEST_CODE,
            launchIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val notification = NotificationCompat.Builder(appContext, OTP_CHANNEL_ID)
            .setSmallIcon(android.R.drawable.ic_dialog_email)
            .setContentTitle(appContext.getString(R.string.otp_consent_prompt_title))
            .setContentText(appContext.getString(R.string.otp_consent_prompt_body))
            .setContentIntent(pendingIntent)
            .setAutoCancel(true)
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .build()
        manager.notify(OTP_CONSENT_NOTIF_ID, notification)
    }

    private fun ensureOtpChannel(manager: NotificationManager) {
        val channel = NotificationChannel(
            OTP_CHANNEL_ID,
            appContext.getString(R.string.otp_consent_channel_name),
            NotificationManager.IMPORTANCE_HIGH,
        ).apply {
            description = appContext.getString(R.string.otp_consent_channel_description)
        }
        manager.createNotificationChannel(channel)
    }

    companion object {
        private const val TAG = "SmsUserConsentMgr"
        private const val OTP_CHANNEL_ID = "keynear_otp_consent"
        private const val PLAY_SERVICES_UNAVAILABLE_NOTIF_ID = 2201
        private const val OTP_CONSENT_NOTIF_ID = 2202
        private const val CONSENT_REQUEST_CODE = 2203
        const val ACTION_HANDLE_SMS_CONSENT = "org.keynear.app.action.HANDLE_SMS_CONSENT"
        const val EXTRA_PENDING_CONSENT_INTENT = "extra_pending_consent_intent"

        @Volatile
        private var foregroundConsentLauncher: ((Intent) -> Unit)? = null

        fun setForegroundConsentLauncher(launcher: ((Intent) -> Unit)?) {
            foregroundConsentLauncher = launcher
        }

        fun handleConsentResult(
            context: Context,
            resultCode: Int,
            data: Intent?,
            client: UniffiCoreClient = CoreClientProvider.init(context),
            executor: ExecutorService? = null,
        ): Boolean {
            if (resultCode != Activity.RESULT_OK || data == null) {
                return false
            }
            if (!client.isOtpPassthroughEnabled()) {
                return false
            }
            val smsMessage = data.getStringExtra(SmsRetriever.EXTRA_SMS_MESSAGE) ?: return false
            val expectedHashes = loadExpectedAppHashes(context)
            val code = OtpConsentVerifier.extractVerifiedOtpCode(smsMessage, expectedHashes) ?: return false

            val runSend = Runnable {
                client.sendVerifiedOtp(code).onFailure { err ->
                    Log.w(TAG, "otp_forward_failed: ${err.message}")
                }
            }
            if (executor != null) {
                executor.execute(runSend)
            } else {
                Thread(runSend).start()
            }
            return true
        }

        private fun loadExpectedAppHashes(context: Context): Set<String> = runCatching {
            val info = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
                context.packageManager.getPackageInfo(
                    context.packageName,
                    PackageManager.GET_SIGNING_CERTIFICATES,
                )
            } else {
                @Suppress("DEPRECATION")
                context.packageManager.getPackageInfo(
                    context.packageName,
                    PackageManager.GET_SIGNATURES,
                )
            }
            val sigs = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
                info.signingInfo?.apkContentsSigners
            } else {
                @Suppress("DEPRECATION")
                info.signatures
            } ?: return@runCatching emptySet()

            sigs.map { sig ->
                OtpConsentVerifier.computeAppSignatureHash(context.packageName, sig.toByteArray())
            }.toSet()
        }.getOrDefault(emptySet())
    }
}
