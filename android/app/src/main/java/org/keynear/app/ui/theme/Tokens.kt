package org.keynear.app.ui.theme

import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
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
    const val LOADING_BLUE: Long = 0xFF0195D4
    const val ERROR_ORANGE: Long = 0xFFE84201

    const val LIGHT_BACKDROP: Long = 0xFFE2E8F2
    const val LIGHT_MAP_GROUND: Long = 0xFFE7EEF9
    const val LIGHT_MAP_BLUE: Long = 0xFFDCE7FA
    const val LIGHT_SURFACE_HOVER: Long = 0xFFF4F7FD
    const val LIGHT_LOADING_TONAL: Long = 0xFFD1ECF7
    const val LIGHT_ON_LOADING_TONAL: Long = 0xFF00425F
    const val LIGHT_WARNING_TONAL: Long = 0xFFFFF6D3
    const val LIGHT_ON_WARNING_TONAL: Long = 0xFF863300
    const val LIGHT_INPUT: Long = 0xFFE8EEFA
    const val LIGHT_SWITCH_TRACK_OFF: Long = 0xFFDEE6F6
    const val LIGHT_SNACKBAR: Long = 0xFF0F223E
    const val LIGHT_ON_SNACKBAR: Long = 0xFFFFFFFE
    const val LIGHT_SHEET_HANDLE: Long = 0xFFCADBEE
    const val LIGHT_GESTURE_PILL: Long = 0xFF819BBB
    const val LIGHT_TRACK_WELL: Long = 0xFFE7EDF8
    const val LIGHT_SCRIM: Long = 0x7300021E

    const val DARK_BACKDROP: Long = 0xFF04070B
    const val DARK_MAP_GROUND: Long = 0xFF080C12
    const val DARK_MAP_BLUE: Long = 0xFF070E1A
    const val DARK_SURFACE_HOVER: Long = 0xFF242531
    const val DARK_LOADING_TONAL: Long = 0xFF11364C
    const val DARK_ON_LOADING_TONAL: Long = 0xFF99D5EE
    const val DARK_WARNING_TONAL: Long = 0xFF381C00
    const val DARK_ON_WARNING_TONAL: Long = 0xFFFFEAA5
    const val DARK_INPUT: Long = 0xFF090E14
    const val DARK_SWITCH_TRACK_OFF: Long = 0xFF10141C
    const val DARK_SNACKBAR: Long = 0xFF1E202B
    const val DARK_ON_SNACKBAR: Long = 0xFFF7F6F7
    const val DARK_SHEET_HANDLE: Long = 0xFF474756
    const val DARK_GESTURE_PILL: Long = 0xFF474756
    const val DARK_TRACK_WELL: Long = 0xFF0D1018
    const val DARK_SCRIM: Long = 0x99000000
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
    val backdrop: Color = Color(KeynearTokenHex.LIGHT_BACKDROP),
    val mapGround: Color = Color(KeynearTokenHex.LIGHT_MAP_GROUND),
    val mapBlue: Color = Color(KeynearTokenHex.LIGHT_MAP_BLUE),
    val surfaceHover: Color = Color(KeynearTokenHex.LIGHT_SURFACE_HOVER),
    val loading: Color = Color(KeynearTokenHex.LOADING_BLUE),
    val loadingTonal: Color = Color(KeynearTokenHex.LIGHT_LOADING_TONAL),
    val onLoadingTonal: Color = Color(KeynearTokenHex.LIGHT_ON_LOADING_TONAL),
    val warningTonal: Color = Color(KeynearTokenHex.LIGHT_WARNING_TONAL),
    val onWarningTonal: Color = Color(KeynearTokenHex.LIGHT_ON_WARNING_TONAL),
    val error: Color = Color(KeynearTokenHex.ERROR_ORANGE),
    val input: Color = Color(KeynearTokenHex.LIGHT_INPUT),
    val switchTrackOff: Color = Color(KeynearTokenHex.LIGHT_SWITCH_TRACK_OFF),
    val snackbar: Color = Color(KeynearTokenHex.LIGHT_SNACKBAR),
    val onSnackbar: Color = Color(KeynearTokenHex.LIGHT_ON_SNACKBAR),
    val sheetHandle: Color = Color(KeynearTokenHex.LIGHT_SHEET_HANDLE),
    val gesturePill: Color = Color(KeynearTokenHex.LIGHT_GESTURE_PILL),
    val trackWell: Color = Color(KeynearTokenHex.LIGHT_TRACK_WELL),
    val scrim: Color = Color(KeynearTokenHex.LIGHT_SCRIM),
    val errorAction: Color = Color(0xFFFFE7E7),
    val onDangerAction: Color = Color(0xFF8B0016),
    val mapRing: Color = Color(0x0F051131),
    val switchThumb: Color = Color(0xFFFFFFFE),
    val isDark: Boolean = false,
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
    backdrop = Color(KeynearTokenHex.LIGHT_BACKDROP),
    mapGround = Color(KeynearTokenHex.LIGHT_MAP_GROUND),
    mapBlue = Color(KeynearTokenHex.LIGHT_MAP_BLUE),
    surfaceHover = Color(KeynearTokenHex.LIGHT_SURFACE_HOVER),
    loading = Color(KeynearTokenHex.LOADING_BLUE),
    loadingTonal = Color(KeynearTokenHex.LIGHT_LOADING_TONAL),
    onLoadingTonal = Color(KeynearTokenHex.LIGHT_ON_LOADING_TONAL),
    warningTonal = Color(KeynearTokenHex.LIGHT_WARNING_TONAL),
    onWarningTonal = Color(KeynearTokenHex.LIGHT_ON_WARNING_TONAL),
    error = Color(KeynearTokenHex.ERROR_ORANGE),
    input = Color(KeynearTokenHex.LIGHT_INPUT),
    switchTrackOff = Color(KeynearTokenHex.LIGHT_SWITCH_TRACK_OFF),
    snackbar = Color(KeynearTokenHex.LIGHT_SNACKBAR),
    onSnackbar = Color(KeynearTokenHex.LIGHT_ON_SNACKBAR),
    sheetHandle = Color(KeynearTokenHex.LIGHT_SHEET_HANDLE),
    gesturePill = Color(KeynearTokenHex.LIGHT_GESTURE_PILL),
    trackWell = Color(KeynearTokenHex.LIGHT_TRACK_WELL),
    scrim = Color(KeynearTokenHex.LIGHT_SCRIM),
    isDark = false,
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
    backdrop = Color(KeynearTokenHex.DARK_BACKDROP),
    mapGround = Color(KeynearTokenHex.DARK_MAP_GROUND),
    mapBlue = Color(KeynearTokenHex.DARK_MAP_BLUE),
    surfaceHover = Color(KeynearTokenHex.DARK_SURFACE_HOVER),
    loading = Color(KeynearTokenHex.LOADING_BLUE),
    loadingTonal = Color(KeynearTokenHex.DARK_LOADING_TONAL),
    onLoadingTonal = Color(KeynearTokenHex.DARK_ON_LOADING_TONAL),
    warningTonal = Color(KeynearTokenHex.DARK_WARNING_TONAL),
    onWarningTonal = Color(KeynearTokenHex.DARK_ON_WARNING_TONAL),
    error = Color(KeynearTokenHex.ERROR_ORANGE),
    input = Color(KeynearTokenHex.DARK_INPUT),
    switchTrackOff = Color(KeynearTokenHex.DARK_SWITCH_TRACK_OFF),
    snackbar = Color(KeynearTokenHex.DARK_SNACKBAR),
    onSnackbar = Color(KeynearTokenHex.DARK_ON_SNACKBAR),
    sheetHandle = Color(KeynearTokenHex.DARK_SHEET_HANDLE),
    gesturePill = Color(KeynearTokenHex.DARK_GESTURE_PILL),
    trackWell = Color(KeynearTokenHex.DARK_TRACK_WELL),
    scrim = Color(KeynearTokenHex.DARK_SCRIM),
    errorAction = Color(0xFF4C040B),
    onDangerAction = Color(0xFFFFA1A2),
    mapRing = Color(0x0FFFFFFE),
    switchThumb = Color(0xFFF7F6F7),
    isDark = true,
)

object KeynearRadii {
    val Lg = 8.dp
    val Xl = 12.dp
    val TwoXl = 16.dp
    val ThreeXl = 24.dp
}

object KeynearShapes {
    val Lg = RoundedCornerShape(KeynearRadii.Lg)
    val Xl = RoundedCornerShape(KeynearRadii.Xl)
    val Card = RoundedCornerShape(KeynearRadii.TwoXl)
    val TopBar = RoundedCornerShape(bottomStart = KeynearRadii.ThreeXl, bottomEnd = KeynearRadii.ThreeXl)
    val Sheet = RoundedCornerShape(topStart = KeynearRadii.ThreeXl, topEnd = KeynearRadii.ThreeXl)
    val Pill = CircleShape
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

object KeynearLayout {
    val ScreenPaddingHorizontal = 16.dp
    val ScreenPaddingVertical = 16.dp
    val SectionGap = 24.dp
    val CompactSectionGap = 16.dp
    val GridGap = 14.dp
    val SectionHeadingBottom = 10.dp
    val SectionInnerPadding = 8.dp
    val SectionRowGap = 6.dp
    val RowHorizontalPadding = 16.dp
    val RowVerticalPadding = 10.dp
    val RowMinHeight = 66.dp
    val CardPadding = 16.dp
    val CardPaddingLarge = 20.dp
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

