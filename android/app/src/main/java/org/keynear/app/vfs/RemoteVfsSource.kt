package org.keynear.app.vfs

import android.util.Log
import uniffi.keynear_core.FfiVfsNode
import uniffi.keynear_core.resumableTransferCapabilityFlags
import uniffi.keynear_core.vfsReadPlan
import uniffi.keynear_core.vfsRootNode

class RemoteVfsSource(
    private val peers: List<Pair<String, String>> = emptyList(),
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
        // TODO: Return the planned bytes once a peer-read FFI export is available.
        return null
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
