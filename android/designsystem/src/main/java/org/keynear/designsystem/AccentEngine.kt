package org.keynear.designsystem

import androidx.compose.ui.graphics.Color
import kotlin.math.atan2
import kotlin.math.cbrt
import kotlin.math.cos
import kotlin.math.pow
import kotlin.math.sin
import kotlin.math.sqrt

private const val SRGB_LINEAR_THRESHOLD = 0.04045f
private const val SRGB_LINEAR_DIVISOR = 12.92f
private const val SRGB_GAMMA_OFFSET = 0.055f
private const val SRGB_GAMMA_DIVISOR = 1.055f
private const val SRGB_GAMMA = 2.4f
private const val DEG_TO_RAD = (Math.PI / 180.0).toFloat()
private const val RAD_TO_DEG = (180.0 / Math.PI).toFloat()
private const val FULL_CIRCLE_DEG = 360f

private const val GRAD_TOP_L_DELTA = 0.03f
private const val GRAD_TOP_C_FACTOR = 1.04f
private const val GRAD_BOT_L_DELTA = -0.04f
private const val GRAD_BOT_C_FACTOR = 1.08f
private const val GRAD_BOT_H_DELTA = 6f

private const val LIGHT_INK_L_DELTA = -0.22f
private const val LIGHT_ON_TONAL_L_DELTA = -0.34f
private const val DARK_ON_TONAL_L_DELTA = 0.14f
private const val DARK_ON_TONAL_C_FACTOR = 0.85f
private const val ACCENT_RIM_L_DELTA = -0.10f
private const val ACCENT_HOVER_L_DELTA = 0.04f

private const val TONAL_MIX_AMOUNT = 0.16f
private const val LIGHT_RIM_ALPHA = 0.24f
private const val DARK_RIM_ALPHA = 0.22f
private const val CONTOUR_ALPHA = 0.22f
private const val CONTOUR_FAR_ALPHA = 0.16f
private const val SPOKE_ALPHA = 0.12f
private const val LIGHT_RING_ALPHA = 0.22f
private const val LIGHT_ECHO_ALPHA = 0.28f
private const val LIGHT_MOTE_ALPHA = 0.36f
private const val DARK_RING_ALPHA = 0.38f
private const val DARK_ECHO_ALPHA = 0.22f
private const val DARK_MOTE_ALPHA = 0.44f
private const val LIGHT_AURA_INNER_ALPHA = 0.24f
private const val LIGHT_AURA_MID_ALPHA = 0.07f
private const val DARK_AURA_INNER_ALPHA = 0.20f
private const val DARK_AURA_MID_ALPHA = 0.05f
private const val HALF_MIX = 0.5f
private const val LIGHT_WAVE_INK_MIX = 0.45f

private const val HEX_LEN = 6
private const val HEX_RADIX = 16
private const val CHANNEL_MAX = 255f

data class Oklch(
    val l: Float,
    val c: Float,
    val hDeg: Float,
)

private fun srgbToLinear(c: Float): Float =
    if (c <= SRGB_LINEAR_THRESHOLD) {
        c / SRGB_LINEAR_DIVISOR
    } else {
        ((c + SRGB_GAMMA_OFFSET) / SRGB_GAMMA_DIVISOR).pow(SRGB_GAMMA)
    }

private fun linearToSrgb(c: Float): Float {
    val clamped = c.coerceIn(0f, 1f)
    return if (clamped <= 0.0031308f) {
        SRGB_LINEAR_DIVISOR * clamped
    } else {
        SRGB_GAMMA_DIVISOR * clamped.pow(1f / SRGB_GAMMA) - SRGB_GAMMA_OFFSET
    }
}

fun Color.toOklch(): Oklch {
    val r = srgbToLinear(red)
    val g = srgbToLinear(green)
    val b = srgbToLinear(blue)

    val l = cbrt(0.41222147f * r + 0.53633255f * g + 0.05144599f * b)
    val m = cbrt(0.2119035f * r + 0.6806995f * g + 0.10739696f * b)
    val s = cbrt(0.08830246f * r + 0.28171885f * g + 0.6299787f * b)

    val okL = 0.21045426f * l + 0.7936178f * m - 0.004072047f * s
    val okA = 1.9779985f * l - 2.4285922f * m + 0.4505937f * s
    val okB = 0.025904037f * l + 0.78277177f * m - 0.80867577f * s

    val chroma = sqrt(okA * okA + okB * okB)
    val hue = (atan2(okB, okA) * RAD_TO_DEG + FULL_CIRCLE_DEG) % FULL_CIRCLE_DEG
    return Oklch(okL.coerceIn(0f, 1f), chroma.coerceAtLeast(0f), hue)
}

fun Oklch.toColor(alpha: Float = 1f): Color {
    val hRad = ((hDeg % FULL_CIRCLE_DEG + FULL_CIRCLE_DEG) % FULL_CIRCLE_DEG) * DEG_TO_RAD
    val okA = c * cos(hRad)
    val okB = c * sin(hRad)

    val lRoot = l + 0.39633778f * okA + 0.21580376f * okB
    val mRoot = l - 0.105561346f * okA - 0.06385417f * okB
    val sRoot = l - 0.08948418f * okA - 1.2914855f * okB

    val lLin = lRoot * lRoot * lRoot
    val mLin = mRoot * mRoot * mRoot
    val sLin = sRoot * sRoot * sRoot

    val rLin = +4.0767417f * lLin - 3.3077116f * mLin + 0.23096994f * sLin
    val gLin = -1.268438f * lLin + 2.6097574f * mLin - 0.34131938f * sLin
    val bLin = -0.0041960863f * lLin - 0.7034186f * mLin + 1.7076147f * sLin

    return Color(
        red = linearToSrgb(rLin).coerceIn(0f, 1f),
        green = linearToSrgb(gLin).coerceIn(0f, 1f),
        blue = linearToSrgb(bLin).coerceIn(0f, 1f),
        alpha = alpha.coerceIn(0f, 1f),
    )
}

fun mixSrgb(from: Color, to: Color, amount: Float): Color {
    val t = amount.coerceIn(0f, 1f)
    return Color(
        red = (from.red + (to.red - from.red) * t).coerceIn(0f, 1f),
        green = (from.green + (to.green - from.green) * t).coerceIn(0f, 1f),
        blue = (from.blue + (to.blue - from.blue) * t).coerceIn(0f, 1f),
        alpha = (from.alpha + (to.alpha - from.alpha) * t).coerceIn(0f, 1f),
    )
}

fun mixOklch(accent: Color, base: Color, accentWeight: Float): Color {
    val w = accentWeight.coerceIn(0f, 1f)
    val a = accent.toOklch()
    val b = base.toOklch()

    val aRad = a.hDeg * DEG_TO_RAD
    val bRad = b.hDeg * DEG_TO_RAD
    val okL = b.l + (a.l - b.l) * w
    val okA = (b.c * cos(bRad)) * (1f - w) + (a.c * cos(aRad)) * w
    val okB = (b.c * sin(bRad)) * (1f - w) + (a.c * sin(aRad)) * w
    val c = sqrt(okA * okA + okB * okB)
    val h = (atan2(okB, okA) * RAD_TO_DEG + FULL_CIRCLE_DEG) % FULL_CIRCLE_DEG
    return Oklch(okL.coerceIn(0f, 1f), c, h).toColor()
}

internal fun chromaGradientPair(baseColor: Color): Pair<Color, Color> {
    val ok = baseColor.toOklch()
    val top = Oklch(
        l = (ok.l + GRAD_TOP_L_DELTA).coerceIn(0f, 1f),
        c = ok.c * GRAD_TOP_C_FACTOR,
        hDeg = ok.hDeg,
    ).toColor()
    val bottom = Oklch(
        l = (ok.l + GRAD_BOT_L_DELTA).coerceIn(0f, 1f),
        c = ok.c * GRAD_BOT_C_FACTOR,
        hDeg = ok.hDeg + GRAD_BOT_H_DELTA,
    ).toColor()
    return top to bottom
}

fun parseAccentHexOrNull(raw: String): Color? {
    val clean = raw.trim().removePrefix("#")
    if (clean.length != HEX_LEN) return null
    val r = clean.substring(0, 2).toIntOrNull(HEX_RADIX) ?: return null
    val g = clean.substring(2, 4).toIntOrNull(HEX_RADIX) ?: return null
    val b = clean.substring(4, 6).toIntOrNull(HEX_RADIX) ?: return null
    return Color(r / CHANNEL_MAX, g / CHANNEL_MAX, b / CHANNEL_MAX, 1f)
}

fun normalizeAccentHex(raw: String): String {
    val clean = raw.trim().removePrefix("#").uppercase()
    return if (clean.length == HEX_LEN && clean.all { it in '0'..'9' || it in 'A'..'F' }) {
        "#$clean"
    } else {
        KeynearAccents.DEFAULT_HEX
    }
}

private data class DerivedWaveColors(
    val spoke: Color,
    val ring: Color,
    val echo: Color,
    val mote: Color,
    val auraInner: Color,
    val auraMid: Color,
)

private fun deriveWaveColors(fill: Color, textPrimary: Color, isDark: Boolean): DerivedWaveColors {
    val waveLightMix = mixSrgb(fill, Color.White, HALF_MIX)
    val waveInkMix = mixSrgb(fill, textPrimary, LIGHT_WAVE_INK_MIX)
    val base = if (isDark) waveLightMix else waveInkMix
    val moteAlpha = if (isDark) DARK_MOTE_ALPHA else LIGHT_MOTE_ALPHA
    val auraInnerAlpha = if (isDark) DARK_AURA_INNER_ALPHA else LIGHT_AURA_INNER_ALPHA
    val auraMidAlpha = if (isDark) DARK_AURA_MID_ALPHA else LIGHT_AURA_MID_ALPHA
    return DerivedWaveColors(
        spoke = base.copy(alpha = SPOKE_ALPHA),
        ring = if (isDark) fill.copy(alpha = DARK_RING_ALPHA) else waveInkMix.copy(alpha = LIGHT_RING_ALPHA),
        echo = if (isDark) waveLightMix.copy(alpha = DARK_ECHO_ALPHA) else fill.copy(alpha = LIGHT_ECHO_ALPHA),
        mote = base.copy(alpha = moteAlpha),
        auraInner = fill.copy(alpha = auraInnerAlpha),
        auraMid = fill.copy(alpha = auraMidAlpha),
    )
}

private fun deriveInkAndTonal(fill: Color, ok: Oklch, isDark: Boolean): Triple<Color, Color, Color> {
    val ink = if (isDark) {
        fill
    } else {
        Oklch((ok.l + LIGHT_INK_L_DELTA).coerceIn(0f, 1f), ok.c, ok.hDeg).toColor()
    }
    val onTonal = if (isDark) {
        Oklch((ok.l + DARK_ON_TONAL_L_DELTA).coerceIn(0f, 1f), ok.c * DARK_ON_TONAL_C_FACTOR, ok.hDeg).toColor()
    } else {
        Oklch((ok.l + LIGHT_ON_TONAL_L_DELTA).coerceIn(0f, 1f), ok.c, ok.hDeg).toColor()
    }
    val rim = if (isDark) {
        fill.copy(alpha = DARK_RIM_ALPHA)
    } else {
        Oklch((ok.l + ACCENT_RIM_L_DELTA).coerceIn(0f, 1f), ok.c, ok.hDeg).toColor(alpha = LIGHT_RIM_ALPHA)
    }
    return Triple(ink, onTonal, rim)
}

fun KeynearPalette.withAccentHex(accentHex: String): KeynearPalette {
    val fill = parseAccentHexOrNull(accentHex) ?: return this
    val ok = fill.toOklch()
    val (ink, onTonal, rim) = deriveInkAndTonal(fill, ok, isDark)
    val tonal = mixOklch(fill, surface, TONAL_MIX_AMOUNT)
    val hover = Oklch((ok.l + ACCENT_HOVER_L_DELTA).coerceIn(0f, 1f), ok.c, ok.hDeg).toColor()
    val wave = deriveWaveColors(fill, textPrimary, isDark)
    val (gradTop, gradBot) = chromaGradientPair(fill)

    return copy(
        accent = fill,
        loading = fill,
        loadingHover = hover,
        loadingInk = ink,
        focusRing = ink,
        loadingTonal = tonal,
        onLoadingTonal = onTonal,
        accentRim = rim,
        accentSheen = gradTop,
        accentShift = gradBot,
        mapContour = fill.copy(alpha = CONTOUR_ALPHA),
        mapContourFar = fill.copy(alpha = CONTOUR_FAR_ALPHA),
        waveSpoke = wave.spoke,
        waveRing = wave.ring,
        waveEcho = wave.echo,
        waveMote = wave.mote,
        brandAuraInner = wave.auraInner,
        brandAuraMid = wave.auraMid,
    )
}

