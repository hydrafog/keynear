package org.keynear.app.carina

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.keynear.app.call.CallAlertController
import org.keynear.app.media.MediaBridgeController
import org.keynear.app.notifications.NotificationMirror

class CarinaMirrorAndMediaTest {

    private fun descriptor(
        key: String = "n-1",
        packageName: String = "org.example.chat",
        sequence: Long = 2L,
    ): NotificationMirror.Descriptor {
        return NotificationMirror.Descriptor(
            key = key,
            packageName = packageName,
            title = "Alice",
            body = "Hello",
            sequence = sequence,
            supportsReply = true,
            supportsMarkRead = true,
            supportsDismiss = true,
        )
    }

    @Test
    fun excludedPackageNeverReachesTheMirror() {
        val exclusions = setOf("com.secret.chat")
        assertFalse(NotificationMirror.accept(descriptor(packageName = "com.secret.chat"), exclusions, 0L))
        assertTrue(NotificationMirror.accept(descriptor(), exclusions, 0L))
    }

    @Test
    fun staleAndUnsafeNotificationsAreRejected() {
        assertFalse(NotificationMirror.accept(descriptor(sequence = 1L), emptySet(), 2L))
        assertFalse(NotificationMirror.accept(descriptor(key = "bad\nkey"), emptySet(), 0L))
        assertFalse(NotificationMirror.isValidText("x".repeat(2049), 2048))
        assertTrue(NotificationMirror.isValidText("", 2048, allowEmpty = true))
    }

    @Test
    fun actionKeysUseTheConfirmedPrefixes() {
        assertEquals("mirror:reply:n-1", NotificationMirror.actionKey(NotificationMirror.PREFIX_REPLY, "n-1"))
        assertEquals(
            Pair(NotificationMirror.PREFIX_MUTE, "call-1"),
            NotificationMirror.parseAction("call:mute:call-1"),
        )
        assertNull(NotificationMirror.parseAction("mirror:reply:"))
        assertNull(NotificationMirror.parseAction("unknown:n-1"))
    }

    @Test
    fun actionLabelsMapToTheConfirmedActions() {
        assertEquals(NotificationMirror.LABEL_ANSWER, "Answer")
        assertEquals(NotificationMirror.LABEL_DECLINE, "Decline")
        assertEquals(NotificationMirror.LABEL_MUTE, "Mute")
        assertEquals(CallAlertController.Action.Answer, CallAlertController.actionFromLabel("Answer"))
        assertEquals(CallAlertController.Action.Decline, CallAlertController.actionFromLabel("Decline"))
        assertEquals(CallAlertController.Action.Mute, CallAlertController.actionFromLabel("Mute"))
        assertNull(CallAlertController.actionFromLabel("Hang up"))
    }

    @Test
    fun mediaCommandsUseStableMethodNames() {
        assertEquals(MediaBridgeController.Command.Play, MediaBridgeController.commandFromLabel("Play"))
        assertEquals(MediaBridgeController.Command.Pause, MediaBridgeController.commandFromLabel("Pause"))
        assertEquals(MediaBridgeController.Command.Next, MediaBridgeController.commandFromLabel("Next"))
        assertEquals("Previous", MediaBridgeController.methodFor(MediaBridgeController.Command.Previous))
        assertNull(MediaBridgeController.commandFromLabel("Stop"))
    }
}
