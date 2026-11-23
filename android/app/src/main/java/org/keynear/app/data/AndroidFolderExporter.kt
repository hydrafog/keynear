package org.keynear.app.data

import android.content.ContentValues
import android.content.Context
import android.net.Uri
import android.os.Environment
import android.provider.DocumentsContract
import android.provider.MediaStore
import org.keynear.app.bridge.resolveDownloadMimeType
import uniffi.keynear_core.FfiTransferSession
import java.io.File
import java.io.FileInputStream
import java.util.UUID

data class ExportFileSpec(
    val sourceFile: File,
    val relativeDir: String,
    val fileName: String,
)

object AndroidFolderExporter {
    private const val UI_PREFS_NAME = "keynear_ui_prefs"
    private const val PREF_SAVE_FOLDER_URI = "save_folder_tree_uri"
    private const val COPY_BUFFER_BYTES = 65_536
    private const val MAX_COLLISION_ATTEMPTS = 10_000

    fun saveFolderTreeUriPref(context: Context, treeUri: Uri) {
        context.getSharedPreferences(UI_PREFS_NAME, Context.MODE_PRIVATE)
            .edit()
            .putString(PREF_SAVE_FOLDER_URI, treeUri.toString())
            .apply()
    }

    fun readSaveFolderTreeUri(context: Context): Uri? =
        context.getSharedPreferences(UI_PREFS_NAME, Context.MODE_PRIVATE)
            .getString(PREF_SAVE_FOLDER_URI, null)
            ?.takeIf { it.isNotBlank() }
            ?.let { runCatching { Uri.parse(it) }.getOrNull() }

    fun isWithinDirectory(root: File, candidate: File): Boolean = runCatching {
        val rootPath = root.canonicalPath
        val candPath = candidate.canonicalPath
        candPath == rootPath || candPath.startsWith("$rootPath${File.separator}")
    }.getOrDefault(false)

    fun pickUniqueFolderName(baseName: String, exists: (String) -> Boolean): String {
        val clean = SafFolderHelper.sanitizePathSegment(baseName, "Folder")
        if (!exists(clean)) return clean
        for (idx in 1 until MAX_COLLISION_ATTEMPTS) {
            val candidate = "$clean ($idx)"
            if (!exists(candidate)) return candidate
        }
        return "$clean-${UUID.randomUUID()}"
    }

    fun buildMediaStoreExportPlan(stagedRoot: File, topName: String): List<ExportFileSpec> {
        if (stagedRoot.isFile) {
            val cleanFile = SafFolderHelper.sanitizePathSegment(topName, "shared-file.bin")
            return listOf(ExportFileSpec(stagedRoot, "", cleanFile))
        }
        if (!stagedRoot.isDirectory) return emptyList()
        val cleanTop = SafFolderHelper.sanitizePathSegment(topName, "Folder")
        val specs = mutableListOf<ExportFileSpec>()
        collectDirSpecs(stagedRoot, cleanTop, specs)
        return specs
    }

    private fun collectDirSpecs(dir: File, relDir: String, out: MutableList<ExportFileSpec>) {
        val children = dir.listFiles()?.sortedBy { it.name } ?: return
        for (child in children) {
            if (child.isDirectory) {
                val safeSub = SafFolderHelper.sanitizePathSegment(child.name, "subfolder")
                collectDirSpecs(child, "$relDir/$safeSub", out)
            } else if (child.isFile) {
                val safeFile = SafFolderHelper.sanitizePathSegment(child.name, "file.bin")
                out.add(ExportFileSpec(child, relDir, safeFile))
            }
        }
    }

    fun exportReceivedArchive(
        context: Context,
        session: FfiTransferSession,
        incomingRoot: File,
    ): Result<Uri?> = runCatching {
        val targets = resolveStagedTargets(session, incomingRoot)
        if (targets.isEmpty()) return@runCatching null

        val safTree = readWritableSaveFolderUri(context)
        val firstUri = if (safTree != null) {
            exportTargetsToSaf(context, safTree, targets)
        } else {
            exportTargetsToMediaStore(context, targets)
        }
        targets.forEach { it.deleteRecursively() }
        firstUri
    }

    private fun resolveStagedTargets(session: FfiTransferSession, incomingRoot: File): List<File> {
        val explicit = session.items
            .mapNotNull { it.targetPath?.takeIf(String::isNotBlank)?.let(::File) }
            .distinctBy { it.absolutePath }
            .filter { it.exists() && isWithinDirectory(incomingRoot, it) }
        if (explicit.isNotEmpty()) return explicit

        return session.items
            .map { File(incomingRoot, SafFolderHelper.sanitizePathSegment(it.name, "transfer")) }
            .distinctBy { it.absolutePath }
            .filter { it.exists() && isWithinDirectory(incomingRoot, it) }
    }

    private fun readWritableSaveFolderUri(context: Context): Uri? {
        val saved = readSaveFolderTreeUri(context) ?: return null
        val granted = context.contentResolver.persistedUriPermissions.any {
            it.uri == saved && it.isWritePermission
        }
        return if (granted) saved else null
    }

    private fun exportTargetsToSaf(context: Context, treeUri: Uri, targets: List<File>): Uri? {
        val rootDocId = DocumentsContract.getTreeDocumentId(treeUri)
        val rootDocUri = DocumentsContract.buildDocumentUriUsingTree(treeUri, rootDocId)
        val existingNames = querySafChildNames(context, treeUri, rootDocId).toMutableSet()
        var firstExported: Uri? = null

        for (target in targets) {
            val exported = if (target.isDirectory) {
                val uniqueDir = pickUniqueFolderName(target.name) { it in existingNames }
                existingNames.add(uniqueDir)
                exportDirectoryToSaf(context, rootDocUri, target, uniqueDir)
            } else {
                exportFileToSaf(context, rootDocUri, target, target.name)
            }
            if (firstExported == null) firstExported = exported
        }
        return firstExported
    }

    private fun querySafChildNames(context: Context, treeUri: Uri, parentDocId: String): Set<String> {
        val childrenUri = DocumentsContract.buildChildDocumentsUriUsingTree(treeUri, parentDocId)
        return runCatching {
            context.contentResolver.query(
                childrenUri,
                arrayOf(DocumentsContract.Document.COLUMN_DISPLAY_NAME),
                null,
                null,
                null,
            )?.use(::readDisplayNames)
        }.getOrNull().orEmpty()
    }

    private fun readDisplayNames(cursor: android.database.Cursor): Set<String> {
        val names = hashSetOf<String>()
        val idx = cursor.getColumnIndex(DocumentsContract.Document.COLUMN_DISPLAY_NAME)
        if (idx < 0) return names
        while (cursor.moveToNext()) {
            val name = cursor.getString(idx)
            if (!name.isNullOrBlank()) names.add(name)
        }
        return names
    }

    private fun exportDirectoryToSaf(
        context: Context,
        parentUri: Uri,
        sourceDir: File,
        dirName: String,
    ): Uri {
        val createdDir = DocumentsContract.createDocument(
            context.contentResolver,
            parentUri,
            DocumentsContract.Document.MIME_TYPE_DIR,
            dirName,
        ) ?: error("Unable to create folder $dirName in save location")
        val children = sourceDir.listFiles()?.sortedBy { it.name }.orEmpty()
        for (child in children) {
            if (child.isDirectory) {
                val subName = SafFolderHelper.sanitizePathSegment(child.name, "subfolder")
                exportDirectoryToSaf(context, createdDir, child, subName)
            } else if (child.isFile) {
                val fileName = SafFolderHelper.sanitizePathSegment(child.name, "file.bin")
                exportFileToSaf(context, createdDir, child, fileName)
            }
        }
        return createdDir
    }

    private fun exportFileToSaf(context: Context, parentUri: Uri, sourceFile: File, displayName: String): Uri {
        val mime = resolveDownloadMimeType(displayName, "application/octet-stream")
        val docUri = DocumentsContract.createDocument(context.contentResolver, parentUri, mime, displayName)
            ?: error("Unable to create file $displayName in save location")
        val output = context.contentResolver.openOutputStream(docUri)
            ?: error("Unable to open output stream for $displayName")
        output.use { out ->
            FileInputStream(sourceFile).use { input ->
                input.copyTo(out, bufferSize = COPY_BUFFER_BYTES)
            }
        }
        return docUri
    }

    private fun exportTargetsToMediaStore(context: Context, targets: List<File>): Uri? {
        var firstUri: Uri? = null
        val reservedDirs = hashSetOf<String>()
        for (target in targets) {
            val topName = if (target.isDirectory) {
                pickUniqueFolderName(target.name) { candidate ->
                    candidate in reservedDirs || mediaStoreFolderExists(context, candidate)
                }.also(reservedDirs::add)
            } else {
                target.name
            }
            val specs = buildMediaStoreExportPlan(target, topName)
            for (spec in specs) {
                val uri = writeSpecToMediaStore(context, spec)
                if (firstUri == null) firstUri = uri
            }
        }
        return firstUri
    }

    private fun mediaStoreFolderExists(context: Context, folderName: String): Boolean {
        val base = Environment.DIRECTORY_DOWNLOADS
        val exact = "$base/$folderName/"
        val escaped = escapeSqlLike(exact)
        val selection = "${MediaStore.Downloads.RELATIVE_PATH} = ? OR " +
            "${MediaStore.Downloads.RELATIVE_PATH} LIKE ? ESCAPE '\\'"
        return runCatching {
            context.contentResolver.query(
                MediaStore.Downloads.EXTERNAL_CONTENT_URI,
                arrayOf(MediaStore.Downloads._ID),
                selection,
                arrayOf(exact, "$escaped%"),
                null,
            )?.use { cursor -> cursor.moveToFirst() } ?: false
        }.getOrDefault(false)
    }

    private fun escapeSqlLike(raw: String): String =
        raw.replace("\\", "\\\\").replace("%", "\\%").replace("_", "\\_")

    private fun writeSpecToMediaStore(context: Context, spec: ExportFileSpec): Uri {
        val relPath = if (spec.relativeDir.isEmpty()) {
            Environment.DIRECTORY_DOWNLOADS
        } else {
            "${Environment.DIRECTORY_DOWNLOADS}/${spec.relativeDir}"
        }
        val mime = resolveDownloadMimeType(spec.fileName, "application/octet-stream")
        val values = ContentValues().apply {
            put(MediaStore.Downloads.DISPLAY_NAME, spec.fileName)
            put(MediaStore.Downloads.MIME_TYPE, mime)
            put(MediaStore.Downloads.RELATIVE_PATH, relPath)
            put(MediaStore.Downloads.IS_PENDING, 1)
        }
        val uri = context.contentResolver.insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI, values)
            ?: error("Unable to insert ${spec.fileName} into Downloads")
        var completed = false
        try {
            val out = context.contentResolver.openOutputStream(uri)
                ?: error("Unable to open Downloads stream for ${spec.fileName}")
            out.use { output ->
                FileInputStream(spec.sourceFile).use { input ->
                    input.copyTo(output, bufferSize = COPY_BUFFER_BYTES)
                }
            }
            val clearValues = ContentValues().apply { put(MediaStore.Downloads.IS_PENDING, 0) }
            context.contentResolver.update(uri, clearValues, null, null)
            completed = true
            return uri
        } finally {
            if (!completed) {
                runCatching { context.contentResolver.delete(uri, null, null) }
            }
        }
    }
}
