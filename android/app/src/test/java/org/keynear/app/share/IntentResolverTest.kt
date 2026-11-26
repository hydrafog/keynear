package org.keynear.app.share

import android.content.Context
import android.content.ContextWrapper
import org.junit.Assert.assertEquals
import org.junit.Test
import sun.misc.Unsafe

class IntentResolverTest {
    @Test
    fun computeTotalShareBytesReturnsZeroForMissingOrBlankText() {
        assertEquals(0L, totalBytesFor(null))
        assertEquals(0L, totalBytesFor(""))
        assertEquals(0L, totalBytesFor("   "))
    }

    @Test
    fun computeTotalShareBytesCountsUtf8BytesOfText() {
        assertEquals(5L, totalBytesFor("hello"))
        assertEquals(6L, totalBytesFor("héllo"))
        assertEquals(9L, totalBytesFor("日本語"))
        assertEquals(4L, totalBytesFor("🙂"))
        assertEquals(2L, totalBytesFor("  hi  "))
    }

    private fun totalBytesFor(sharedText: String?): Long =
        IntentResolver.computeTotalShareBytes(unusedContext(), emptyList(), sharedText)

    // HACK: Allocation skips Android's throwing "Stub!" constructor; the empty URI list reads no members.
    private fun unusedContext(): Context {
        val field = Unsafe::class.java.getDeclaredField("theUnsafe")
        field.isAccessible = true
        val unsafe = field.get(null) as Unsafe
        return unsafe.allocateInstance(ContextWrapper::class.java) as Context
    }
}
