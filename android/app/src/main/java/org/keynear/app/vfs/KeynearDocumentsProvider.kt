package org.keynear.app.vfs

import android.database.Cursor
import android.database.MatrixCursor
import android.os.Bundle
import android.os.CancellationSignal
import android.os.ParcelFileDescriptor
import android.provider.DocumentsContract
import android.provider.DocumentsProvider
import android.util.Log
import java.io.FileNotFoundException
import org.keynear.app.bridge.CoreClientProvider
import org.keynear.app.ui.model.CoreState

interface VfsSource {
    fun roots(): List<VfsDescriptors.Node>
    fun children(parentDocumentId: String): List<VfsDescriptors.Node>
    fun node(documentId: String): VfsDescriptors.Node?
    fun readChunk(documentId: String, offset: Long, maxBytes: Int): ByteArray?
}

class EmptyVfsSource : VfsSource {
    override fun roots(): List<VfsDescriptors.Node> = emptyList()
    override fun children(parentDocumentId: String): List<VfsDescriptors.Node> = emptyList()
    override fun node(documentId: String): VfsDescriptors.Node? = null
    override fun readChunk(documentId: String, offset: Long, maxBytes: Int): ByteArray? = null
}

class KeynearDocumentsProvider : DocumentsProvider() {
    override fun onCreate(): Boolean {
        // TODO: Supply paired-peer descriptors when a VFS peer registry is available.
        val peers = runCatching {
            val snapshot = when (val state = CoreClientProvider.get().loadState()) {
                is CoreState.Empty -> state.snapshot
                is CoreState.Ready -> state.snapshot
                is CoreState.Error -> state.previous
                CoreState.Loading -> null
            }
            snapshot?.pairedDevices?.map { it.deviceId to it.displayName }.orEmpty()
        }.getOrElse { err ->
            Log.w(TAG, "Paired-peer VFS roots unavailable", err)
            emptyList()
        }
        install(RemoteVfsSource(peers, context))
        return true
    }

    override fun queryRoots(projection: Array<out String>?): Cursor {
        val cursor = MatrixCursor(resolveRootProjection(projection))
        for (root in source().roots()) {
            cursor.newRow()
                .add(DocumentsContract.Root.COLUMN_ROOT_ID, root.documentId)
                .add(DocumentsContract.Root.COLUMN_DOCUMENT_ID, root.documentId)
                .add(DocumentsContract.Root.COLUMN_FLAGS, DocumentsContract.Root.FLAG_LOCAL_ONLY)
                .add(DocumentsContract.Root.COLUMN_TITLE, root.name)
                .add(DocumentsContract.Root.COLUMN_SUMMARY, null)
        }
        return cursor
    }

    override fun queryDocument(documentId: String, projection: Array<out String>?): Cursor {
        val node = source().node(documentId) ?: throw FileNotFoundException(documentId)
        return documentCursor(projection, listOf(node))
    }

    override fun queryChildDocuments(
        parentDocumentId: String,
        projection: Array<out String>?,
        sortOrder: String?,
    ): Cursor = queryChildDocuments(parentDocumentId, projection, null as Bundle?)

    override fun queryChildDocuments(
        parentDocumentId: String,
        projection: Array<out String>?,
        queryArgs: Bundle?,
    ): Cursor {
        val offset = queryArgs?.getInt("offset", 0) ?: 0
        val children = VfsDescriptors.page(source().children(parentDocumentId), offset, VfsDescriptors.PAGE_SIZE)
        return documentCursor(projection, children)
    }

    override fun openDocument(
        documentId: String,
        mode: String,
        signal: CancellationSignal?,
    ): ParcelFileDescriptor {
        val node = source().node(documentId) ?: throw FileNotFoundException(documentId)
        val source = source()
        val stream = RemoteDocumentStream { id, offset, maxBytes -> source.readChunk(id, offset, maxBytes) }
        return stream.open(node) ?: throw FileNotFoundException(documentId)
    }

    private fun documentCursor(projection: Array<out String>?, nodes: List<VfsDescriptors.Node>): Cursor {
        val cursor = MatrixCursor(resolveDocumentProjection(projection))
        for (node in nodes) {
            val mime = if (node.isDirectory) {
                DocumentsContract.Document.MIME_TYPE_DIR
            } else {
                "application/octet-stream"
            }
            cursor.newRow()
                .add(DocumentsContract.Document.COLUMN_DOCUMENT_ID, node.documentId)
                .add(DocumentsContract.Document.COLUMN_DISPLAY_NAME, node.name)
                .add(DocumentsContract.Document.COLUMN_MIME_TYPE, mime)
                .add(DocumentsContract.Document.COLUMN_SIZE, node.sizeBytes)
                .add(DocumentsContract.Document.COLUMN_FLAGS, 0)
        }
        return cursor
    }

    companion object {
        private const val TAG = "KeynearVfs"

        private val DEFAULT_ROOT_PROJECTION = arrayOf(
            DocumentsContract.Root.COLUMN_ROOT_ID,
            DocumentsContract.Root.COLUMN_DOCUMENT_ID,
            DocumentsContract.Root.COLUMN_FLAGS,
            DocumentsContract.Root.COLUMN_TITLE,
            DocumentsContract.Root.COLUMN_SUMMARY,
        )

        private val DEFAULT_DOCUMENT_PROJECTION = arrayOf(
            DocumentsContract.Document.COLUMN_DOCUMENT_ID,
            DocumentsContract.Document.COLUMN_DISPLAY_NAME,
            DocumentsContract.Document.COLUMN_MIME_TYPE,
            DocumentsContract.Document.COLUMN_SIZE,
            DocumentsContract.Document.COLUMN_FLAGS,
        )

        @Volatile
        private var installed: VfsSource = EmptyVfsSource()

        fun install(source: VfsSource) {
            installed = source
        }

        private fun source(): VfsSource = installed

        private fun resolveRootProjection(projection: Array<out String>?): Array<out String> =
            projection ?: DEFAULT_ROOT_PROJECTION

        private fun resolveDocumentProjection(projection: Array<out String>?): Array<out String> =
            projection ?: DEFAULT_DOCUMENT_PROJECTION
    }
}
