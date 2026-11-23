package org.keynear.app.data

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import org.keynear.app.ble.PairingNotifier
import org.keynear.app.ble.showOrUpdateTransferProgress
import org.keynear.app.bridge.PendingMediaStoreTracker
import org.keynear.app.service.KeynearTransferJobService
import org.keynear.app.ui.model.SendItemKind
import org.keynear.app.ui.model.SendQueueItemUi
import org.keynear.app.ui.model.TransferDirectionUi
import org.keynear.app.ui.model.TransferStateUi
import org.keynear.app.ui.model.TransferUi
import uniffi.keynear_core.FfiTransferSession
import uniffi.keynear_core.OrionNodeHandle
import java.io.File
import java.io.FileOutputStream
import java.util.UUID

data class OutboundTarget(
    val context: Context,
    val node: OrionNodeHandle,
    val peerDeviceId: String,
    val peerNames: Map<String, String>,
)

object TransferRepo {
    private const val STREAM_COPY_BUFFER_BYTES = 65_536

    fun queryContentUriMetadata(context: Context, uri: Uri): Triple<String, Long, String> {
        var name = uri.lastPathSegment?.substringAfterLast('/')?.ifBlank { null } ?: "shared-file.bin"
        var size = 0L
        runCatching {
            val cols = arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE)
            context.contentResolver.query(uri, cols, null, null, null)?.use { cursor ->
                if (cursor.moveToFirst()) {
                    val nameIdx = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                    if (nameIdx >= 0) {
                        cursor.getString(nameIdx)?.takeIf { it.isNotBlank() }?.let { name = it }
                    }
                    val sizeIdx = cursor.getColumnIndex(OpenableColumns.SIZE)
                    if (sizeIdx >= 0 && !cursor.isNull(sizeIdx)) {
                        size = cursor.getLong(sizeIdx)
                    }
                }
            }
        }
        val mime = context.contentResolver.getType(uri)?.ifBlank { null } ?: "application/octet-stream"
        return Triple(name, size, mime)
    }

    fun summarizeTransferItems(session: FfiTransferSession): String {
        val items = session.items
        if (items.isEmpty()) return "Transfer"
        if (items.size == 1) return items[0].name
        return "${items[0].name} (+${items.size - 1} more)"
    }

    fun mapTransferSessionToUi(session: FfiTransferSession, peerNames: Map<String, String>): TransferUi =
        TransferUi(
            transferId = session.transferId,
            peerDeviceId = session.peerDeviceId,
            peerDisplayName = peerNames[session.peerDeviceId] ?: session.peerDeviceId,
            summaryName = summarizeTransferItems(session),
            mimeType = session.items.firstOrNull()?.mimeType ?: "application/octet-stream",
            isDirectory = session.items.any { it.isDirectory },
            direction = TransferDirectionUi.fromFfi(session.direction),
            totalBytes = session.totalBytes.toLong().coerceAtLeast(0L),
            transferredBytes = session.transferredBytes.toLong().coerceAtLeast(0L),
            state = TransferStateUi.fromFfi(session.state),
            startedAt = session.startedAt,
        )

    fun isSingleFileReceive(session: FfiTransferSession): Boolean =
        session.items.size == 1 && !session.items[0].isDirectory

    fun prepareAndHandOffReceiveFd(
        tracker: PendingMediaStoreTracker<Uri>?,
        fdHandedOffIds: MutableSet<String>,
        node: OrionNodeHandle,
        session: FfiTransferSession,
    ): Boolean {
        val activeTracker = tracker ?: return false
        val item = session.items.firstOrNull() ?: return false
        if (!fdHandedOffIds.add(session.transferId)) {
            return false
        }
        val ok = runCatching {
            activeTracker.prepareAndHandOff(
                transferId = session.transferId,
                fileName = item.name,
                mimeType = item.mimeType,
            ) { rawFd ->
                node.provideReceiveFd(session.transferId, rawFd)
            }
        }.getOrDefault(false)
        if (!ok) {
            fdHandedOffIds.remove(session.transferId)
        }
        return ok
    }

    fun sendQueueItemsViaOrion(
        target: OutboundTarget,
        items: List<SendQueueItemUi>,
    ): Result<TransferUi> = runCatching {
        val singleFile = items.singleOrNull()?.takeIf { it.kind == SendItemKind.FILE }
        val session = if (singleFile != null) {
            sendSingleFileFd(target, Uri.parse(singleFile.uriString))
        } else {
            stageAndSendQueueItems(target, items)
        }
        notifyOutboundStarted(target, session)
    }

    fun sendContentUrisViaOrion(
        target: OutboundTarget,
        uris: List<String>,
        sharedText: String?,
    ): Result<TransferUi> = runCatching {
        val singleUri = uris.singleOrNull()?.takeIf { sharedText.isNullOrBlank() }?.let(Uri::parse)
        val session = if (singleUri != null && !SafFolderHelper.isTreeDocumentUri(singleUri)) {
            sendSingleFileFd(target, singleUri)
        } else {
            stageAndSendContentUris(target, uris, sharedText)
        }
        notifyOutboundStarted(target, session)
    }

    private fun sendSingleFileFd(target: OutboundTarget, uri: Uri): FfiTransferSession {
        val (fileName, sizeHint, mime) = queryContentUriMetadata(target.context, uri)
        val pfd = target.context.contentResolver.openFileDescriptor(uri, "r")
            ?: error("Unable to open shared file descriptor")
        val sizeBytes = if (sizeHint > 0L) sizeHint else pfd.statSize.coerceAtLeast(0L)
        val rawFd = pfd.detachFd()
        return target.node.sendFd(
            peerDeviceId = target.peerDeviceId,
            fileName = fileName,
            mimeType = mime,
            sizeBytes = sizeBytes.toULong(),
            rawFd = rawFd,
        )
    }

    private fun stageAndSendQueueItems(
        target: OutboundTarget,
        items: List<SendQueueItemUi>,
    ): FfiTransferSession {
        val stagingDir = File(target.context.cacheDir, "orion_share_${UUID.randomUUID()}").apply { mkdirs() }
        try {
            val stage = StageContext(target.context, stagingDir)
            items.forEach { stage.addQueueItem(it) }
            stage.flushText()
            require(stage.stagedPaths.isNotEmpty()) { "No shareable payload found" }
            return target.node.sendPaths(target.peerDeviceId, stage.stagedPaths)
        } finally {
            stagingDir.deleteRecursively()
        }
    }

    private fun stageAndSendContentUris(
        target: OutboundTarget,
        uris: List<String>,
        sharedText: String?,
    ): FfiTransferSession {
        val stagingDir = File(target.context.cacheDir, "orion_share_${UUID.randomUUID()}").apply { mkdirs() }
        try {
            val stage = StageContext(target.context, stagingDir)
            uris.forEach { raw -> stage.addContentUri(Uri.parse(raw)) }
            if (!sharedText.isNullOrBlank()) {
                stage.textParts.add(sharedText.trim())
            }
            stage.flushText()
            require(stage.stagedPaths.isNotEmpty()) { "No shareable payload found" }
            return target.node.sendPaths(target.peerDeviceId, stage.stagedPaths)
        } finally {
            stagingDir.deleteRecursively()
        }
    }

    private fun notifyOutboundStarted(
        target: OutboundTarget,
        session: FfiTransferSession,
    ): TransferUi {
        val uiTransfer = mapTransferSessionToUi(session, target.peerNames)
        PairingNotifier.showOrUpdateTransferProgress(target.context, uiTransfer)
        KeynearTransferJobService.syncActiveTransfers(target.context, true)
        return uiTransfer
    }

    private class StageContext(
        private val context: Context,
        private val stagingDir: File,
    ) {
        val stagedPaths = mutableListOf<String>()
        val textParts = mutableListOf<String>()

        fun addQueueItem(item: SendQueueItemUi) {
            when (item.kind) {
                SendItemKind.FOLDER -> {
                    val treeUri = Uri.parse(item.uriString)
                    val folderName = item.name.trimEnd('/').ifBlank {
                        SafFolderHelper.resolveTreeDisplayName(context, treeUri)
                    }
                    stageFolderTree(treeUri, folderName)
                }
                SendItemKind.FILE -> addContentUri(Uri.parse(item.uriString))
                SendItemKind.CLIPBOARD, SendItemKind.TEXT -> {
                    val trimmed = item.textPayload.trim()
                    if (trimmed.isNotEmpty()) textParts.add(trimmed)
                }
            }
        }

        fun addContentUri(uri: Uri) {
            if (SafFolderHelper.isTreeDocumentUri(uri)) {
                val name = SafFolderHelper.resolveTreeDisplayName(context, uri)
                stageFolderTree(uri, name)
            } else {
                val (name, _, _) = queryContentUriMetadata(context, uri)
                val dest = SafFolderHelper.uniqueStagingTarget(stagingDir, name, isDir = false)
                context.contentResolver.openInputStream(uri)?.use { input ->
                    FileOutputStream(dest).use { output ->
                        input.copyTo(output, bufferSize = STREAM_COPY_BUFFER_BYTES)
                    }
                } ?: error("Unable to read shared item $name")
                stagedPaths.add(dest.absolutePath)
            }
        }

        fun flushText() {
            val combined = textParts.joinToString(separator = "\n\n").trim()
            if (combined.isEmpty()) return
            val textFile = SafFolderHelper.uniqueStagingTarget(stagingDir, "shared-text.txt", isDir = false)
            textFile.writeText(combined, Charsets.UTF_8)
            stagedPaths.add(textFile.absolutePath)
        }

        private fun stageFolderTree(treeUri: Uri, folderName: String) {
            val destDir = SafFolderHelper.uniqueStagingTarget(stagingDir, folderName, isDir = true)
            SafFolderHelper.stageFolderTreeToDirectory(context, treeUri, destDir)
            stagedPaths.add(destDir.absolutePath)
        }
    }
}
