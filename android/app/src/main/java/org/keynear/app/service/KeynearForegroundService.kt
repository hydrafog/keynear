package org.keynear.app.service

import android.app.Service
import android.content.Intent
import android.os.IBinder

class KeynearForegroundService : Service() {
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        // TODO: Promote to connectedDevice foreground notification and start UniFFI core runtime.
        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null
}

