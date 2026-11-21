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

interface CoreBridge {
    fun loadState(): CoreState
    fun cycleVisibility(): CoreState
    fun setVisibility(mode: VisibilityState): CoreState = loadState()
    fun tick(): CoreState = loadState()
    fun updateNearbyPeers(peers: List<DiscoveredPeerUi>) {}
    fun updateBluetoothStatus(status: BluetoothStatus, warning: String? = null) {}
    fun updateLocalNetworkStatus(status: LocalNetworkStatus, message: String? = null) {}
    fun currentLocalNetworkStatus(): LocalNetworkStatus = LocalNetworkStatus.READY
    fun setPendingPairing(prompt: PendingPairingUi?) {}
    fun unpairDevice(deviceId: String): CoreState = loadState()
    fun renamePairedDevice(deviceId: String, newDisplayName: String): CoreState = loadState()
    fun setAutoAcceptFiles(deviceId: String, enabled: Boolean): CoreState = loadState()
    fun resetBleBond(deviceId: String): CoreState = loadState()
    fun acceptTransfer(transferId: String): CoreState = loadState()
    fun declineTransfer(transferId: String): CoreState = loadState()
    fun cancelTransfer(transferId: String): CoreState = loadState()
    fun stageFileUris(uris: List<String>): CoreState = loadState()
    fun stageFolderUri(
        folderUri: String,
        displayName: String,
        fileUris: List<String> = emptyList(),
    ): CoreState = loadState()
    fun stageClipboardText(text: String): CoreState = loadState()
    fun stageTextSnippet(text: String): CoreState = loadState()
    fun removeSendQueueItem(itemId: String): CoreState = loadState()
    fun clearSendQueue(): CoreState = loadState()
    fun sendQueueToPeer(peerDeviceId: String): Result<TransferUi> =
        Result.failure(UnsupportedOperationException("send unavailable"))
    fun updateDaemonConfig(transform: (DaemonConfigUi) -> DaemonConfigUi): CoreState = loadState()
    fun sendPhoneClipboardText(text: String): Result<Unit> =
        Result.failure(UnsupportedOperationException("clipboard send unavailable"))
    fun sendPhoneClipboardBlob(mimeType: String, data: ByteArray): Result<Unit> =
        Result.failure(UnsupportedOperationException("clipboard blob unavailable"))
    fun sendVerifiedOtp(code: String): Result<Unit> =
        Result.failure(UnsupportedOperationException("otp send unavailable"))
    fun sendContentUris(
        peerDeviceId: String,
        uris: List<String>,
        sharedText: String?,
    ): Result<TransferUi> = Result.failure(UnsupportedOperationException("send unavailable"))
}
