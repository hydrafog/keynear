package org.keynear.app.ui.navigation

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.DocumentsContract
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.CoreState

fun launchFirstResolvableActivity(
    context: Context,
    candidates: List<Intent>,
): Boolean {
    for (candidate in candidates) {
        val intent = Intent(candidate).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        if (intent.resolveActivity(context.packageManager) != null) {
            runCatching {
                context.startActivity(intent)
                return true
            }
        }
    }
    return false
}

fun snapshotFromState(state: CoreState): CoreSnapshot? = when (state) {
    is CoreState.Empty -> state.snapshot
    is CoreState.Ready -> state.snapshot
    is CoreState.Error -> state.previous
    CoreState.Loading -> null
}

fun enumerateTreeChildUris(context: Context, treeUri: Uri): Pair<String, List<String>> {
    val treeDocId = DocumentsContract.getTreeDocumentId(treeUri)
    val folderLabel = treeDocId.substringAfterLast(':').substringAfterLast('/').ifBlank { "Selected folder" }
    val childrenUri = DocumentsContract.buildChildDocumentsUriUsingTree(treeUri, treeDocId)
    val collected = mutableListOf<String>()
    runCatching {
        context.contentResolver.query(
            childrenUri,
            arrayOf(
                DocumentsContract.Document.COLUMN_DOCUMENT_ID,
                DocumentsContract.Document.COLUMN_MIME_TYPE,
            ),
            null,
            null,
            null,
        )?.use { cursor ->
            val idIdx = cursor.getColumnIndex(DocumentsContract.Document.COLUMN_DOCUMENT_ID)
            val mimeIdx = cursor.getColumnIndex(DocumentsContract.Document.COLUMN_MIME_TYPE)
            while (cursor.moveToNext()) {
                val docId = if (idIdx >= 0) cursor.getString(idIdx) else null
                val mime = if (mimeIdx >= 0) cursor.getString(mimeIdx) else null
                if (!docId.isNullOrBlank() && mime != DocumentsContract.Document.MIME_TYPE_DIR) {
                    collected.add(DocumentsContract.buildDocumentUriUsingTree(treeUri, docId).toString())
                }
            }
        }
    }
    return folderLabel to collected
}
