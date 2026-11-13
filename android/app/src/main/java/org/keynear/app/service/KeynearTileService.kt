package org.keynear.app.service

import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import android.util.Log
import org.keynear.app.R
import org.keynear.app.core.CoreClientProvider
import org.keynear.app.core.CoreState
import org.keynear.app.core.VisibilityState

class KeynearTileService : TileService() {
    override fun onStartListening() {
        super.onStartListening()
        val state = CoreClientProvider.get().loadState()
        val visibility = when (state) {
            is CoreState.Empty -> state.snapshot.visibility
            is CoreState.Ready -> state.snapshot.visibility
            else -> null
        }
        if (visibility != null) {
            applyVisibilityToTile(visibility)
        }
    }

    override fun onClick() {
        super.onClick()
        when (val result = CoreClientProvider.get().cycleVisibility()) {
            is CoreState.Empty -> applyVisibilityToTile(result.snapshot.visibility)
            is CoreState.Ready -> applyVisibilityToTile(result.snapshot.visibility)
            is CoreState.Error -> {
                Log.w(TAG, "visibility_cycle_failed")
            }
            CoreState.Loading -> Unit
        }
    }

    private fun applyVisibilityToTile(visibility: VisibilityState) {
        val tile = qsTile ?: return
        tile.label = getString(R.string.tile_label)
        tile.subtitle = visibility.label()
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
