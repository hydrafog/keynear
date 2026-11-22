package org.keynear.app.notifications

object NotificationMirror {
    const val MAX_KEY_LENGTH: Int = 128
    const val MAX_PACKAGE_LENGTH: Int = 128
    const val MAX_TEXT_LENGTH: Int = 2048

    const val LABEL_REPLY: String = "Reply"
    const val LABEL_MARK_READ: String = "Mark as read"
    const val LABEL_DISMISS: String = "Dismiss"
    const val LABEL_ANSWER: String = "Answer"
    const val LABEL_DECLINE: String = "Decline"
    const val LABEL_MUTE: String = "Mute"

    const val PREFIX_REPLY: String = "mirror:reply:"
    const val PREFIX_READ: String = "mirror:read:"
    const val PREFIX_DISMISS: String = "mirror:dismiss:"
    const val PREFIX_ANSWER: String = "call:answer:"
    const val PREFIX_DECLINE: String = "call:decline:"
    const val PREFIX_MUTE: String = "call:mute:"

    data class Descriptor(
        val key: String,
        val packageName: String,
        val title: String,
        val body: String,
        val sequence: Long,
        val supportsReply: Boolean,
        val supportsMarkRead: Boolean,
        val supportsDismiss: Boolean,
    )

    fun isValidText(value: String, max: Int, allowEmpty: Boolean = false): Boolean {
        if (value.isEmpty() && !allowEmpty) return false
        return value.length <= max && value.none { it.isISOControl() }
    }

    fun isExcluded(packageName: String, exclusions: Set<String>): Boolean {
        return exclusions.contains(packageName)
    }

    fun accept(descriptor: Descriptor, exclusions: Set<String>, lastSequence: Long): Boolean {
        if (isExcluded(descriptor.packageName, exclusions)) return false
        if (!isValidText(descriptor.key, MAX_KEY_LENGTH)) return false
        if (!isValidText(descriptor.packageName, MAX_PACKAGE_LENGTH)) return false
        if (!isValidText(descriptor.title, MAX_TEXT_LENGTH, allowEmpty = true)) return false
        if (!isValidText(descriptor.body, MAX_TEXT_LENGTH, allowEmpty = true)) return false
        return descriptor.sequence > lastSequence
    }

    fun actionKey(prefix: String, key: String): String = prefix + key

    fun parseAction(actionKey: String): Pair<String, String>? {
        val prefixes = listOf(
            PREFIX_REPLY,
            PREFIX_READ,
            PREFIX_DISMISS,
            PREFIX_ANSWER,
            PREFIX_DECLINE,
            PREFIX_MUTE,
        )
        val prefix = prefixes.firstOrNull { actionKey.startsWith(it) } ?: return null
        val target = actionKey.removePrefix(prefix)
        if (target.isEmpty()) return null
        return Pair(prefix, target)
    }
}
