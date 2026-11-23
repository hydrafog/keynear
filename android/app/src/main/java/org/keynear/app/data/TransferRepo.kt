package org.keynear.app.data

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import org.keynear.app.ble.PairingNotifier
import org.keynear.app.bridge.PendingMediaStoreTracker
import org.keynear.app.bridge.formatByteCount
import org.keynear.app.service.KeynearTransferJobService
import org.keynear.app.ui.model.TransferDirectionUi
import org.keynear.app.ui.model.TransferStateUi
import org.keynear.app.ui.model.TransferUi
import uniffi.keynear_core.FfiPairedDevice
import uniffi.keynear_core.FfiTransferDirection
import uniffi.keynear_core.FfiTransferSession
import uniffi.keynear_core.OrionNodeHandle
import java.io.File
import java.io.FileOutputStream
import java.util.UUID
import org.keynear.app.ble.showOrUpdateTransferProgress
import org.keynear.app.ble.showIncomingTransferConsent
import org.keynear.app.ble.showTransferCompleted
import org.keynear.app.ble.showTransferError

object TransferRepo {
    private const val STREAM_COPY_BUFFER_BYTES = 65_536

    fun queryContentUriMetadata(context: Context, uri: Uri): Triple<String, Long, String> {
        var name = uri.lastPathSegment?.substringAfterLast('/')?.ifBlank { null } ?: "shared-file.bin"
        var size = 0L
        runCatching {
            context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE), null, null, null)
                ?.use { cursor ->
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

    fun sendContentUrisViaOrion(
        context: Context,
        node: OrionNodeHandle,
        peerDeviceId: String,
        uris: List<String>,
        sharedText: String?,
        peerNames: Map<String, String>,
    ): Result<TransferUi> = runCatching {
        val session = if (uris.size == 1 && sharedText.isNullOrBlank()) {
            val uri = Uri.parse(uris.first())
            val (fileName, sizeHint, mime) = queryContentUriMetadata(context, uri)
            val pfd = context.contentResolver.openFileDescriptor(uri, "r")
                ?: error("Unable to open shared file descriptor")
            val sizeBytes = if (sizeHint > 0L) sizeHint else pfd.statSize.coerceAtLeast(0L)
            val rawFd = pfd.detachFd()
            node.sendFd(
                peerDeviceId = peerDeviceId,
                fileName = fileName,
                mimeType = mime,
                sizeBytes = sizeBytes.toULong(),
                rawFd = rawFd,
            )
        } else {
            val stagingDir = File(context.cacheDir, "orion_share_${UUID.randomUUID()}").apply { mkdirs() }
            try {
                val stagedPaths = mutableListOf<String>()
                uris.forEachIndexed { idx, raw ->
                    val uri = Uri.parse(raw)
                    val (name, _, _) = queryContentUriMetadata(context, uri)
                    val safeName = "${idx}_${name.replace('/', '_')}"
                    val dest = File(stagingDir, safeName)
                    context.contentResolver.openInputStream(uri)?.use { input ->
                        FileOutputStream(dest).use { output ->
                            input.copyTo(output, bufferSize = STREAM_COPY_BUFFER_BYTES)
                        }
                    } ?: error("Unable to read shared item $name")
                    stagedPaths.add(dest.absolutePath)
                }
                val trimmedText = sharedText?.trim()?.takeIf { it.isNotEmpty() }
                if (trimmedText != null) {
                    val textFile = File(stagingDir, "shared-text.txt")
                    textFile.writeText(trimmedText, Charsets.UTF_8)
                    stagedPaths.add(textFile.absolutePath)
                }
                require(stagedPaths.isNotEmpty()) { "No shareable payload found" }
                node.sendPaths(peerDeviceId, stagedPaths)
            } finally {
                stagingDir.deleteRecursively()
            }
        }
        val uiTransfer = mapTransferSessionToUi(session, peerNames)
        PairingNotifier.showOrUpdateTransferProgress(context, uiTransfer)
        KeynearTransferJobService.syncActiveTransfers(context, true)
        uiTransfer
    }

    fun syncTransferStates(
        app: Context,
        node: OrionNodeHandle,
        pairedDevices: List<FfiPairedDevice>,
        tracker: PendingMediaStoreTracker<Uri>?,
        fdHandedOffIds: MutableSet<String>,
        notifiedConsentIds: MutableSet<String>,
        notifiedTerminalIds: MutableSet<String>,
    ): Boolean {
        val sessions = runCatching { node.listTransfers() }.getOrNull() ?: return false
        val peerNames = pairedDevices.associate { it.deviceId to it.displayName }
        val ctx = SyncContext(
            app = app,
            node = node,
            pairedDevices = pairedDevices,
            tracker = tracker,
            fdHandedOffIds = fdHandedOffIds,
            notifiedConsentIds = notifiedConsentIds,
            notifiedTerminalIds = notifiedTerminalIds,
            peerNames = peerNames,
        )
        var changed = false
        var hasActiveTransfers = false

        for (session in sessions) {
            val uiState = TransferStateUi.fromFfi(session.state)
            if (ctx.handlePendingConsent(session, uiState)) {
                changed = true
            }
            if (uiState == TransferStateUi.ACTIVE) {
                hasActiveTransfers = true
                if (ctx.handleActiveTransfer(session)) {
                    changed = true
                }
            }
            val completedUri = if (uiState == TransferStateUi.COMPLETED) {
                tracker?.getPending(session.transferId)
            } else {
                null
            }
            tracker?.reconcileState(session.transferId, uiState)
            if (ctx.handleTerminalState(session, uiState, completedUri)) {
                changed = true
            }
        }
        KeynearTransferJobService.syncActiveTransfers(app, hasActiveTransfers)
        return changed
    }

    private fun SyncContext.handlePendingConsent(
        session: FfiTransferSession,
        uiState: TransferStateUi,
    ): Boolean {
        if (session.direction != FfiTransferDirection.RECEIVE || uiState != TransferStateUi.PENDING_CONSENT) {
            return false
        }
        if (!notifiedConsentIds.add(session.transferId)) return false
        PairingNotifier.showIncomingTransferConsent(
            context = app,
            transferId = session.transferId,
            peerName = peerNames[session.peerDeviceId] ?: session.peerDeviceId,
            summary = summarizeTransferItems(session),
            sizeLabel = formatByteCount(session.totalBytes.toLong()),
        )
        return true
    }

    private fun SyncContext.handleActiveTransfer(session: FfiTransferSession): Boolean {
        var changed = false
        if (session.direction == FfiTransferDirection.RECEIVE &&
            isSingleFileReceive(session) &&
            !fdHandedOffIds.contains(session.transferId)
        ) {
            if (prepareAndHandOffReceiveFd(tracker, fdHandedOffIds, node, session)) {
                changed = true
            }
        }
        PairingNotifier.showOrUpdateTransferProgress(
            context = app,
            transfer = mapTransferSessionToUi(session, peerNames),
        )
        return changed
    }

    private fun SyncContext.handleTerminalState(
        session: FfiTransferSession,
        uiState: TransferStateUi,
        completedUri: Uri? = null,
    ): Boolean {
        if (!uiState.isTerminal()) return false
        PairingNotifier.cancelTransferNotification(app, session.transferId)
        if (!notifiedTerminalIds.add(session.transferId)) return false
        if (session.direction == FfiTransferDirection.RECEIVE && uiState == TransferStateUi.COMPLETED) {
            val autoAccepted = pairedDevices
                .firstOrNull { it.deviceId == session.peerDeviceId }
                ?.autoAcceptFiles == true
            PairingNotifier.showTransferCompleted(
                context = app,
                transfer = mapTransferSessionToUi(session, peerNames),
                autoAccepted = autoAccepted,
                completedUri = completedUri,
            )
        } else if (uiState == TransferStateUi.FAILED) {
            PairingNotifier.showTransferError(
                context = app,
                summary = summarizeTransferItems(session),
                reason = "Connection interrupted or rejected",
            )
        }
        return true
    }

    private data class SyncContext(
        val app: Context,
        val node: OrionNodeHandle,
        val pairedDevices: List<FfiPairedDevice>,
        val tracker: PendingMediaStoreTracker<Uri>?,
        val fdHandedOffIds: MutableSet<String>,
        val notifiedConsentIds: MutableSet<String>,
        val notifiedTerminalIds: MutableSet<String>,
        val peerNames: Map<String, String>,
    )
}

