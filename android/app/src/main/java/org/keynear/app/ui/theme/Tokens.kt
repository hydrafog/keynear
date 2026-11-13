package org.keynear.app.ui.theme

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

object KeynearTokenHex {
    const val LIGHT_CANVAS: Long = 0xFFF0F4FA
    const val LIGHT_SURFACE: Long = 0xFFFAFBFD
    const val LIGHT_SURFACE_TONAL: Long = 0xFFE7EDF8
    const val LIGHT_TEXT_PRIMARY: Long = 0xFF07132B
    const val LIGHT_TEXT_SECONDARY: Long = 0xFF283950
    const val LIGHT_TEXT_MUTED: Long = 0xFF3B4B5E
    const val LIGHT_ERROR_TONAL: Long = 0xFFFFE7E7
    const val LIGHT_ON_DANGER_TONAL: Long = 0xFF8B0016

    const val DARK_CANVAS: Long = 0xFF0B1016
    const val DARK_SURFACE: Long = 0xFF161821
    const val DARK_SURFACE_TONAL: Long = 0xFF1E202B
    const val DARK_TEXT_PRIMARY: Long = 0xFFF7F6F7
    const val DARK_TEXT_SECONDARY: Long = 0xFFDBD8DB
    const val DARK_TEXT_MUTED: Long = 0xFF9A9AA6
    const val DARK_ERROR_TONAL: Long = 0xFF3F0308
    const val DARK_ON_DANGER_TONAL: Long = 0xFFFFCFCF

    const val ACCENT_BLUE_600: Long = 0xFF004CED
    const val ON_ACCENT_WHITE: Long = 0xFFFFFFFE
    const val FOCUS_BLUE_500: Long = 0xFF0172F9
}

data class KeynearPalette(
    val canvas: Color,
    val surface: Color,
    val surfaceTonal: Color,
    val textPrimary: Color,
    val textSecondary: Color,
    val textMuted: Color,
    val accent: Color,
    val onAccent: Color,
    val focusRing: Color,
    val errorTonal: Color,
    val onDangerTonal: Color,
)

val LightKeynearPalette = KeynearPalette(
    canvas = Color(KeynearTokenHex.LIGHT_CANVAS),
    surface = Color(KeynearTokenHex.LIGHT_SURFACE),
    surfaceTonal = Color(KeynearTokenHex.LIGHT_SURFACE_TONAL),
    textPrimary = Color(KeynearTokenHex.LIGHT_TEXT_PRIMARY),
    textSecondary = Color(KeynearTokenHex.LIGHT_TEXT_SECONDARY),
    textMuted = Color(KeynearTokenHex.LIGHT_TEXT_MUTED),
    accent = Color(KeynearTokenHex.ACCENT_BLUE_600),
    onAccent = Color(KeynearTokenHex.ON_ACCENT_WHITE),
    focusRing = Color(KeynearTokenHex.FOCUS_BLUE_500),
    errorTonal = Color(KeynearTokenHex.LIGHT_ERROR_TONAL),
    onDangerTonal = Color(KeynearTokenHex.LIGHT_ON_DANGER_TONAL),
)

val DarkKeynearPalette = KeynearPalette(
    canvas = Color(KeynearTokenHex.DARK_CANVAS),
    surface = Color(KeynearTokenHex.DARK_SURFACE),
    surfaceTonal = Color(KeynearTokenHex.DARK_SURFACE_TONAL),
    textPrimary = Color(KeynearTokenHex.DARK_TEXT_PRIMARY),
    textSecondary = Color(KeynearTokenHex.DARK_TEXT_SECONDARY),
    textMuted = Color(KeynearTokenHex.DARK_TEXT_MUTED),
    accent = Color(KeynearTokenHex.ACCENT_BLUE_600),
    onAccent = Color(KeynearTokenHex.ON_ACCENT_WHITE),
    focusRing = Color(KeynearTokenHex.FOCUS_BLUE_500),
    errorTonal = Color(KeynearTokenHex.DARK_ERROR_TONAL),
    onDangerTonal = Color(KeynearTokenHex.DARK_ON_DANGER_TONAL),
)

object KeynearRadii {
    val Lg = 8.dp
    val Xl = 12.dp
    val TwoXl = 16.dp
    val ThreeXl = 24.dp
}

object KeynearSpacing {
    val Xs = 4.dp
    val Sm = 8.dp
    val Md = 12.dp
    val Lg = 16.dp
    val Xl = 24.dp
    val TwoXl = 32.dp
    const val MIN_TOUCH_TARGET_DP = 48
    val MinTouchTarget = MIN_TOUCH_TARGET_DP.dp
}

object KeynearTypeSizes {
    val Title = 24.sp
    val Heading = 20.sp
    val Body = 16.sp
    val Description = 14.sp
    val Caption = 12.sp
}

object KeynearMotion {
    const val SETTLE_DURATION_MS = 240
    const val FADE_DURATION_MS = 180
    const val SHORT_DURATION_MS = 150
}

