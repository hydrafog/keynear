package org.keynear.app.share

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.OpenableColumns
import androidx.core.content.IntentCompat

object IntentResolver {
    fun extractStreamUris(intent: Intent?, action: String?): List<String> {
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

    fun computeTotalShareBytes(
        context: Context,
        uris: List<String>,
        sharedText: String?,
    ): Long {
        var total = 0L
        uris.forEach { raw ->
            runCatching {
                context.contentResolver.query(
                    Uri.parse(raw),
                    arrayOf(OpenableColumns.SIZE),
                    null,
                    null,
                    null,
                )?.use { cursor ->
                    if (cursor.moveToFirst()) {
                        val sizeIdx = cursor.getColumnIndex(OpenableColumns.SIZE)
                        if (sizeIdx >= 0 && !cursor.isNull(sizeIdx)) {
                            total += cursor.getLong(sizeIdx).coerceAtLeast(0L)
                        }
                    }
                }
            }
        }
        if (!sharedText.isNullOrBlank()) {
            total += sharedText.trim().toByteArray(Charsets.UTF_8).size.toLong()
        }
        return total
    }

    fun extractPreselectedDeviceId(intent: Intent?): String? =
        DirectShareManager.extractDeviceId(
            shortcutId = intent?.getStringExtra(Intent.EXTRA_SHORTCUT_ID),
            explicitDeviceId = intent?.getStringExtra(DirectShareManager.EXTRA_TARGET_DEVICE_ID),
        )
}

