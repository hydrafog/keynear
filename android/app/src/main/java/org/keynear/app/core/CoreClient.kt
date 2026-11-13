package org.keynear.app.core

import uniffi.keynear_core.CoreHandle
import uniffi.keynear_core.FfiVisibilityMode
import uniffi.keynear_core.defaultConfigSummary

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

    companion object {
        fun fromFfi(mode: FfiVisibilityMode): VisibilityState = when (mode) {
            FfiVisibilityMode.HIDDEN -> HIDDEN
            FfiVisibilityMode.PAIRED_ONLY -> PAIRED_ONLY
            FfiVisibilityMode.EVERYONE_NEARBY -> EVERYONE_NEARBY
        }
    }
}

data class CoreSnapshot(
    val version: String,
    val maxControlFrameBytes: Long,
    val ready: Boolean,
    val visibility: VisibilityState,
    val configSummary: String,
)

sealed interface CoreState {
    data object Loading : CoreState
    data class Empty(val snapshot: CoreSnapshot) : CoreState
    data class Ready(val snapshot: CoreSnapshot) : CoreState
    data class Error(val message: String, val previous: CoreSnapshot? = null) : CoreState
}

sealed interface ShareValidationResult {
    data class Valid(val itemCount: Int, val hasText: Boolean) : ShareValidationResult
    data class Invalid(val reason: String) : ShareValidationResult
}

interface CoreBridge {
    fun loadState(): CoreState
    fun cycleVisibility(): CoreState
}

private val ALLOWED_SHARE_SCHEMES = setOf("content", "file")

fun validateShareIntent(
    action: String?,
    streamUris: List<String>,
    sharedText: String?,
): ShareValidationResult {
    if (action != "android.intent.action.SEND" && action != "android.intent.action.SEND_MULTIPLE") {
        return ShareValidationResult.Invalid("unsupported_action")
    }

    val normalizedText = sharedText?.trim()?.takeIf { it.isNotEmpty() }
    if (streamUris.isEmpty() && normalizedText == null) {
        return ShareValidationResult.Invalid("missing_payload")
    }

    for (rawUri in streamUris) {
        val trimmed = rawUri.trim()
        val colonIndex = trimmed.indexOf(':')
        if (colonIndex <= 0) {
            return ShareValidationResult.Invalid("missing_uri_scheme")
        }
        val scheme = trimmed.substring(0, colonIndex).lowercase()
        if (scheme !in ALLOWED_SHARE_SCHEMES) {
            return ShareValidationResult.Invalid("disallowed_uri_scheme")
        }
    }

    return ShareValidationResult.Valid(
        itemCount = streamUris.size,
        hasText = normalizedText != null,
    )
}

class UniffiCoreClient : CoreBridge {
    private val handle: CoreHandle by lazy { CoreHandle() }

    override fun loadState(): CoreState = try {
        val status = handle.status()
        val visibility = VisibilityState.fromFfi(handle.visibilityMode())
        val snapshot = CoreSnapshot(
            version = status.version,
            maxControlFrameBytes = status.maxControlFrameBytes.toLong(),
            ready = status.ready,
            visibility = visibility,
            configSummary = defaultConfigSummary(),
        )
        CoreState.Empty(snapshot)
    } catch (t: Throwable) {
        CoreState.Error(t.message ?: "Unable to initialize keynear core")
    }

    override fun cycleVisibility(): CoreState = try {
        val status = handle.status()
        val visibility = VisibilityState.fromFfi(handle.cycleVisibilityMode())
        val snapshot = CoreSnapshot(
            version = status.version,
            maxControlFrameBytes = status.maxControlFrameBytes.toLong(),
            ready = status.ready,
            visibility = visibility,
            configSummary = defaultConfigSummary(),
        )
        CoreState.Empty(snapshot)
    } catch (t: Throwable) {
        CoreState.Error(t.message ?: "Unable to cycle visibility mode")
    }
}

object CoreClientProvider {
    @Volatile
    private var instance: CoreBridge? = null

    fun get(): CoreBridge {
        val current = instance
        if (current != null) return current
        return synchronized(this) {
            instance ?: UniffiCoreClient().also { instance = it }
        }
    }
}

