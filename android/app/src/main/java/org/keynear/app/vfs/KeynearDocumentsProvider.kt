package org.keynear.app.vfs

import android.database.Cursor
import android.database.MatrixCursor
import android.os.Bundle
import android.os.CancellationSignal
import android.os.ParcelFileDescriptor
import android.provider.DocumentsContract
import android.provider.DocumentsProvider
import java.io.FileNotFoundException

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
    override fun onCreate(): Boolean = true

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
        @Volatile
        private var installed: VfsSource = EmptyVfsSource()

        fun install(source: VfsSource) {
            installed = source
        }

        private fun source(): VfsSource = installed
    }
}
