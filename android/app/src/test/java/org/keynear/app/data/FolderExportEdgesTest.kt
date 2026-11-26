package org.keynear.app.data

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File
import java.nio.file.Files

class FolderExportEdgesTest {
    @Test
    fun pickUniqueFolderNameFallsBackAndSanitizesBase() {
        val taken = setOf("Docs", "Docs (1)")
        assertEquals("Docs (2)", AndroidFolderExporter.pickUniqueFolderName("Docs", taken::contains))
        assertEquals("my_folder", AndroidFolderExporter.pickUniqueFolderName("my/folder/", { false }))
        assertEquals("Folder", AndroidFolderExporter.pickUniqueFolderName("..", { false }))
    }

    @Test
    fun isWithinDirectoryRejectsPrefixSiblingAndReversedContainment() {
        val root = Files.createTempDirectory("keynear-within-test").toFile()
        val sibling = File(root.parentFile, "${root.name}-extra").apply { mkdirs() }
        try {
            assertTrue(AndroidFolderExporter.isWithinDirectory(root, root))
            assertFalse(AndroidFolderExporter.isWithinDirectory(root, sibling))
            assertFalse(AndroidFolderExporter.isWithinDirectory(File(root, "nested"), root))
        } finally {
            root.deleteRecursively()
            sibling.deleteRecursively()
        }
    }

    @Test
    fun buildMediaStoreExportPlanSanitizesNamesAndSortsNestedFiles() {
        val root = Files.createTempDirectory("keynear-plan-test").toFile()
        try {
            val staged = File(root, "Album").apply { mkdirs() }
            File(staged, "b.txt").writeText("b")
            File(staged, "a.txt").writeText("a")
            File(staged, "note").mkdirs()
            File(File(staged, "note"), "n.txt").writeText("n")

            val plan = AndroidFolderExporter.buildMediaStoreExportPlan(staged, "Album (1)")
            assertEquals(3, plan.size)
            assertEquals("Album (1)", plan[0].relativeDir)
            assertEquals("a.txt", plan[0].fileName)
            assertEquals("b.txt", plan[1].fileName)
            assertEquals("Album (1)/note", plan[2].relativeDir)
            assertEquals("n.txt", plan[2].fileName)

            val weird = File(root, "weird.bin").apply { writeText("x") }
            assertEquals(
                "a_b.txt",
                AndroidFolderExporter.buildMediaStoreExportPlan(weird, "a/b.txt").single().fileName,
            )
            assertTrue(AndroidFolderExporter.buildMediaStoreExportPlan(File(root, "missing"), "x").isEmpty())
        } finally {
            root.deleteRecursively()
        }
    }
}
