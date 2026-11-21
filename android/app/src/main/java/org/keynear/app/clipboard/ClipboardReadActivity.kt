package org.keynear.app.clipboard

import android.content.ClipData
import android.content.ClipDescription
import android.content.ClipboardManager
import android.os.Build
import android.os.Bundle
import android.util.Log
import androidx.activity.ComponentActivity
import org.keynear.app.bridge.CoreClientProvider
import org.keynear.app.bridge.EventQueue
import org.keynear.app.ui.model.CoreState
import org.keynear.app.bridge.MAX_CLIPBOARD_IMAGE_BYTES
import org.keynear.app.bridge.shouldReadClipboardOnFocus
import org.keynear.app.bridge.shouldSkipClipDescription
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
        val config = resolveClipboardConfig(client)
        if (config != null && !config.clipboardSyncEnabled) {
            return
        }

        val ignored = config?.ignoredMimeTypes.orEmpty()
        val candidate = resolveClipCandidate(ignored) ?: return
        val imagesEnabled = config?.clipboardImagesEnabled ?: true
        val imageMime = imageMimeType(candidate.item.uri, candidate.offeredMimes)
        if (imageMime != null) {
            if (!imagesEnabled || shouldSkipClipDescription(listOf(imageMime), candidate.isSensitive, ignored)) {
                return
            }
            dispatchImage(client, candidate.item.uri!!, imageMime)
            return
        }

        val text = candidate.item.coerceToText(this)?.toString().orEmpty()
        if (text.isBlank()) {
            return
        }
        dispatchText(client, text)
    }

    private fun resolveClipboardConfig(client: EventQueue) =
        when (val state = client.loadState()) {
            is CoreState.Empty -> state.snapshot.daemonConfig
            is CoreState.Ready -> state.snapshot.daemonConfig
            is CoreState.Error -> state.previous?.daemonConfig
            CoreState.Loading -> null
        }

    private fun resolveClipCandidate(ignored: List<String>): ClipCandidate? {
        val manager = getSystemService(ClipboardManager::class.java) ?: return null
        val clip = runCatching { manager.primaryClip }.getOrNull() ?: return null
        val description = clip.description ?: return null
        val offeredMimes = (0 until description.mimeTypeCount).map { idx -> description.getMimeType(idx) }
        val isSensitive = Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            description.extras?.getBoolean(ClipDescription.EXTRA_IS_SENSITIVE, false) == true
        if (shouldSkipClipDescription(offeredMimes, isSensitive, ignored)) {
            return null
        }
        if (clip.itemCount <= 0) {
            return null
        }
        return ClipCandidate(clip.getItemAt(0), offeredMimes, isSensitive, ignored)
    }

    private fun imageMimeType(uri: android.net.Uri?, offeredMimes: List<String>): String? {
        if (uri == null) return null
        val resolved = contentResolver.getType(uri)
            ?: offeredMimes.firstOrNull { it.startsWith("image/") }
        return resolved?.takeIf { it.startsWith("image/") }
    }

    private fun dispatchImage(client: EventQueue, uri: android.net.Uri, mime: String) {
        dispatchExecutor.execute {
            val bytes = readBoundedUriBytes(uri) ?: return@execute
            client.sendPhoneClipboardBlob(mime, bytes).onFailure { err ->
                Log.w(TAG, "clipboard_image_send_failed: ${err.message}")
            }
        }
    }

    private fun dispatchText(client: EventQueue, text: String) {
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

    private data class ClipCandidate(
        val item: ClipData.Item,
        val offeredMimes: List<String>,
        val isSensitive: Boolean,
        val ignored: List<String>,
    )

    companion object {
        private const val TAG = "ClipboardReadActivity"
    }
}
