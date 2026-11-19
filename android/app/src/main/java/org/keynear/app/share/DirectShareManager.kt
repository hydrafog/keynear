package org.keynear.app.share

import android.content.Context
import android.content.Intent
import android.content.pm.ShortcutInfo
import android.content.pm.ShortcutManager
import android.graphics.drawable.Icon
import org.keynear.app.ui.model.PairedDeviceUi

data class DirectShareTargetSpec(
    val shortcutId: String,
    val deviceId: String,
    val shortLabel: String,
    val longLabel: String,
    val rank: Int,
)

object DirectShareManager {
    const val SHARE_TARGET_CATEGORY = "org.keynear.app.category.SHARE_TARGET"
    const val SHORTCUT_ID_PREFIX = "keynear_peer_"
    const val EXTRA_TARGET_DEVICE_ID = "org.keynear.app.extra.TARGET_DEVICE_ID"
    private const val MAX_DIRECT_SHARE_SLOTS = 4
    private const val MAX_SHORT_LABEL_LEN = 25
    private const val MAX_LONG_LABEL_LEN = 60

    fun shortcutIdForDevice(deviceId: String): String = "$SHORTCUT_ID_PREFIX$deviceId"

    fun extractDeviceId(
        shortcutId: String?,
        explicitDeviceId: String? = null,
    ): String? {
        val normalizedShortcut = shortcutId?.trim().orEmpty()
        if (normalizedShortcut.isNotEmpty()) {
            return if (normalizedShortcut.startsWith(SHORTCUT_ID_PREFIX)) {
                normalizedShortcut.removePrefix(SHORTCUT_ID_PREFIX).trim().ifBlank { null }
            } else {
                normalizedShortcut
            }
        }
        return explicitDeviceId?.trim()?.ifBlank { null }
    }

    fun buildShortcutSpecs(
        devices: List<PairedDeviceUi>,
        maxShortcuts: Int = MAX_DIRECT_SHARE_SLOTS,
    ): List<DirectShareTargetSpec> {
        val limit = maxShortcuts.coerceAtLeast(0)
        if (limit == 0 || devices.isEmpty()) {
            return emptyList()
        }
        val seenIds = HashSet<String>(devices.size)
        val specs = ArrayList<DirectShareTargetSpec>(minOf(devices.size, limit))
        for (device in devices) {
            val id = device.deviceId.trim()
            if (id.isEmpty() || !seenIds.add(id)) {
                continue
            }
            val label = device.displayName.trim().ifBlank { id }
            specs.add(
                DirectShareTargetSpec(
                    shortcutId = shortcutIdForDevice(id),
                    deviceId = id,
                    shortLabel = label.take(MAX_SHORT_LABEL_LEN),
                    longLabel = "Send to $label".take(MAX_LONG_LABEL_LEN),
                    rank = specs.size,
                ),
            )
            if (specs.size >= limit) {
                break
            }
        }
        return specs
    }

    fun diffObsoleteShortcutIds(
        existingShortcutIds: Set<String>,
        desiredSpecs: List<DirectShareTargetSpec>,
    ): List<String> {
        val desiredIds = desiredSpecs.mapTo(HashSet(desiredSpecs.size)) { it.shortcutId }
        return existingShortcutIds
            .asSequence()
            .filter { it.startsWith(SHORTCUT_ID_PREFIX) && it !in desiredIds }
            .sorted()
            .toList()
    }

    fun syncShortcuts(
        context: Context,
        devices: List<PairedDeviceUi>,
    ) {
        runCatching {
            val app = context.applicationContext
            val manager = app.getSystemService(ShortcutManager::class.java) ?: return
            val available = (manager.maxShortcutCountPerActivity - manager.manifestShortcuts.size)
                .coerceIn(1, MAX_DIRECT_SHARE_SLOTS)
            val specs = buildShortcutSpecs(devices, available)
            val existingIds = manager.dynamicShortcuts.mapTo(HashSet()) { it.id }
            val longLivedIds = manager.getShortcuts(
                ShortcutManager.FLAG_MATCH_DYNAMIC or ShortcutManager.FLAG_MATCH_CACHED,
            ).mapTo(HashSet()) { it.id }
            val obsolete = diffObsoleteShortcutIds(existingIds + longLivedIds, specs)
            if (obsolete.isNotEmpty()) {
                manager.removeDynamicShortcuts(obsolete)
                manager.removeLongLivedShortcuts(obsolete)
            }
            val icon = Icon.createWithResource(app, android.R.drawable.stat_sys_data_bluetooth)
            val categories = setOf(SHARE_TARGET_CATEGORY)
            for (spec in specs) {
                val targetIntent = Intent(app, ShareActivity::class.java).apply {
                    action = Intent.ACTION_SEND
                    putExtra(EXTRA_TARGET_DEVICE_ID, spec.deviceId)
                }
                val info = ShortcutInfo.Builder(app, spec.shortcutId)
                    .setShortLabel(spec.shortLabel)
                    .setLongLabel(spec.longLabel)
                    .setRank(spec.rank)
                    .setLongLived(true)
                    .setCategories(categories)
                    .setIcon(icon)
                    .setIntent(targetIntent)
                    .build()
                manager.pushDynamicShortcut(info)
            }
        }
    }
}

