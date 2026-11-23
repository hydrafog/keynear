package org.keynear.app.ui.navigation

import android.content.Context
import android.content.Intent
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.CoreState

fun launchFirstResolvableActivity(
    context: Context,
    candidates: List<Intent>,
): Boolean {
    for (candidate in candidates) {
        val intent = Intent(candidate).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        if (intent.resolveActivity(context.packageManager) != null) {
            runCatching {
                context.startActivity(intent)
                return true
            }
        }
    }
    return false
}

fun snapshotFromState(state: CoreState): CoreSnapshot? = when (state) {
    is CoreState.Empty -> state.snapshot
    is CoreState.Ready -> state.snapshot
    is CoreState.Error -> state.previous
    CoreState.Loading -> null
}
