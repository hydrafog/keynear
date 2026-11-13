package org.keynear.app.service

import android.service.quicksettings.Tile
import android.service.quicksettings.TileService

class KeynearTileService : TileService() {
    override fun onClick() {
        super.onClick()
        // TODO: Cycle visibility mode (Hidden -> PairedOnly -> EveryoneNearby) with unlock gate.
        qsTile?.apply {
            state = if (state == Tile.STATE_ACTIVE) Tile.STATE_INACTIVE else Tile.STATE_ACTIVE
            updateTile()
        }
    }
}

