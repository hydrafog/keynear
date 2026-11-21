package org.keynear.app.vfs

object VfsDescriptors {
    const val AUTHORITY: String = "org.keynear.app.vfs.provider"
    const val MAX_NAME_LEN = 128
    const val MAX_DOCUMENT_ID_LEN = 256
    const val PAGE_SIZE = 64
    const val CHUNK_BYTES = 61_440
    const val CAP_RESUMABLE_TRANSFER: UInt = 0x100u

    data class Node(
        val documentId: String,
        val name: String,
        val isDirectory: Boolean,
        val sizeBytes: Long,
    )

    enum class ResumeDecision { Full, FromChunk, Unavailable }

    fun root(rootId: String, label: String): Node? {
        if (!valid(rootId) || !valid(label)) return null
        return Node(rootId, label, true, 0L)
    }

    fun child(rootId: String, relative: String, isDirectory: Boolean, sizeBytes: Long): Node? {
        if (!valid(rootId) || !valid(relative) || sizeBytes < 0) return null
        val segments = relative.split('/')
        if (segments.any { it.isEmpty() || it == "." || it == ".." || it.contains('\\') }) return null
        val documentId = "$rootId/$relative"
        if (documentId.length > MAX_DOCUMENT_ID_LEN) return null
        return Node(documentId, relative, isDirectory, sizeBytes)
    }

    fun <T> page(items: List<T>, offset: Int, limit: Int): List<T> {
        if (limit <= 0) return emptyList()
        val start = offset.coerceIn(0, items.size)
        val end = (start + limit).coerceAtMost(items.size)
        return items.subList(start, end)
    }

    fun resumeDecision(
        localFlags: UInt,
        peerFlags: UInt,
        sequence: Long,
        lastSequence: Long,
        verifiedChunks: Long,
        chunkCount: Long,
    ): ResumeDecision {
        val negotiated = localFlags and peerFlags and CAP_RESUMABLE_TRANSFER
        if (negotiated == 0u || sequence <= lastSequence) return ResumeDecision.Unavailable
        if (verifiedChunks <= 0L) return ResumeDecision.Full
        if (verifiedChunks > chunkCount) return ResumeDecision.Unavailable
        return ResumeDecision.FromChunk
    }

    private fun valid(raw: String): Boolean =
        raw.isNotBlank() && raw.length <= MAX_NAME_LEN && raw.none { it.isISOControl() }
}
