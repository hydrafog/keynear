package org.keynear.app.data

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.keynear.app.ui.model.TransferDirectionUi
import org.keynear.app.ui.model.TransferStateUi
import uniffi.keynear_core.FfiTransferDirection
import uniffi.keynear_core.FfiTransferItem
import uniffi.keynear_core.FfiTransferSession
import uniffi.keynear_core.FfiTransferState

class TransferMappingTest {
    private fun item(name: String, mime: String = "image/png", directory: Boolean = false) = FfiTransferItem(
        name = name,
        mimeType = mime,
        sizeBytes = 100UL,
        isDirectory = directory,
        targetPath = null,
        checksum = null,
    )

    private fun session(
        items: List<FfiTransferItem> = emptyList(),
        total: ULong = 0UL,
        transferred: ULong = 0UL,
    ) = FfiTransferSession(
        transferId = "tx-42",
        peerDeviceId = "dev-7",
        direction = FfiTransferDirection.RECEIVE,
        items = items,
        totalBytes = total,
        transferredBytes = transferred,
        state = FfiTransferState.ACTIVE,
        startedAt = "1731585600",
    )

    @Test
    fun summarizeTransferItemsCoversEmptySingleAndMultiple() {
        assertEquals("Transfer", TransferRepo.summarizeTransferItems(session()))
        assertEquals("photo.png", TransferRepo.summarizeTransferItems(session(listOf(item("photo.png")))))
        assertEquals(
            "a.txt (+2 more)",
            TransferRepo.summarizeTransferItems(session(listOf(item("a.txt"), item("b.txt"), item("c.txt")))),
        )
    }

    @Test
    fun mapTransferSessionToUiUsesPeerNamesAndItemDefaults() {
        val ui = TransferRepo.mapTransferSessionToUi(
            session(listOf(item("photo.png")), total = 2048UL, transferred = 1024UL),
            mapOf("dev-7" to "Studio Laptop"),
        )
        assertEquals("tx-42", ui.transferId)
        assertEquals("Studio Laptop", ui.peerDisplayName)
        assertEquals("photo.png", ui.summaryName)
        assertEquals("image/png", ui.mimeType)
        assertFalse(ui.isDirectory)
        assertEquals(TransferDirectionUi.RECEIVE, ui.direction)
        assertEquals(TransferStateUi.ACTIVE, ui.state)
        assertEquals(2048L, ui.totalBytes)
        assertEquals(1024L, ui.transferredBytes)
        assertEquals("1731585600", ui.startedAt)

        val unknown = TransferRepo.mapTransferSessionToUi(session(), emptyMap())
        assertEquals("dev-7", unknown.peerDisplayName)
        assertEquals("application/octet-stream", unknown.mimeType)
        assertEquals("Transfer", unknown.summaryName)
    }

    @Test
    fun mapTransferSessionToUiMarksDirectoryAndClampsOverflowByteCounts() {
        val directoryUi = TransferRepo.mapTransferSessionToUi(
            session(listOf(item("album", directory = true))),
            emptyMap(),
        )
        assertTrue(directoryUi.isDirectory)

        val overflow = TransferRepo.mapTransferSessionToUi(
            session(listOf(item("huge.bin")), total = ULong.MAX_VALUE, transferred = ULong.MAX_VALUE),
            emptyMap(),
        )
        assertEquals(0L, overflow.totalBytes)
        assertEquals(0L, overflow.transferredBytes)
    }

    @Test
    fun isSingleFileReceiveRequiresExactlyOneFileItem() {
        assertTrue(TransferRepo.isSingleFileReceive(session(listOf(item("photo.png")))))
        assertFalse(TransferRepo.isSingleFileReceive(session(listOf(item("album", directory = true)))))
        assertFalse(TransferRepo.isSingleFileReceive(session(listOf(item("a"), item("b")))))
        assertFalse(TransferRepo.isSingleFileReceive(session()))
    }
}
