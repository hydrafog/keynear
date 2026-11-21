package org.keynear.app.bridge

import android.content.ContentValues
import android.content.Context
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.os.ParcelFileDescriptor
import android.provider.MediaStore
import org.keynear.app.ui.model.BluetoothStatus
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.CoreState
import org.keynear.app.ui.model.DaemonConfigUi
import org.keynear.app.ui.model.DiscoveredPeerUi
import org.keynear.app.ui.model.LocalNetworkStatus
import org.keynear.app.ui.model.PendingPairingUi
import org.keynear.app.ui.model.SendItemKind
import org.keynear.app.ui.model.SendQueueItemUi
import org.keynear.app.ui.model.ShareValidationResult
import org.keynear.app.ui.model.TransferStateUi
import org.keynear.app.ui.model.TransferUi
import org.keynear.app.ui.model.VisibilityState
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong

private const val CLIPBOARD_PREVIEW_LEN = 36
private const val SNIPPET_PREVIEW_LEN = 32

class SendQueueStore {
    private val items = mutableListOf<SendQueueItemUi>()
    private val seq = AtomicLong(1L)

    @Synchronized
    fun snapshot(): List<SendQueueItemUi> = items.toList()

    @Synchronized
    fun addFile(
        name: String,
        sizeBytes: Long,
        mime: String,
        uriString: String,
    ): SendQueueItemUi {
        val item = SendQueueItemUi(
            id = "sq-${seq.getAndIncrement()}",
            name = name.ifBlank { "shared-file.bin" },
            sizeBytes = sizeBytes.coerceAtLeast(0L),
            kind = SendItemKind.FILE,
            mime = mime.ifBlank { "application/octet-stream" },
            uriString = uriString,
        )
        items.add(item)
        return item
    }

    @Synchronized
    fun addFolder(
        displayName: String,
        uriString: String,
        sizeBytes: Long = 0L,
    ): SendQueueItemUi {
        val clean = displayName.trim().ifBlank { "Folder" }
        val item = SendQueueItemUi(
            id = "sq-${seq.getAndIncrement()}",
            name = "$clean/",
            sizeBytes = sizeBytes.coerceAtLeast(0L),
            kind = SendItemKind.FOLDER,
            mime = "inode/directory",
            uriString = uriString,
        )
        items.add(item)
        return item
    }

    @Synchronized
    fun addClipboard(text: String): SendQueueItemUi? {
        val clean = text.trim()
        if (clean.isEmpty()) return null
        val bytes = clean.toByteArray(Charsets.UTF_8).size.toLong()
        val label = if (clean.length > CLIPBOARD_PREVIEW_LEN) "${clean.take(CLIPBOARD_PREVIEW_LEN)}..." else clean
        val item = SendQueueItemUi(
            id = "sq-${seq.getAndIncrement()}",
            name = "Clipboard: $label",
            sizeBytes = bytes,
            kind = SendItemKind.CLIPBOARD,
            mime = "text/plain",
            textPayload = clean,
        )
        items.add(item)
        return item
    }

    @Synchronized
    fun addTextSnippet(text: String): SendQueueItemUi? {
        val clean = text.trim()
        if (clean.isEmpty()) return null
        val bytes = clean.toByteArray(Charsets.UTF_8).size.toLong()
        val label = if (clean.length > SNIPPET_PREVIEW_LEN) "${clean.take(SNIPPET_PREVIEW_LEN)}..." else clean
        val item = SendQueueItemUi(
            id = "sq-${seq.getAndIncrement()}",
            name = "Text: $label",
            sizeBytes = bytes,
            kind = SendItemKind.TEXT,
            mime = "text/plain",
            textPayload = clean,
        )
        items.add(item)
        return item
    }

    @Synchronized
    fun remove(itemId: String) {
        items.removeAll { it.id == itemId }
    }

    @Synchronized
    fun clear() {
        items.clear()
    }
}
