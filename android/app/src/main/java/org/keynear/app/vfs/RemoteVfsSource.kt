package org.keynear.app.vfs

import android.content.Context
import android.util.Log
import org.keynear.app.bridge.CoreClientProvider
import uniffi.keynear_core.FfiVfsNode
import uniffi.keynear_core.OrionNodeHandle
import uniffi.keynear_core.resumableTransferCapabilityFlags
import uniffi.keynear_core.vfsReadPlan
import uniffi.keynear_core.vfsRootNode

class RemoteVfsSource(
    private val peers: List<Pair<String, String>> = emptyList(),
    private val context: Context? = null,
) : VfsSource {
    override fun roots(): List<VfsDescriptors.Node> =
        peers.mapNotNull { (peerId, label) -> rootNode(peerId, label) }

    override fun children(parentDocumentId: String): List<VfsDescriptors.Node> = emptyList()

    override fun node(documentId: String): VfsDescriptors.Node? =
        roots().firstOrNull { it.documentId == documentId }

    override fun readChunk(documentId: String, offset: Long, maxBytes: Int): ByteArray? {
        val node = node(documentId) ?: return null
        if (!supportsRangedReads()) return null
        val plan = runCatching {
            vfsReadPlan(
                FfiVfsNode(
                    node.documentId,
                    node.name,
                    node.isDirectory,
                    node.sizeBytes.coerceAtLeast(0L).toULong(),
                ),
                offset.coerceAtLeast(0L).toULong(),
                maxBytes.coerceAtLeast(0).toULong(),
            )
        }.getOrElse { err ->
            Log.w(TAG, "VFS read plan unavailable for $documentId", err)
            return null
        }
        if (plan.length == 0UL) return null

        val peerId = peers.firstOrNull { documentId == it.first || documentId.startsWith("${it.first}/") }?.first
            ?: run {
                Log.w(TAG, "No paired peer owns $documentId")
                return null
            }
        val relativePath = documentId.removePrefix(peerId).trimStart('/')
        if (relativePath.isEmpty()) {
            Log.w(TAG, "Peer root has no readable bytes: $documentId")
            return null
        }
        val handle = orionNode() ?: return null
        return runCatching {
            handle.readRemoteVfsChunk(peerId, relativePath, plan.offset, plan.length.toUInt())
        }.getOrElse { err ->
            Log.w(TAG, "VFS chunk fetch failed for $documentId", err)
            null
        }.takeIf { it?.isNotEmpty() == true }
    }

    private fun orionNode(): OrionNodeHandle? {
        val ctx = context ?: return null
        return runCatching {
            CoreClientProvider.getClient().ensureOrionNode(ctx)
        }.getOrElse { err ->
            Log.w(TAG, "Orion node unavailable for VFS read", err)
            null
        }
    }

    private fun rootNode(peerId: String, label: String): VfsDescriptors.Node? =
        runCatching { vfsRootNode(peerId, label) }
            .map { ffi ->
                VfsDescriptors.Node(
                    ffi.documentId,
                    ffi.name,
                    ffi.isDirectory,
                    ffi.sizeBytes.toLong(),
                )
            }
            .getOrElse { err ->
                Log.w(TAG, "VFS root unavailable for $peerId", err)
                null
            }

    private fun supportsRangedReads(): Boolean =
        runCatching {
            (resumableTransferCapabilityFlags() and VfsDescriptors.CAP_RESUMABLE_TRANSFER) != 0u
        }.getOrElse { err ->
            Log.w(TAG, "VFS capability flags unavailable", err)
            false
        }

    companion object {
        private const val TAG = "KeynearVfs"
    }
}
