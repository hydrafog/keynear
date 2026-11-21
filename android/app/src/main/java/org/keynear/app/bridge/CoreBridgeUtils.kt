package org.keynear.app.bridge

import android.content.ContentValues
import android.content.Context
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.os.ParcelFileDescriptor
import android.provider.MediaStore
import org.keynear.app.ui.model.BluetoothStatus
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.CoreState
import org.keynear.app.ui.model.DaemonConfigUi
import org.keynear.app.ui.model.DiscoveredPeerUi
import org.keynear.app.ui.model.LocalNetworkStatus
import org.keynear.app.ui.model.PendingPairingUi
import org.keynear.app.ui.model.SendItemKind
import org.keynear.app.ui.model.SendQueueItemUi
import org.keynear.app.ui.model.ShareValidationResult
import org.keynear.app.ui.model.TransferStateUi
import org.keynear.app.ui.model.TransferUi
import org.keynear.app.ui.model.VisibilityState
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong

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

private const val BYTES_PER_UNIT = 1024.0
private const val BYTES_PER_UNIT_LONG = 1024L

fun formatByteCount(bytes: Long): String {
    val safe = bytes.coerceAtLeast(0L)
    if (safe < BYTES_PER_UNIT_LONG) return "$safe B"
    val kib = safe / BYTES_PER_UNIT
    if (kib < BYTES_PER_UNIT) return "%.1f KB".format(kib)
    val mib = kib / BYTES_PER_UNIT
    if (mib < BYTES_PER_UNIT) return "%.1f MB".format(mib)
    val gib = mib / BYTES_PER_UNIT
    return "%.2f GB".format(gib)
}

fun evaluateConsentRequirement(
    isPaired: Boolean,
    deviceAutoAccept: Boolean,
    globalAutoAccept: Boolean,
): Boolean = isPaired && (deviceAutoAccept || globalAutoAccept)
object EndpointHexCodec {
    private const val ENDPOINT_ID_HEX_LEN = 64
    private const val ENDPOINT_BYTES = 32
    private const val HEX_RADIX = 16
    private const val BYTE_MASK = 0xFF

    fun decode32(hex: String): ByteArray? {
        val trimmed = hex.trim()
        if (trimmed.length != ENDPOINT_ID_HEX_LEN) return null
        val out = ByteArray(ENDPOINT_BYTES)
        for (i in 0 until ENDPOINT_BYTES) {
            val hi = Character.digit(trimmed[i * 2], HEX_RADIX)
            val lo = Character.digit(trimmed[i * 2 + 1], HEX_RADIX)
            if (hi < 0 || lo < 0) return null
            out[i] = ((hi shl 4) or lo).toByte()
        }
        return out
    }

    fun encode(bytes: ByteArray): String =
        bytes.joinToString(separator = "") { b -> "%02x".format(b.toInt() and BYTE_MASK) }
}

const val MAX_CLIPBOARD_IMAGE_BYTES: Int = 16 * 1024 * 1024

private val BUILT_IN_IGNORED_CLIPBOARD_MIMES = setOf(
    "x-kde-passwordmanagerhint",
    "application/x-password",
)

fun normalizeClipboardMime(raw: String): String =
    raw.substringBefore(';').trim().lowercase()

fun shouldIgnoreClipboardMimes(
    offeredMimes: List<String>,
    ignoredMimes: List<String>,
): Boolean {
    val rules = buildSet {
        addAll(BUILT_IN_IGNORED_CLIPBOARD_MIMES)
        for (rule in ignoredMimes) {
            val clean = normalizeClipboardMime(rule)
            if (clean.isNotEmpty()) add(clean)
        }
    }
    return offeredMimes.any { mime ->
        val norm = normalizeClipboardMime(mime)
        norm.isNotEmpty() && norm in rules
    }
}

fun shouldSkipClipDescription(
    offeredMimes: List<String>,
    isSensitive: Boolean,
    ignoredMimes: List<String>,
): Boolean = isSensitive || shouldIgnoreClipboardMimes(offeredMimes, ignoredMimes)

fun shouldReadClipboardOnFocus(hasFocus: Boolean, alreadyHandled: Boolean): Boolean =
    hasFocus && !alreadyHandled

fun usePendingIntentTileCollapse(sdkInt: Int): Boolean =
    sdkInt >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE
