package org.keynear.app.data

import android.content.Context
import android.net.Uri
import android.provider.DocumentsContract
import java.io.File
import java.io.FileOutputStream
import java.util.UUID

data class FolderSummary(
    val displayName: String,
    val totalBytes: Long,
    val fileCount: Int,
)

object SafFolderHelper {
    const val MAX_FOLDER_DEPTH = 64
    const val MAX_FOLDER_ENTRIES = 10_000
    private const val COPY_BUFFER_BYTES = 65_536
    private const val MAX_COLLISION_ATTEMPTS = 10_000

    private val CHILD_PROJECTION = arrayOf(
        DocumentsContract.Document.COLUMN_DOCUMENT_ID,
        DocumentsContract.Document.COLUMN_DISPLAY_NAME,
        DocumentsContract.Document.COLUMN_MIME_TYPE,
        DocumentsContract.Document.COLUMN_SIZE,
    )

    fun isTreeDocumentUri(uri: Uri): Boolean =
        runCatching { DocumentsContract.isTreeUri(uri) }.getOrDefault(false)

    fun sanitizePathSegment(raw: String, fallback: String): String {
        val cleaned = raw
            .trim()
            .trimEnd('/', '\\')
            .replace('/', '_')
            .replace('\\', '_')
            .replace("\u0000", "")
        if (cleaned.isEmpty() || cleaned == "." || cleaned == "..") {
            return fallback
        }
        return cleaned
    }

    fun uniqueStagingTarget(parent: File, rawName: String, isDir: Boolean): File {
        val fallback = if (isDir) "Folder" else "shared-file.bin"
        val clean = sanitizePathSegment(rawName, fallback)
        val first = File(parent, clean)
        if (!first.exists()) return first

        val dotIdx = if (isDir) -1 else clean.lastIndexOf('.')
        val hasExt = dotIdx > 0 && dotIdx < clean.length - 1
        val stem = if (hasExt) clean.substring(0, dotIdx) else clean
        val ext = if (hasExt) clean.substring(dotIdx + 1) else null

        for (idx in 1 until MAX_COLLISION_ATTEMPTS) {
            val candidateName = if (ext != null) "$stem ($idx).$ext" else "$stem ($idx)"
            val candidate = File(parent, candidateName)
            if (!candidate.exists()) return candidate
        }
        return File(parent, "$stem-${UUID.randomUUID()}")
    }

    fun resolveTreeDisplayName(context: Context, treeUri: Uri): String {
        val treeDocId = DocumentsContract.getTreeDocumentId(treeUri)
        val docUri = DocumentsContract.buildDocumentUriUsingTree(treeUri, treeDocId)
        val queried = runCatching {
            context.contentResolver.query(
                docUri,
                arrayOf(DocumentsContract.Document.COLUMN_DISPLAY_NAME),
                null,
                null,
                null,
            )?.use { cursor ->
                val idx = cursor.getColumnIndex(DocumentsContract.Document.COLUMN_DISPLAY_NAME)
                if (idx >= 0 && cursor.moveToFirst()) cursor.getString(idx) else null
            }
        }.getOrNull()
        val fallback = treeDocId.substringAfterLast(':').substringAfterLast('/').ifBlank { "Folder" }
        return sanitizePathSegment(queried ?: fallback, "Folder")
    }

    fun inspectFolderTree(context: Context, treeUri: Uri): FolderSummary {
        val displayName = resolveTreeDisplayName(context, treeUri)
        val treeDocId = DocumentsContract.getTreeDocumentId(treeUri)
        val state = InspectState(context = context, treeUri = treeUri)
        state.walk(treeDocId, depth = 0)
        return FolderSummary(
            displayName = displayName,
            totalBytes = state.totalBytes,
            fileCount = state.fileCount,
        )
    }

    fun stageFolderTreeToDirectory(context: Context, treeUri: Uri, targetDir: File): Int {
        targetDir.mkdirs()
        val treeDocId = DocumentsContract.getTreeDocumentId(treeUri)
        val state = StageState(context = context, treeUri = treeUri)
        state.walk(treeDocId, targetDir, depth = 0)
        return state.copiedFiles
    }

    private data class ChildDoc(
        val docId: String,
        val displayName: String,
        val isDirectory: Boolean,
        val sizeBytes: Long,
    )

    private fun queryChildren(context: Context, treeUri: Uri, parentDocId: String): List<ChildDoc> {
        val childrenUri = DocumentsContract.buildChildDocumentsUriUsingTree(treeUri, parentDocId)
        val result = runCatching {
            context.contentResolver.query(childrenUri, CHILD_PROJECTION, null, null, null)?.use { cursor ->
                readCursorChildren(cursor)
            }
        }.getOrNull().orEmpty()
        return result.sortedBy { it.displayName }
    }

    private fun readCursorChildren(cursor: android.database.Cursor): List<ChildDoc> {
        val result = mutableListOf<ChildDoc>()
        val idIdx = cursor.getColumnIndex(DocumentsContract.Document.COLUMN_DOCUMENT_ID)
        val nameIdx = cursor.getColumnIndex(DocumentsContract.Document.COLUMN_DISPLAY_NAME)
        val mimeIdx = cursor.getColumnIndex(DocumentsContract.Document.COLUMN_MIME_TYPE)
        val sizeIdx = cursor.getColumnIndex(DocumentsContract.Document.COLUMN_SIZE)
        while (cursor.moveToNext()) {
            val docId = if (idIdx >= 0) cursor.getString(idIdx) else null
            if (docId.isNullOrBlank()) continue
            val mime = if (mimeIdx >= 0) cursor.getString(mimeIdx) else null
            val rawName = if (nameIdx >= 0) cursor.getString(nameIdx) else null
            val size = if (sizeIdx >= 0 && !cursor.isNull(sizeIdx)) cursor.getLong(sizeIdx) else 0L
            val isDir = mime == DocumentsContract.Document.MIME_TYPE_DIR
            val name = sanitizePathSegment(rawName.orEmpty(), if (isDir) "subfolder" else "file.bin")
            result.add(ChildDoc(docId, name, isDir, size.coerceAtLeast(0L)))
        }
        return result
    }

    private class InspectState(
        private val context: Context,
        private val treeUri: Uri,
    ) {
        private val visited = hashSetOf<String>()
        private var entries = 0
        var totalBytes = 0L
        var fileCount = 0

        fun walk(docId: String, depth: Int) {
            if (depth > MAX_FOLDER_DEPTH || entries >= MAX_FOLDER_ENTRIES || !visited.add(docId)) return
            for (child in queryChildren(context, treeUri, docId)) {
                if (entries >= MAX_FOLDER_ENTRIES) break
                entries += 1
                if (child.isDirectory) {
                    walk(child.docId, depth + 1)
                } else {
                    fileCount += 1
                    totalBytes += child.sizeBytes
                }
            }
        }
    }

    private class StageState(
        private val context: Context,
        private val treeUri: Uri,
    ) {
        private val visited = hashSetOf<String>()
        private var entries = 0
        var copiedFiles = 0

        fun walk(docId: String, destDir: File, depth: Int) {
            if (depth > MAX_FOLDER_DEPTH || entries >= MAX_FOLDER_ENTRIES || !visited.add(docId)) return
            for (child in queryChildren(context, treeUri, docId)) {
                if (entries >= MAX_FOLDER_ENTRIES) break
                entries += 1
                if (child.isDirectory) {
                    val subDir = uniqueStagingTarget(destDir, child.displayName, isDir = true)
                    subDir.mkdirs()
                    walk(child.docId, subDir, depth + 1)
                } else {
                    copyChildFile(child, destDir)
                }
            }
        }

        private fun copyChildFile(child: ChildDoc, destDir: File) {
            val docUri = DocumentsContract.buildDocumentUriUsingTree(treeUri, child.docId)
            val destFile = uniqueStagingTarget(destDir, child.displayName, isDir = false)
            context.contentResolver.openInputStream(docUri)?.use { input ->
                FileOutputStream(destFile).use { output ->
                    input.copyTo(output, bufferSize = COPY_BUFFER_BYTES)
                }
            } ?: error("Unable to read folder entry ${child.displayName}")
            copiedFiles += 1
        }
    }
}
