package org.keynear.app.data

import android.content.Context
import android.net.Uri
import org.keynear.app.ble.PairingNotifier
import org.keynear.app.ble.showIncomingTransferConsent
import org.keynear.app.ble.showOrUpdateTransferProgress
import org.keynear.app.ble.showTransferCompleted
import org.keynear.app.ble.showTransferError
import org.keynear.app.bridge.PendingMediaStoreTracker
import org.keynear.app.bridge.formatByteCount
import org.keynear.app.service.KeynearTransferJobService
import org.keynear.app.ui.model.TransferStateUi
import uniffi.keynear_core.FfiPairedDevice
import uniffi.keynear_core.FfiTransferDirection
import uniffi.keynear_core.FfiTransferSession
import uniffi.keynear_core.OrionNodeHandle
import java.io.File

data class TransferSyncEnv(
    val app: Context,
    val node: OrionNodeHandle,
    val pairedDevices: List<FfiPairedDevice>,
    val incomingDir: File,
)

data class TransferSyncIds(
    val fdHandedOffIds: MutableSet<String>,
    val exportedArchiveIds: MutableSet<String>,
    val notifiedConsentIds: MutableSet<String>,
    val notifiedTerminalIds: MutableSet<String>,
)

data class TransferSyncInput(
    val env: TransferSyncEnv,
    val tracker: PendingMediaStoreTracker<Uri>?,
    val ids: TransferSyncIds,
)

object TransferSyncHandler {
    fun syncTransferStates(input: TransferSyncInput): Boolean {
        val sessions = runCatching { input.env.node.listTransfers() }.getOrNull() ?: return false
        val peerNames = input.env.pairedDevices.associate { it.deviceId to it.displayName }
        val ctx = SyncContext(input = input, peerNames = peerNames)
        var changed = false
        var hasActiveTransfers = false

        for (session in sessions) {
            val uiState = TransferStateUi.fromFfi(session.state)
            if (ctx.handlePendingConsent(session, uiState)) changed = true
            if (uiState == TransferStateUi.ACTIVE) {
                hasActiveTransfers = true
                if (ctx.handleActiveTransfer(session)) changed = true
            }
            val singleUri = if (uiState == TransferStateUi.COMPLETED) {
                input.tracker?.getPending(session.transferId)
            } else {
                null
            }
            input.tracker?.reconcileState(session.transferId, uiState)
            if (ctx.handleTerminalState(session, uiState, singleUri)) changed = true
        }
        KeynearTransferJobService.syncActiveTransfers(input.env.app, hasActiveTransfers)
        return changed
    }

    private fun SyncContext.handlePendingConsent(
        session: FfiTransferSession,
        uiState: TransferStateUi,
    ): Boolean {
        if (session.direction != FfiTransferDirection.RECEIVE || uiState != TransferStateUi.PENDING_CONSENT) {
            return false
        }
        if (!input.ids.notifiedConsentIds.add(session.transferId)) return false
        PairingNotifier.showIncomingTransferConsent(
            context = input.env.app,
            transferId = session.transferId,
            peerName = peerNames[session.peerDeviceId] ?: session.peerDeviceId,
            summary = TransferRepo.summarizeTransferItems(session),
            sizeLabel = formatByteCount(session.totalBytes.toLong()),
        )
        return true
    }

    private fun SyncContext.handleActiveTransfer(session: FfiTransferSession): Boolean {
        var changed = false
        if (session.direction == FfiTransferDirection.RECEIVE &&
            TransferRepo.isSingleFileReceive(session) &&
            !input.ids.fdHandedOffIds.contains(session.transferId)
        ) {
            val handed = TransferRepo.prepareAndHandOffReceiveFd(
                tracker = input.tracker,
                fdHandedOffIds = input.ids.fdHandedOffIds,
                node = input.env.node,
                session = session,
            )
            if (handed) changed = true
        }
        PairingNotifier.showOrUpdateTransferProgress(
            context = input.env.app,
            transfer = TransferRepo.mapTransferSessionToUi(session, peerNames),
        )
        return changed
    }

    private fun SyncContext.handleTerminalState(
        session: FfiTransferSession,
        uiState: TransferStateUi,
        singleFileUri: Uri?,
    ): Boolean {
        if (!uiState.isTerminal()) return false
        PairingNotifier.cancelTransferNotification(input.env.app, session.transferId)
        if (!input.ids.notifiedTerminalIds.add(session.transferId)) return false
        if (session.direction == FfiTransferDirection.RECEIVE && uiState == TransferStateUi.COMPLETED) {
            handleCompletedReceive(session, singleFileUri)
        } else if (uiState == TransferStateUi.FAILED) {
            PairingNotifier.showTransferError(
                context = input.env.app,
                summary = TransferRepo.summarizeTransferItems(session),
                reason = "Connection interrupted or rejected",
            )
        }
        return true
    }

    private fun SyncContext.handleCompletedReceive(
        session: FfiTransferSession,
        singleFileUri: Uri?,
    ) {
        val autoAccepted = input.env.pairedDevices
            .firstOrNull { it.deviceId == session.peerDeviceId }
            ?.autoAcceptFiles == true
        val completedUri = if (TransferRepo.isSingleFileReceive(session)) {
            singleFileUri
        } else {
            exportArchiveOnce(session).getOrElse { err ->
                PairingNotifier.showTransferError(
                    context = input.env.app,
                    summary = TransferRepo.summarizeTransferItems(session),
                    reason = err.message ?: "Unable to save received folder",
                )
                return
            }
        }
        PairingNotifier.showTransferCompleted(
            context = input.env.app,
            transfer = TransferRepo.mapTransferSessionToUi(session, peerNames),
            autoAccepted = autoAccepted,
            completedUri = completedUri,
        )
    }

    private fun SyncContext.exportArchiveOnce(session: FfiTransferSession): Result<Uri?> {
        if (!input.ids.exportedArchiveIds.add(session.transferId)) {
            return Result.success(null)
        }
        return AndroidFolderExporter.exportReceivedArchive(
            context = input.env.app,
            session = session,
            incomingRoot = input.env.incomingDir,
        )
    }

    private data class SyncContext(
        val input: TransferSyncInput,
        val peerNames: Map<String, String>,
    )
}

