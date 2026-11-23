package org.keynear.app.data

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.keynear.app.bridge.EventQueue
import org.keynear.app.ui.model.SendItemKind
import java.io.File
import java.nio.file.Files

class FolderTransferTest {
    @Test
    fun sanitizePathSegmentBlocksTraversalAndSlashes() {
        assertEquals("Folder", SafFolderHelper.sanitizePathSegment("", "Folder"))
        assertEquals("Folder", SafFolderHelper.sanitizePathSegment(".", "Folder"))
        assertEquals("Folder", SafFolderHelper.sanitizePathSegment("..", "Folder"))
        assertEquals("my_folder", SafFolderHelper.sanitizePathSegment("my/folder/", "Folder"))
        assertEquals("v1.2", SafFolderHelper.sanitizePathSegment("  v1.2  ", "Folder"))
    }

    @Test
    fun uniqueStagingTargetPreservesNamesAndDisambiguatesCollisions() {
        val root = Files.createTempDirectory("keynear-stage-test").toFile()
        try {
            val firstFile = SafFolderHelper.uniqueStagingTarget(root, "report.pdf", isDir = false)
            assertEquals("report.pdf", firstFile.name)
            firstFile.writeText("one")

            val secondFile = SafFolderHelper.uniqueStagingTarget(root, "report.pdf", isDir = false)
            assertEquals("report (1).pdf", secondFile.name)

            val firstDir = SafFolderHelper.uniqueStagingTarget(root, "v1.2", isDir = true)
            assertEquals("v1.2", firstDir.name)
            firstDir.mkdirs()

            val secondDir = SafFolderHelper.uniqueStagingTarget(root, "v1.2", isDir = true)
            assertEquals("v1.2 (1)", secondDir.name)
        } finally {
            root.deleteRecursively()
        }
    }

    @Test
    fun mediaStoreExportPlanPreservesNestedFolderHierarchy() {
        val root = Files.createTempDirectory("keynear-export-test").toFile()
        try {
            val staged = File(root, "Project").apply { mkdirs() }
            File(staged, "README.md").writeText("hello")
            val nested = File(staged, "src/utils").apply { mkdirs() }
            File(nested, "helper.kt").writeText("fun ok() = Unit")
            File(staged, "empty_sub").mkdirs()

            val plan = AndroidFolderExporter.buildMediaStoreExportPlan(staged, "Project (1)")
            assertEquals(2, plan.size)
            assertEquals("Project (1)", plan[0].relativeDir)
            assertEquals("README.md", plan[0].fileName)
            assertEquals("Project (1)/src/utils", plan[1].relativeDir)
            assertEquals("helper.kt", plan[1].fileName)

            val single = File(root, "loose.txt").apply { writeText("solo") }
            val singlePlan = AndroidFolderExporter.buildMediaStoreExportPlan(single, "loose.txt")
            assertEquals(1, singlePlan.size)
            assertEquals("", singlePlan[0].relativeDir)
            assertEquals("loose.txt", singlePlan[0].fileName)
        } finally {
            root.deleteRecursively()
        }
    }

    @Test
    fun pickUniqueFolderNameAndContainmentGuardWork() {
        val taken = setOf("Photos", "Photos (1)")
        assertEquals("Photos (2)", AndroidFolderExporter.pickUniqueFolderName("Photos") { it in taken })
        assertEquals("Docs", AndroidFolderExporter.pickUniqueFolderName("Docs") { it in taken })

        val root = Files.createTempDirectory("keynear-guard-test").toFile()
        val outside = Files.createTempDirectory("keynear-outside-test").toFile()
        try {
            val inside = File(root, "sub/file.txt")
            assertTrue(AndroidFolderExporter.isWithinDirectory(root, inside))
            assertFalse(AndroidFolderExporter.isWithinDirectory(root, outside))
            assertFalse(AndroidFolderExporter.isWithinDirectory(root, File(root, "../escape")))
        } finally {
            root.deleteRecursively()
            outside.deleteRecursively()
        }
    }

    @Test
    fun stageFolderUriKeepsWholeFolderAsSingleQueueEntry() {
        val queue = EventQueue(appContext = null)
        queue.stageFolderUri("content://com.android.externalstorage.documents/tree/primary%3AWork", "Work", 12_345L)
        val staged = queue.sendQueueStore.snapshot()
        assertEquals(1, staged.size)
        assertEquals(SendItemKind.FOLDER, staged[0].kind)
        assertEquals("Work/", staged[0].name)
        assertEquals(12_345L, staged[0].sizeBytes)
        assertEquals("content://com.android.externalstorage.documents/tree/primary%3AWork", staged[0].uriString)
    }
}

