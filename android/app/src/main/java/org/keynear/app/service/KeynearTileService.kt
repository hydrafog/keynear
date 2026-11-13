package org.keynear.app.service

import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import android.util.Log
import org.keynear.app.R
import org.keynear.app.ble.KeynearBleManager
import org.keynear.app.core.CoreClientProvider
import org.keynear.app.core.CoreState
import org.keynear.app.core.VisibilityState

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
        when (val result = CoreClientProvider.init(applicationContext).cycleVisibility()) {
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
                Log.w(TAG, "visibility_cycle_failed")
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
