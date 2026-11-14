package org.keynear.app.service

import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import android.util.Log
import org.keynear.app.R
import org.keynear.app.ble.KeynearBleManager
import org.keynear.app.core.CoreClientProvider
import org.keynear.app.core.CoreState
import org.keynear.app.core.VisibilityState

enum class TileTransitionDecision {
    APPLY_DIRECTLY,
    REQUIRE_UNLOCK,
}

fun evaluateTileTransition(
    current: VisibilityState,
    isDeviceLocked: Boolean,
): Pair<VisibilityState, TileTransitionDecision> = when (current) {
    VisibilityState.HIDDEN -> VisibilityState.PAIRED_ONLY to TileTransitionDecision.APPLY_DIRECTLY
    VisibilityState.PAIRED_ONLY -> {
        val decision = if (isDeviceLocked) {
            TileTransitionDecision.REQUIRE_UNLOCK
        } else {
            TileTransitionDecision.APPLY_DIRECTLY
        }
        VisibilityState.EVERYONE_NEARBY to decision
    }
    VisibilityState.EVERYONE_NEARBY -> {
        val nextMode = if (isDeviceLocked) {
            VisibilityState.PAIRED_ONLY
        } else {
            VisibilityState.HIDDEN
        }
        nextMode to TileTransitionDecision.APPLY_DIRECTLY
    }
}

class KeynearTileService : TileService() {
    override fun onStartListening() {
        super.onStartListening()
        val state = CoreClientProvider.init(applicationContext).loadState()
        val snapshot = when (state) {
            is CoreState.Empty -> state.snapshot
            is CoreState.Ready -> state.snapshot
            else -> null
        }
        if (snapshot != null) {
            applyVisibilityToTile(snapshot.visibility, snapshot.everyoneRemainingSecs)
        }
    }

    override fun onClick() {
        super.onClick()
        val client = CoreClientProvider.init(applicationContext)
        val currentVisibility = when (val currentState = client.loadState()) {
            is CoreState.Empty -> currentState.snapshot.visibility
            is CoreState.Ready -> currentState.snapshot.visibility
            is CoreState.Error -> currentState.previous?.visibility ?: VisibilityState.PAIRED_ONLY
            CoreState.Loading -> VisibilityState.PAIRED_ONLY
        }
        val (targetVisibility, decision) = evaluateTileTransition(currentVisibility, isLocked)
        if (decision == TileTransitionDecision.REQUIRE_UNLOCK) {
            unlockAndRun {
                applyVisibilityTarget(targetVisibility)
            }
        } else {
            applyVisibilityTarget(targetVisibility)
        }
    }

    private fun applyVisibilityTarget(target: VisibilityState) {
        when (val result = CoreClientProvider.init(applicationContext).setVisibility(target)) {
            is CoreState.Empty -> {
                applyVisibilityToTile(result.snapshot.visibility, result.snapshot.everyoneRemainingSecs)
                KeynearBleManager.startOrSync(applicationContext, result.snapshot.visibility)
                KeynearForegroundService.syncIfPermitted(applicationContext)
            }
            is CoreState.Ready -> {
                applyVisibilityToTile(result.snapshot.visibility, result.snapshot.everyoneRemainingSecs)
                KeynearBleManager.startOrSync(applicationContext, result.snapshot.visibility)
                KeynearForegroundService.syncIfPermitted(applicationContext)
            }
            is CoreState.Error -> {
                Log.w(TAG, "visibility_set_failed")
            }
            CoreState.Loading -> Unit
        }
    }

    private fun applyVisibilityToTile(visibility: VisibilityState, remainingSecs: Long) {
        val tile = qsTile ?: return
        tile.label = getString(R.string.tile_label)
        tile.subtitle = visibility.formattedStatus(remainingSecs)
        tile.state = if (visibility == VisibilityState.HIDDEN) {
            Tile.STATE_INACTIVE
        } else {
            Tile.STATE_ACTIVE
        }
        tile.updateTile()
    }

    companion object {
        private const val TAG = "KeynearTileService"
    }
}
