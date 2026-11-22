package org.keynear.app.vfs

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class RemoteVfsTest {
    @Test
    fun providerAuthorityMatchesManifestRegistration() {
        assertEquals("org.keynear.app.vfs.provider", VfsDescriptors.AUTHORITY)
    }

    @Test
    fun rootsAndChildrenUseBoundedOpaqueIds() {
        val root = VfsDescriptors.root("phone-1", "Phone")
        assertTrue(root != null)
        assertTrue(root!!.isDirectory)

        val child = VfsDescriptors.child("phone-1", "docs/report.txt", false, 42L)
        assertEquals("phone-1/docs/report.txt", child?.documentId)
        assertFalse(child!!.isDirectory)
        assertEquals(42L, child.sizeBytes)

        assertNull(VfsDescriptors.child("phone-1", "../escape", false, 1L))
        assertNull(VfsDescriptors.child("phone-1", "a".repeat(129), false, 1L))
        assertNull(VfsDescriptors.child("phone-1", "ok.txt", false, -1L))
    }

    @Test
    fun missingPeerReportsNoRoots() {
        val source = EmptyVfsSource()
        assertTrue(source.roots().isEmpty())
        assertNull(source.node("phone-1"))
        assertNull(source.readChunk("phone-1/file", 0L, 1_024))
    }

    @Test
    fun paginationClampsToAvailableRows() {
        val rows = (0 until 5).toList()
        assertEquals(listOf(0, 1), VfsDescriptors.page(rows, 0, 2))
        assertEquals(listOf(4), VfsDescriptors.page(rows, 4, 4))
        assertTrue(VfsDescriptors.page(rows, 9, 4).isEmpty())
        assertTrue(VfsDescriptors.page(rows, 0, 0).isEmpty())
    }

    @Test
    fun resumeRequiresCapabilityAndFreshSequence() {
        val capability = VfsDescriptors.CAP_RESUMABLE_TRANSFER
        assertEquals(VfsDescriptors.ResumeDecision.FromChunk, VfsDescriptors.resumeDecision(capability, capability, 3L, 1L, 7L, 12L))
        assertEquals(VfsDescriptors.ResumeDecision.Full, VfsDescriptors.resumeDecision(capability, capability, 3L, 1L, 0L, 12L))
        assertEquals(VfsDescriptors.ResumeDecision.Unavailable, VfsDescriptors.resumeDecision(capability, 0u, 3L, 1L, 7L, 12L))
        assertEquals(VfsDescriptors.ResumeDecision.Unavailable, VfsDescriptors.resumeDecision(capability, capability, 2L, 2L, 7L, 12L))
    }
}
