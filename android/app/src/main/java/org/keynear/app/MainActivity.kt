package org.keynear.app

import android.content.Intent
import android.graphics.Color as AndroidColor
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import org.keynear.app.bridge.CoreClientProvider
import org.keynear.app.otp.OtpConsentVerifier
import org.keynear.app.otp.SmsUserConsentManager

class MainActivity : ComponentActivity() {
    private var resumeTick by mutableIntStateOf(0)
    private var pendingConsentIntent by mutableStateOf<Intent?>(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge(
            statusBarStyle = SystemBarStyle.auto(AndroidColor.TRANSPARENT, AndroidColor.TRANSPARENT),
            navigationBarStyle = SystemBarStyle.auto(AndroidColor.TRANSPARENT, AndroidColor.TRANSPARENT),
        )
        extractPendingConsentIntent(intent)?.let { pendingConsentIntent = it }
        val bridge = CoreClientProvider.init(applicationContext)
        setContent {
            KeynearAppRoot(
                bridge = bridge,
                resumeTick = resumeTick,
                pendingConsentIntent = pendingConsentIntent,
                onConsentConsumed = { pendingConsentIntent = null },
            )
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        extractPendingConsentIntent(intent)?.let { pendingConsentIntent = it }
    }

    override fun onResume() {
        super.onResume()
        resumeTick += 1
    }

    private fun extractPendingConsentIntent(source: Intent?): Intent? {
        if (source?.action != SmsUserConsentManager.ACTION_HANDLE_SMS_CONSENT) return null
        @Suppress("DEPRECATION")
        val consent = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            source.getParcelableExtra(SmsUserConsentManager.EXTRA_PENDING_CONSENT_INTENT, Intent::class.java)
        } else {
            source.getParcelableExtra(SmsUserConsentManager.EXTRA_PENDING_CONSENT_INTENT)
        } ?: return null
        val pkg = consent.resolveActivity(packageManager)?.packageName
        return consent.takeIf { OtpConsentVerifier.isConsentIntentTrusted(pkg) }
    }
}
