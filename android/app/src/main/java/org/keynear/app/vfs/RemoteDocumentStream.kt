package org.keynear.app.vfs

import android.os.ParcelFileDescriptor
import java.io.IOException

typealias RemoteChunkReader = (documentId: String, offset: Long, maxBytes: Int) -> ByteArray?

class RemoteDocumentStream(private val reader: RemoteChunkReader) {
    fun open(node: VfsDescriptors.Node): ParcelFileDescriptor? {
        if (node.isDirectory) return null
        val pipe = ParcelFileDescriptor.createPipe()
        val readSide = pipe[0]
        val writeSide = pipe[1]
        val writer = Thread {
            try {
                ParcelFileDescriptor.AutoCloseOutputStream(writeSide).use { output ->
                    var offset = 0L
                    while (offset < node.sizeBytes) {
                        val chunk = reader(node.documentId, offset, VfsDescriptors.CHUNK_BYTES)
                        if (chunk == null || chunk.isEmpty()) break
                        output.write(chunk)
                        offset += chunk.size
                    }
                }
            } catch (_: IOException) {
                try {
                    readSide.close()
                } catch (_: IOException) {
                    Unit
                }
            }
        }
        writer.isDaemon = true
        writer.start()
        return readSide
    }
}
