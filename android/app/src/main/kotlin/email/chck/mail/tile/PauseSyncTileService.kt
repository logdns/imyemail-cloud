package email.imy.cloud.tile

import email.imy.cloud.data.L10n

import android.os.Build
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import androidx.work.WorkManager

class PauseSyncTileService : TileService() {
    override fun onStartListening() {
        qsTile?.apply {
            label = L10n.t("稍后同步")
            state = Tile.STATE_INACTIVE
            updateTile()
        }
    }

    override fun onClick() {
        WorkManager.getInstance(this).cancelUniqueWork("chck-sync")
        qsTile?.apply {
            state = Tile.STATE_ACTIVE
            if (Build.VERSION.SDK_INT >= 29) subtitle = L10n.t("1 小时后再同步")
            updateTile()
        }
    }
}
