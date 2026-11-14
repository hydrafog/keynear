package org.keynear.app.core

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import uniffi.keynear_core.FfiTransferSession
import org.keynear.app.ble.PairingNotificationManager
import org.keynear.app.service.KeynearTransferJobService
import uniffi.keynear_core.CoreHandle
import uniffi.keynear_core.FfiLanEndpoint
import uniffi.keynear_core.FfiPairedDevice
import uniffi.keynear_core.OrionNodeHandle
import java.io.FileOutputStream
import java.io.File
import java.util.UUID

internal fun queryContentUriMetadata(context: Context, uri: Uri): Triple<String, Long, String> {
        var name = uri.lastPathSegment?.substringAfterLast('/')?.ifBlank { null } ?: "shared-file.bin"
        var size = 0L
        runCatching {
            context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE), null, null, null)
                ?.use { cursor ->
                    if (cursor.moveToFirst()) {
                        val nameIdx = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                        if (nameIdx >= 0) {
                            cursor.getString(nameIdx)?.takeIf { it.isNotBlank() }?.let { name = it }
                        }
                        val sizeIdx = cursor.getColumnIndex(OpenableColumns.SIZE)
                        if (sizeIdx >= 0 && !cursor.isNull(sizeIdx)) {
                            size = cursor.getLong(sizeIdx)
                        }
                    }
                }
        }
        val mime = context.contentResolver.getType(uri)?.ifBlank { null } ?: "application/octet-stream"
        return Triple(name, size, mime)
    }


internal fun mapTransferSessionToUi(session: FfiTransferSession, peerNames: Map<String, String>): TransferUi =
        TransferUi(
            transferId = session.transferId,
            peerDeviceId = session.peerDeviceId,
            peerDisplayName = peerNames[session.peerDeviceId] ?: session.peerDeviceId,
            summaryName = summarizeTransferItems(session),
            mimeType = session.items.firstOrNull()?.mimeType ?: "application/octet-stream",
            isDirectory = session.items.any { it.isDirectory },
            direction = TransferDirectionUi.fromFfi(session.direction),
            totalBytes = session.totalBytes.toLong().coerceAtLeast(0L),
            transferredBytes = session.transferredBytes.toLong().coerceAtLeast(0L),
            state = TransferStateUi.fromFfi(session.state),
            startedAt = session.startedAt,
        )

internal fun summarizeTransferItems(session: FfiTransferSession): String {
        val items = session.items
        if (items.isEmpty()) return "Transfer"
        if (items.size == 1) return items[0].name
        return "${items[0].name} (+${items.size - 1} more)"
    }

internal fun resolveStableLocalDeviceId(context: Context): String {
        val prefs = context.getSharedPreferences("keynear_device_identity", Context.MODE_PRIVATE)
        val existing = prefs.getString("device_id", null)
        if (!existing.isNullOrBlank()) {
            return existing
        }
        val generated = "android-${UUID.randomUUID().toString().take(12)}"
        prefs.edit().putString("device_id", generated).apply()
        return generated
    }


internal fun sendContentUrisViaOrion(
    context: Context,
    node: OrionNodeHandle,
    peerDeviceId: String,
    uris: List<String>,
    sharedText: String?,
    peerNames: Map<String, String>,
): Result<TransferUi> {
    val ctx = context
        return runCatching {
            val session = if (uris.size == 1 && sharedText.isNullOrBlank()) {
                val uri = Uri.parse(uris.first())
                val (fileName, sizeHint, mime) = queryContentUriMetadata(ctx, uri)
                val pfd = ctx.contentResolver.openFileDescriptor(uri, "r")
                    ?: error("Unable to open shared file descriptor")
                val sizeBytes = if (sizeHint > 0L) sizeHint else pfd.statSize.coerceAtLeast(0L)
                val rawFd = pfd.detachFd()
                node.sendFd(
                    peerDeviceId = peerDeviceId,
                    fileName = fileName,
                    mimeType = mime,
                    sizeBytes = sizeBytes.toULong(),
                    rawFd = rawFd,
                )
            } else {
                val stagingDir = File(ctx.cacheDir, "orion_share_${UUID.randomUUID()}").apply { mkdirs() }
                try {
                    val stagedPaths = mutableListOf<String>()
                    uris.forEachIndexed { idx, raw ->
                        val uri = Uri.parse(raw)
                        val (name, _, _) = queryContentUriMetadata(ctx, uri)
                        val safeName = "${idx}_${name.replace('/', '_')}"
                        val dest = File(stagingDir, safeName)
                        ctx.contentResolver.openInputStream(uri)?.use { input ->
                            FileOutputStream(dest).use { output ->
                                input.copyTo(output, bufferSize = 65_536)
                            }
                        } ?: error("Unable to read shared item $name")
                        stagedPaths.add(dest.absolutePath)
                    }
                    val trimmedText = sharedText?.trim()?.takeIf { it.isNotEmpty() }
                    if (trimmedText != null) {
                        val textFile = File(stagingDir, "shared-text.txt")
                        textFile.writeText(trimmedText, Charsets.UTF_8)
                        stagedPaths.add(textFile.absolutePath)
                    }
                    require(stagedPaths.isNotEmpty()) { "No shareable payload found" }
                    node.sendPaths(peerDeviceId, stagedPaths)
                } finally {
                    stagingDir.deleteRecursively()
                }
            }
            val uiTransfer = mapTransferSessionToUi(session, peerNames)
            PairingNotificationManager.showOrUpdateTransferProgress(ctx, uiTransfer)
            KeynearTransferJobService.syncActiveTransfers(ctx, true)
            uiTransfer
        }
    }

internal fun decodeEncryptedLanEndpointFromBle(
    handle: CoreHandle,
    store: KeystoreSecretStore,
    devices: List<FfiPairedDevice>,
    encoded: ByteArray,
    bleAddress: String?,
): FfiLanEndpoint? {
    if (encoded.isEmpty() || encoded.size > 65_536) return null
    val ordered = if (!bleAddress.isNullOrBlank()) {
        val exact = devices.filter { it.lastBleAddress.equals(bleAddress, ignoreCase = true) }
        val rest = devices.filterNot { it.lastBleAddress.equals(bleAddress, ignoreCase = true) }
        exact + rest
    } else {
        devices
    }
    for (dev in ordered) {
        val secrets = store.getPeerSecrets(dev.deviceId) ?: continue
        val decoded = try {
            handle.decodeLanEndpoint(encoded, dev.deviceId, secrets.reconnectSecret)
        } catch (_: Throwable) {
            null
        } finally {
            secrets.zeroize()
        }
        if (decoded != null) return decoded
    }
    return null
}

internal fun encodeLanEndpointForPeerHelper(
    handle: CoreHandle,
    orionNode: OrionNodeHandle?,
    store: KeystoreSecretStore?,
    deviceId: String,
): ByteArray? {
    val endpoint = orionNode?.localLanEndpoint() ?: return null
    val activeStore = store ?: return null
    val secrets = activeStore.getPeerSecrets(deviceId) ?: return null
    return try {
        handle.encodeLanEndpoint(endpoint, secrets.reconnectSecret)
    } catch (_: Throwable) {
        null
    } finally {
        secrets.zeroize()
    }
}
