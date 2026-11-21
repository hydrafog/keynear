package org.keynear.app.ui.model

import uniffi.keynear_core.FfiDeviceRole
import uniffi.keynear_core.FfiTransferDirection
import uniffi.keynear_core.FfiTransferState
import uniffi.keynear_core.FfiVisibilityMode

enum class VisibilityState {
    HIDDEN,
    PAIRED_ONLY,
    EVERYONE_NEARBY;

    fun next(): VisibilityState = when (this) {
        HIDDEN -> PAIRED_ONLY
        PAIRED_ONLY -> EVERYONE_NEARBY
        EVERYONE_NEARBY -> HIDDEN
    }

    fun label(): String = when (this) {
        HIDDEN -> "Hidden"
        PAIRED_ONLY -> "Paired devices only"
        EVERYONE_NEARBY -> "Everyone nearby"
    }

    fun formattedStatus(remainingSecs: Long): String = when (this) {
        HIDDEN -> "Hidden"
        PAIRED_ONLY -> "Paired devices only"
        EVERYONE_NEARBY -> {
            val clamped = remainingSecs.coerceAtLeast(0L)
            val mins = clamped / 60L
            val secs = clamped % 60L
            "Everyone (%02d:%02d)".format(mins, secs)
        }
    }

    fun toFfi(): FfiVisibilityMode = when (this) {
        HIDDEN -> FfiVisibilityMode.HIDDEN
        PAIRED_ONLY -> FfiVisibilityMode.PAIRED_ONLY
        EVERYONE_NEARBY -> FfiVisibilityMode.EVERYONE_NEARBY
    }

    companion object {
        fun fromFfi(mode: FfiVisibilityMode): VisibilityState = when (mode) {
            FfiVisibilityMode.HIDDEN -> HIDDEN
            FfiVisibilityMode.PAIRED_ONLY -> PAIRED_ONLY
            FfiVisibilityMode.EVERYONE_NEARBY -> EVERYONE_NEARBY
        }
    }
}

enum class BluetoothStatus {
    READY,
    MISSING_PERMISSION,
    ADAPTER_DISABLED,
    UNAVAILABLE,
}

enum class LocalNetworkStatus {
    READY,
    MISSING_PERMISSION,
    BLOCKED_LNP,
    UNAVAILABLE,
}

enum class DeviceRoleUi {
    DESKTOP,
    MOBILE;

    fun label(): String = when (this) {
        DESKTOP -> "Desktop"
        MOBILE -> "Mobile"
    }

    companion object {
        fun fromFfi(role: FfiDeviceRole): DeviceRoleUi = when (role) {
            FfiDeviceRole.DESKTOP -> DESKTOP
            FfiDeviceRole.MOBILE -> MOBILE
        }
    }
}

enum class TransferDirectionUi {
    SEND,
    RECEIVE;

    companion object {
        fun fromFfi(dir: FfiTransferDirection): TransferDirectionUi = when (dir) {
            FfiTransferDirection.SEND -> SEND
            FfiTransferDirection.RECEIVE -> RECEIVE
        }
    }
}

enum class TransferStateUi {
    PENDING_CONSENT,
    ACTIVE,
    COMPLETED,
    CANCELLED,
    FAILED;

    fun isTerminal(): Boolean = this == COMPLETED || this == CANCELLED || this == FAILED

    fun label(): String = when (this) {
        PENDING_CONSENT -> "Awaiting consent"
        ACTIVE -> "Transferring"
        COMPLETED -> "Completed"
        CANCELLED -> "Cancelled"
        FAILED -> "Failed"
    }

    companion object {
        fun fromFfi(state: FfiTransferState): TransferStateUi = when (state) {
            FfiTransferState.PENDING_CONSENT -> PENDING_CONSENT
            FfiTransferState.ACTIVE -> ACTIVE
            FfiTransferState.COMPLETED -> COMPLETED
            FfiTransferState.CANCELLED -> CANCELLED
            FfiTransferState.FAILED -> FAILED
        }
    }
}

enum class SendItemKind {
    FILE,
    FOLDER,
    CLIPBOARD,
    TEXT,
}

enum class TransferProgressStage {
    PREPARE,
    TRANSFER,
    COMPLETE,
}
