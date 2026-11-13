package org.keynear.app.share

import android.app.Activity
import android.os.Bundle

class ShareActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // TODO: Resolve ACTION_SEND / ACTION_SEND_MULTIPLE URIs and dispatch transfer via core.
        finish()
    }
}

