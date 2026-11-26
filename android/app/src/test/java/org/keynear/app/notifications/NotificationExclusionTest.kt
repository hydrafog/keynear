package org.keynear.app.notifications

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class NotificationExclusionTest {
    @Test
    fun isExcludedMatchesExactPackageNameOnly() {
        val exclusions = setOf("com.secret.chat", "org.example.vault")
        assertTrue(NotificationMirror.isExcluded("com.secret.chat", exclusions))
        assertFalse(NotificationMirror.isExcluded("COM.SECRET.CHAT", exclusions))
        assertFalse(NotificationMirror.isExcluded("com.secret.chat.v2", exclusions))
        assertFalse(NotificationMirror.isExcluded("org.example.chat", emptySet()))
        assertFalse(NotificationMirror.isExcluded("", exclusions))
    }
}
