package org.keynear.app.share

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.util.Log
import androidx.core.content.IntentCompat
import org.keynear.app.core.ShareValidationResult
import org.keynear.app.core.validateShareIntent

class ShareActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val incoming = intent
        val action = incoming?.action
        val streamUris = extractStreamUris(incoming, action)
        val sharedText = incoming?.getStringExtra(Intent.EXTRA_TEXT)

        when (val result = validateShareIntent(action, streamUris, sharedText)) {
            is ShareValidationResult.Valid -> {
                Log.i(
                    TAG,
                    "share_intent_validated items=${result.itemCount} has_text=${result.hasText}",
                )
            }
            is ShareValidationResult.Invalid -> {
                Log.w(TAG, "share_intent_rejected reason=${result.reason}")
            }
        }

        finish()
    }

    private fun extractStreamUris(intent: Intent?, action: String?): List<String> {
        if (intent == null) return emptyList()
        return when (action) {
            Intent.ACTION_SEND -> {
                val single = IntentCompat.getParcelableExtra(intent, Intent.EXTRA_STREAM, Uri::class.java)
                if (single != null) listOf(single.toString()) else emptyList()
            }
            Intent.ACTION_SEND_MULTIPLE -> {
                val multiple = IntentCompat.getParcelableArrayListExtra(
                    intent,
                    Intent.EXTRA_STREAM,
                    Uri::class.java,
                )
                multiple?.map { it.toString() } ?: emptyList()
            }
            else -> emptyList()
        }
    }

    companion object {
        private const val TAG = "ShareActivity"
    }
}
