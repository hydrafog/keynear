package org.keynear.app.clipboard

import android.content.ClipDescription
import android.content.ClipboardManager
import android.os.Build
import android.os.Bundle
import android.util.Log
import androidx.activity.ComponentActivity
import org.keynear.app.core.CoreClientProvider
import org.keynear.app.core.CoreState
import org.keynear.app.core.MAX_CLIPBOARD_IMAGE_BYTES
import org.keynear.app.core.shouldReadClipboardOnFocus
import org.keynear.app.core.shouldSkipClipDescription
import java.io.ByteArrayOutputStream
import java.util.concurrent.Executors

class ClipboardReadActivity : ComponentActivity() {
    private var handled = false
    private val dispatchExecutor = Executors.newSingleThreadExecutor()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        overridePendingTransition(0, 0)
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (!shouldReadClipboardOnFocus(hasFocus, handled)) {
            return
        }
        handled = true
        readAndForwardPrimaryClip()
        finish()
        overridePendingTransition(0, 0)
    }

    private fun readAndForwardPrimaryClip() {
        val client = CoreClientProvider.init(applicationContext)
        val config = when (val state = client.loadState()) {
            is CoreState.Empty -> state.snapshot.daemonConfig
            is CoreState.Ready -> state.snapshot.daemonConfig
            is CoreState.Error -> state.previous?.daemonConfig
            CoreState.Loading -> null
        }
        if (config != null && !config.clipboardSyncEnabled) {
            return
        }

        val manager = getSystemService(ClipboardManager::class.java) ?: return
        val clip = runCatching { manager.primaryClip }.getOrNull() ?: return
        val description = clip.description ?: return
        val offeredMimes = (0 until description.mimeTypeCount).map { idx ->
            description.getMimeType(idx)
        }
        val isSensitive = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            description.extras?.getBoolean(ClipDescription.EXTRA_IS_SENSITIVE, false) == true
        } else {
            false
        }
        val ignored = config?.ignoredMimeTypes ?: emptyList()
        if (shouldSkipClipDescription(offeredMimes, isSensitive, ignored)) {
            return
        }
        if (clip.itemCount <= 0) {
            return
        }

        val item = clip.getItemAt(0)
        val uri = item.uri
        if (uri != null) {
            val resolvedMime = contentResolver.getType(uri)
                ?: offeredMimes.firstOrNull { it.startsWith("image/") }
            if (resolvedMime != null && resolvedMime.startsWith("image/")) {
                if (config != null && !config.clipboardImagesEnabled) {
                    return
                }
                if (shouldSkipClipDescription(listOf(resolvedMime), isSensitive, ignored)) {
                    return
                }
                dispatchExecutor.execute {
                    val bytes = readBoundedUriBytes(uri) ?: return@execute
                    client.sendPhoneClipboardBlob(resolvedMime, bytes).onFailure { err ->
                        Log.w(TAG, "clipboard_image_send_failed: ${err.message}")
                    }
                }
                return
            }
        }

        val text = item.coerceToText(this)?.toString().orEmpty()
        if (text.isBlank()) {
            return
        }
        dispatchExecutor.execute {
            client.sendPhoneClipboardText(text).onFailure { err ->
                Log.w(TAG, "clipboard_text_send_failed: ${err.message}")
            }
        }
    }

    private fun readBoundedUriBytes(uri: android.net.Uri): ByteArray? {
        return runCatching {
            contentResolver.openInputStream(uri)?.use { input ->
                val out = ByteArrayOutputStream()
                val buf = ByteArray(16 * 1024)
                var total = 0
                while (true) {
                    val read = input.read(buf)
                    if (read <= 0) break
                    total += read
                    if (total > MAX_CLIPBOARD_IMAGE_BYTES) {
                        return@runCatching null
                    }
                    out.write(buf, 0, read)
                }
                out.toByteArray().takeIf { it.isNotEmpty() }
            }
        }.getOrNull()
    }

    override fun onDestroy() {
        dispatchExecutor.shutdown()
        super.onDestroy()
    }

    companion object {
        private const val TAG = "ClipboardReadActivity"
    }
}

