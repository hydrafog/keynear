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

class VisibilityCountdownTracker(
    private val timeoutSecs: Long = 600L,
) {
    private var mode: VisibilityState = VisibilityState.PAIRED_ONLY
    private var remainingSecs: Long = 0L

    @Synchronized
    fun currentMode(): VisibilityState = mode

    @Synchronized
    fun remainingSecs(): Long = remainingSecs

    @Synchronized
    fun setMode(next: VisibilityState) {
        mode = next
        remainingSecs = if (next == VisibilityState.EVERYONE_NEARBY) timeoutSecs else 0L
    }

    @Synchronized
    fun cycle(): VisibilityState {
        setMode(mode.next())
        return mode
    }

    @Synchronized
    fun tick(): VisibilityState {
        if (mode == VisibilityState.EVERYONE_NEARBY) {
            if (remainingSecs <= 1L) {
                mode = VisibilityState.PAIRED_ONLY
                remainingSecs = 0L
            } else {
                remainingSecs -= 1L
            }
        }
        return mode
    }
}
