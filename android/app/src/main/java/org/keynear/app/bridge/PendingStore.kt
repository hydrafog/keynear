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

interface PendingRowOps<T> {
    fun createPending(fileName: String, mimeType: String): T?
    fun openDetachedFd(handle: T): Int?
    fun clearPendingFlag(handle: T)
    fun deleteRow(handle: T)
}

class PendingMediaStoreTracker<T>(
    private val ops: PendingRowOps<T>,
) {
    private val pendingRows = ConcurrentHashMap<String, T>()

    fun prepareAndHandOff(
        transferId: String,
        fileName: String,
        mimeType: String,
        handOffFd: (Int) -> Unit,
    ): Boolean {
        val handle = ops.createPending(fileName, mimeType) ?: return false
        var handedOff = false
        try {
            val fd = ops.openDetachedFd(handle) ?: return false
            handOffFd(fd)
            handedOff = true
            pendingRows[transferId] = handle
            return true
        } finally {
            if (!handedOff) {
                ops.deleteRow(handle)
            }
        }
    }

    fun reconcileState(transferId: String, state: TransferStateUi) {
        when (state) {
            TransferStateUi.COMPLETED -> pendingRows.remove(transferId)?.let { ops.clearPendingFlag(it) }
            TransferStateUi.CANCELLED, TransferStateUi.FAILED ->
                pendingRows.remove(transferId)?.let { ops.deleteRow(it) }
            TransferStateUi.PENDING_CONSENT, TransferStateUi.ACTIVE -> Unit
        }
    }

    fun hasPending(transferId: String): Boolean = pendingRows.containsKey(transferId)

    fun cleanupAllPending() {
        val keys = pendingRows.keys.toList()
        for (key in keys) {
            pendingRows.remove(key)?.let { ops.deleteRow(it) }
        }
    }
}

internal class AndroidMediaStoreOps(
    private val context: Context,
) : PendingRowOps<Uri> {
    override fun createPending(fileName: String, mimeType: String): Uri? {
        val cleanName = sanitizeFileName(fileName)
        val values = ContentValues().apply {
            put(MediaStore.Downloads.DISPLAY_NAME, cleanName)
            put(MediaStore.Downloads.MIME_TYPE, mimeType.ifBlank { "application/octet-stream" })
            put(MediaStore.Downloads.RELATIVE_PATH, Environment.DIRECTORY_DOWNLOADS)
            put(MediaStore.Downloads.IS_PENDING, 1)
        }
        return runCatching {
            context.contentResolver.insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI, values)
        }.getOrNull()
    }

    override fun openDetachedFd(handle: Uri): Int? = runCatching {
        val pfd: ParcelFileDescriptor = context.contentResolver.openFileDescriptor(handle, "w")
            ?: return@runCatching null
        pfd.detachFd()
    }.getOrNull()

    override fun clearPendingFlag(handle: Uri) {
        val values = ContentValues().apply {
            put(MediaStore.Downloads.IS_PENDING, 0)
        }
        runCatching {
            context.contentResolver.update(handle, values, null, null)
        }
    }

    override fun deleteRow(handle: Uri) {
        runCatching {
            context.contentResolver.delete(handle, null, null)
        }
    }

    private fun sanitizeFileName(raw: String): String {
        val base = raw.substringAfterLast('/').substringAfterLast('\\').trim()
        return base.ifEmpty { "keynear-download.bin" }
    }
}
