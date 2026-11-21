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

object CoreClientProvider {
    @Volatile
    private var instance: EventQueue? = null

    fun init(context: Context): EventQueue {
        val client = getInternal()
        client.attachContext(context.applicationContext)
        return client
    }

    fun get(): CoreBridge = getInternal()

    fun getClient(): EventQueue = getInternal()

    private fun getInternal(): EventQueue {
        val current = instance
        if (current != null) return current
        return synchronized(this) {
            instance ?: EventQueue().also { instance = it }
        }
    }
}
