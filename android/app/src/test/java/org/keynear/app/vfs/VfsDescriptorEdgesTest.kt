package org.keynear.app.vfs

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class VfsDescriptorEdgesTest {
    @Test
    fun pageClampsNegativeOffsetAndRejectsNegativeLimit() {
        val rows = (0 until 5).toList()
        assertEquals(listOf(0, 1, 2), VfsDescriptors.page(rows, -3, 3))
        assertTrue(VfsDescriptors.page(rows, 0, -1).isEmpty())
        assertTrue(VfsDescriptors.page(rows, 5, 3).isEmpty())
        assertEquals(rows, VfsDescriptors.page(rows, -1, 10))
    }

    @Test
    fun resumeDecisionRejectsExcessChunksAndNonIncreasingSequence() {
        val capability = VfsDescriptors.CAP_RESUMABLE_TRANSFER
        assertEquals(
            VfsDescriptors.ResumeDecision.Unavailable,
            VfsDescriptors.resumeDecision(capability, capability, 4L, 1L, 13L, 12L),
        )
        assertEquals(
            VfsDescriptors.ResumeDecision.FromChunk,
            VfsDescriptors.resumeDecision(capability, capability, 4L, 1L, 12L, 12L),
        )
        assertEquals(
            VfsDescriptors.ResumeDecision.Unavailable,
            VfsDescriptors.resumeDecision(capability, capability, 1L, 2L, 7L, 12L),
        )
        assertEquals(
            VfsDescriptors.ResumeDecision.Unavailable,
            VfsDescriptors.resumeDecision(0u, capability, 4L, 1L, 7L, 12L),
        )
    }
}
