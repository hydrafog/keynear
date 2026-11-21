package org.keynear.designsystem

import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

enum class TactileTone {
    SURFACE,
    TONAL,
    ACCENT,
    SHEET,
    INSET,
    THUMB,
    NONE,
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
    val backdrop: Color,
    val mapGround: Color,
    val mapBlue: Color,
    val mapRing: Color,
    val surfaceHover: Color,
    val loading: Color,
    val loadingTonal: Color,
    val onLoadingTonal: Color,
    val warningTonal: Color,
    val onWarningTonal: Color,
    val error: Color,
    val errorAction: Color,
    val onDangerAction: Color,
    val input: Color,
    val switchTrackOff: Color,
    val switchThumb: Color,
    val snackbar: Color,
    val onSnackbar: Color,
    val sheetHandle: Color,
    val gesturePill: Color,
    val trackWell: Color,
    val visibilityWell: Color,
    val scrim: Color,
    val onCanvas: Color,
    val loadingHover: Color,
    val loadingInk: Color,
    val errorInk: Color,
    val errorActionHover: Color,
    val notifSubCard: Color,
    val qsOn: Color,
    val qsHover: Color,
    val mapContour: Color,
    val mapContourFar: Color,
    val waveSpoke: Color,
    val waveRing: Color,
    val waveEcho: Color,
    val isDark: Boolean,
)

val LightKeynearPalette = KeynearPalette(
    canvas = Color(0xFFF0F4FA),
    surface = Color(0xFFFAFBFD),
    surfaceTonal = Color(0xFFE7EDF8),
    textPrimary = Color(0xFF07132B),
    textSecondary = Color(0xFF283950),
    textMuted = Color(0xFF3B4B5E),
    accent = Color(0xFF004CED),
    onAccent = Color(0xFFFFFFFE),
    focusRing = Color(0xFF0172F9),
    errorTonal = Color(0xFFFFE7E7),
    onDangerTonal = Color(0xFF8B0016),
    backdrop = Color(0xFFE2E8F2),
    mapGround = Color(0xFFE7EEF9),
    mapBlue = Color(0xFFDCE7FA),
    mapRing = Color(0x0F051131),
    surfaceHover = Color(0xFFF4F7FD),
    loading = Color(0xFF0195D4),
    loadingTonal = Color(0xFFD1ECF7),
    onLoadingTonal = Color(0xFF00425F),
    warningTonal = Color(0xFFFFF6D3),
    onWarningTonal = Color(0xFF863300),
    error = Color(0xFFE84201),
    errorAction = Color(0xFFFFE7E7),
    onDangerAction = Color(0xFF8B0016),
    input = Color(0xFFE8EEFA),
    switchTrackOff = Color(0xFFDEE6F6),
    switchThumb = Color(0xFFFFFFFE),
    snackbar = Color(0xFF0F223E),
    onSnackbar = Color(0xFFFFFFFE),
    sheetHandle = Color(0xFFCADBEE),
    gesturePill = Color(0xFF819BBB),
    trackWell = Color(0xFFE7EDF8),
    visibilityWell = Color(0xFFD2DFF4),
    scrim = Color(0x7300021E),
    onCanvas = Color(0xFF07132B),
    loadingHover = Color(0xFF0195D4),
    loadingInk = Color(0xFF0195D4),
    errorInk = Color(0xFFE84201),
    errorActionHover = Color(0xFFFFCFCF),
    notifSubCard = Color(0xFFE9EFFB),
    qsOn = Color(0xFF07132B),
    qsHover = Color(0xFFF3F7FD),
    mapContour = Color(0x33004CED),
    mapContourFar = Color(0x26004CED),
    waveSpoke = Color(0x2E94B3D9),
    waveRing = Color(0x5C38A8EE),
    waveEcho = Color(0x427DCCFA),
    isDark = false,
)

val DarkKeynearPalette = KeynearPalette(
    canvas = Color(0xFF0B1016),
    surface = Color(0xFF161821),
    surfaceTonal = Color(0xFF1E202B),
    textPrimary = Color(0xFFF7F6F7),
    textSecondary = Color(0xFFDBD8DB),
    textMuted = Color(0xFF9A9AA6),
    accent = Color(0xFF004CED),
    onAccent = Color(0xFFFFFFFE),
    focusRing = Color(0xFF0172F9),
    errorTonal = Color(0xFF3F0308),
    onDangerTonal = Color(0xFFFFCFCF),
    backdrop = Color(0xFF04070B),
    mapGround = Color(0xFF080C12),
    mapBlue = Color(0xFF070E1A),
    mapRing = Color(0x0FFFFFFE),
    surfaceHover = Color(0xFF242531),
    loading = Color(0xFF0195D4),
    loadingTonal = Color(0xFF11364C),
    onLoadingTonal = Color(0xFF99D5EE),
    warningTonal = Color(0xFF381C00),
    onWarningTonal = Color(0xFFFFEAA5),
    error = Color(0xFFE84201),
    errorAction = Color(0xFF4C040B),
    onDangerAction = Color(0xFFFFA1A2),
    input = Color(0xFF090E14),
    switchTrackOff = Color(0xFF10141C),
    switchThumb = Color(0xFFF7F6F7),
    snackbar = Color(0xFF1E202B),
    onSnackbar = Color(0xFFF7F6F7),
    sheetHandle = Color(0xFF474756),
    gesturePill = Color(0xFF474756),
    trackWell = Color(0xFF0D1018),
    visibilityWell = Color(0xFF0D1018),
    scrim = Color(0x99000000),
    onCanvas = Color(0xFFF7F6F7),
    loadingHover = Color(0xFF0195D4),
    loadingInk = Color(0xFF0195D4),
    errorInk = Color(0xFFE84201),
    errorActionHover = Color(0xFF5E040F),
    notifSubCard = Color(0xFF0D1018),
    qsOn = Color(0xFFE9E9EC),
    qsHover = Color(0xFF1D1F2A),
    mapContour = Color(0x334199FF),
    mapContourFar = Color(0x264199FF),
    waveSpoke = Color(0x2E94B3D9),
    waveRing = Color(0x5C38A8EE),
    waveEcho = Color(0x427DCCFA),
    isDark = true,
)

object KeynearRadii {
    val Lg = 8.dp
    val Xl = 12.dp
    val TwoXl = 16.dp
    val ThreeXl = 24.dp
    val Md10 = 10.dp
    val Lg14 = 14.dp
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

object KeynearDimensions {
    val Dp2 = 2.dp
    val Dp5 = 5.dp
    val Dp6 = 6.dp
    val Dp10 = 10.dp
    val Dp13 = 13.dp
    val Dp17 = 17.dp
    val Dp18 = 18.dp
    val Dp20 = 20.dp
    val Dp26 = 26.dp
    val Dp34 = 34.dp
    val Dp52 = 52.dp
    val Dp58 = 58.dp
    val Dp62 = 62.dp
    val Dp128 = 128.dp
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
    val CardPaddingLarge = KeynearDimensions.Dp20
    val ActionButtonHorizontalPadding = KeynearDimensions.Dp20
    val ControlMinHeight = KeynearDimensions.Dp52
    val ContentGap = KeynearDimensions.Dp10
    val SelectionTileMinHeight = KeynearDimensions.Dp128
    val SwitchTrackWidth = KeynearDimensions.Dp58
    val SwitchTrackHeight = KeynearDimensions.Dp34
    val SwitchThumbSize = KeynearDimensions.Dp26
    val SwitchMinWidth = KeynearDimensions.Dp62
    val CompactGap = KeynearDimensions.Dp6
    val SwitchDragThreshold = KeynearDimensions.Dp18
    val SwitchThumbRadius = KeynearDimensions.Dp13
    val SwitchTrackRadius = KeynearDimensions.Dp17
    val TextSubtitleGap = KeynearDimensions.Dp2
    val ChevronIconSize = KeynearDimensions.Dp20
}

object KeynearTypeSizes {
    val Title = 24.sp
    val Heading = 20.sp
    val Body = 16.sp
    val Description = 14.sp
    val Caption = 12.sp
    val Micro = 0.8.sp
    val Sp4 = 4.sp
    val Sp11 = 11.sp
}

object KeynearLineHeights {
    val Title = 30.sp
    val Heading = 26.sp
    val Body = 24.sp
    val Description = 20.sp
    val Caption = 16.sp
}

object KeynearMotion {
    const val SETTLE_DURATION_MS = 240
    const val FADE_DURATION_MS = 180
    const val SHORT_DURATION_MS = 150
}

object KeynearOpacity {
    const val DISABLED_CONTAINER = 0.45f
    const val DISABLED_CONTENT = 0.65f
    const val WAVE_MOTE = 0.5f
    const val PRESSED_OVERLAY = 0.12f
}

object KeynearStates {
    val FocusOutlineWidth = 2.dp
    val FocusCornerRadius = KeynearRadii.Xl
}

data class KeynearOuterShadow(
    val offsetY: Dp,
    val blurRadius: Dp,
    val spread: Dp,
    val color: Color,
)

data class KeynearInsetShadow(
    val offsetY: Dp,
    val blurRadius: Dp,
    val color: Color,
)

object KeynearElevation {
    val OuterSurfaceDark = listOf(
        KeynearOuterShadow(6.dp, 11.2.dp, (-3).dp, Color(0x73000000)),
        KeynearOuterShadow(2.dp, 2.6.dp, (-1).dp, Color(0x40000000)),
    )
    val OuterSurfaceLight = listOf(
        KeynearOuterShadow(5.dp, 9.5.dp, (-3).dp, Color(0x1405112F)),
        KeynearOuterShadow(2.dp, 2.6.dp, (-1).dp, Color(0x0A05112F)),
    )
    val OuterTonalDark = listOf(
        KeynearOuterShadow(4.dp, 7.8.dp, (-2).dp, Color(0x5E000000)),
    )
    val OuterTonalLight = listOf(
        KeynearOuterShadow(3.dp, 6.dp, (-2).dp, Color(0x1205112F)),
    )
    val OuterAccentDark = listOf(
        KeynearOuterShadow(6.dp, 13.dp, (-3).dp, Color(0x5E0053E2)),
    )
    val OuterAccentLight = listOf(
        KeynearOuterShadow(6.dp, 11.2.dp, (-3).dp, Color(0x52003DCF)),
    )
    val OuterSheetDark = listOf(
        KeynearOuterShadow((-8).dp, 19.5.dp, (-4).dp, Color(0x80000000)),
    )
    val OuterSheetLight = listOf(
        KeynearOuterShadow((-6).dp, 16.5.dp, (-4).dp, Color(0x1405112F)),
    )
    val OuterThumbDark = listOf(
        KeynearOuterShadow(2.dp, 4.3.dp, 0.dp, Color(0x7A000000)),
    )
    val OuterThumbLight = listOf(
        KeynearOuterShadow(2.dp, 3.6.dp, 0.dp, Color(0x3805112F)),
    )
    val OuterNone = emptyList<KeynearOuterShadow>()

    val InsetSurfaceDark = listOf(
        KeynearInsetShadow(1.5.dp, 0.9.dp, Color(0x14FFFFFE)),
        KeynearInsetShadow((-2).dp, 2.6.dp, Color(0x80000000)),
    )
    val InsetSurfaceLight = listOf(
        KeynearInsetShadow(1.5.dp, 0.9.dp, Color(0xF2FFFFFE)),
        KeynearInsetShadow((-2).dp, 1.8.dp, Color(0x0F05112F)),
    )
    val InsetTonalDark = listOf(
        KeynearInsetShadow(1.5.dp, 0.9.dp, Color(0x12FFFFFE)),
        KeynearInsetShadow((-1.5).dp, 1.8.dp, Color(0x73000000)),
    )
    val InsetTonalLight = listOf(
        KeynearInsetShadow(1.5.dp, 0.9.dp, Color(0xCCFFFFFE)),
        KeynearInsetShadow((-1.5).dp, 1.5.dp, Color(0x1405112F)),
    )
    val InsetAccentDark = listOf(
        KeynearInsetShadow(1.8.dp, 1.1.dp, Color(0x52FFFFFE)),
        KeynearInsetShadow((-2.5).dp, 2.6.dp, Color(0x80000000)),
    )
    val InsetAccentLight = listOf(
        KeynearInsetShadow(1.8.dp, 1.1.dp, Color(0x70FFFFFE)),
        KeynearInsetShadow((-2.5).dp, 2.6.dp, Color(0x61000269)),
    )
    val InsetSheetDark = listOf(
        KeynearInsetShadow(1.5.dp, 0.9.dp, Color(0x17FFFFFE)),
    )
    val InsetSheetLight = listOf(
        KeynearInsetShadow(2.dp, 1.1.dp, Color(0xF2FFFFFE)),
    )
    val InsetThumbDark = listOf(
        KeynearInsetShadow(1.5.dp, 0.9.dp, Color(0xE6FFFFFE)),
        KeynearInsetShadow((-1.5).dp, 1.2.dp, Color(0x40000000)),
    )
    val InsetThumbLight = listOf(
        KeynearInsetShadow(1.5.dp, 0.9.dp, Color(0xFFFFFFFF)),
        KeynearInsetShadow((-1.5).dp, 1.2.dp, Color(0x2605112F)),
    )
    val InsetInsetDark = listOf(
        KeynearInsetShadow(2.2.dp, 3.2.dp, Color(0x99000000)),
        KeynearInsetShadow((-1.2).dp, 0.9.dp, Color(0x1FFFFFFE)),
    )
    val InsetInsetLight = listOf(
        KeynearInsetShadow(2.2.dp, 2.6.dp, Color(0x2405112F)),
        KeynearInsetShadow((-1.2).dp, 0.9.dp, Color(0xE6FFFFFE)),
    )
    val InsetNone = emptyList<KeynearInsetShadow>()
}
