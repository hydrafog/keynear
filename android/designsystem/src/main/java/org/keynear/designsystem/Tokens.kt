package org.keynear.designsystem

import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

data class KeynearAccentPreset(
    val id: String,
    val name: String,
    val hex: String,
    val color: Color,
)

object KeynearAccents {
    const val DEFAULT_HEX = "#4ABBD5"

    val Presets: List<KeynearAccentPreset> = listOf(
        KeynearAccentPreset("cyan", "Keynear cyan", "#4ABBD5", Color(0xFF4ABBD5)),
        KeynearAccentPreset("violet", "Violet", "#C594E9", Color(0xFFC594E9)),
        KeynearAccentPreset("green", "Green", "#6AC380", Color(0xFF6AC380)),
        KeynearAccentPreset("amber", "Amber", "#E79750", Color(0xFFE79750)),
        KeynearAccentPreset("rose", "Rose", "#F3898A", Color(0xFFF3898A)),
    )

    val CustomPickerSwatches: List<String> = listOf(
        "2596D4",
        "4E73DF",
        "6D5BD0",
        "9B59B6",
        "D45B90",
        "E05A5A",
        "D97336",
        "C2922B",
        "3A9E6F",
        "1E9E96",
        "3E82A8",
        "64748B",
    )
}

enum class TactileTone {
    SURFACE,
    TONAL,
    ACCENT,
    ACCENT_TONAL,
    DANGER,
    DANGER_TONAL,
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
    val surfaceRim: Color,
    val surfaceTopRim: Color,
    val tonalRim: Color,
    val accentRim: Color,
    val accentTopRim: Color,
    val dangerRim: Color,
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
    val waveMote: Color,
    val surfaceSheen: Color,
    val tonalSheen: Color,
    val sheetSheen: Color,
    val accentSheen: Color,
    val accentShift: Color,
    val accentTonalSheen: Color,
    val thumbSheen: Color,
    val dangerSheen: Color,
    val brandAuraInner: Color,
    val brandAuraMid: Color,
    val isDark: Boolean,
)

val LightKeynearPalette = KeynearPalette(
    canvas = Color(0xFFEEF5F7),
    surface = Color(0xFFF9FCFD),
    surfaceTonal = Color(0xFFE4F0F3),
    textPrimary = Color(0xFF071A21),
    textSecondary = Color(0xFF26404A),
    textMuted = Color(0xFF3A525C),
    accent = Color(0xFF4ABBD5),
    onAccent = Color(0xFFFFFFFF),
    focusRing = Color(0xFF0E748A),
    surfaceRim = Color(0x140A2C38),
    surfaceTopRim = Color(0x00000000),
    tonalRim = Color(0x170A2C38),
    accentRim = Color(0x380E748A),
    accentTopRim = Color(0x8CFFFFFF),
    dangerRim = Color(0x29CA244D),
    errorTonal = Color(0xFFFFDCE1),
    onDangerTonal = Color(0xFF64172B),
    backdrop = Color(0xFFE0EBEE),
    mapGround = Color(0xFFE3EEF1),
    mapBlue = Color(0xFFE3EEF1),
    mapRing = Color(0x0F05222B),
    surfaceHover = Color(0xFFF1F8FA),
    loading = Color(0xFF4ABBD5),
    loadingTonal = Color(0xFFD8F3F8),
    onLoadingTonal = Color(0xFF084B5B),
    warningTonal = Color(0xFFFFF6D3),
    onWarningTonal = Color(0xFF863300),
    error = Color(0xFFCA244D),
    errorAction = Color(0xFFFFDCE1),
    onDangerAction = Color(0xFF64172B),
    input = Color(0xFFE5F0F4),
    switchTrackOff = Color(0xFFD7E6EC),
    switchThumb = Color(0xFFFFFFFE),
    snackbar = Color(0xFF0F2229),
    onSnackbar = Color(0xFFFFFFFE),
    sheetHandle = Color(0xFFC5DAE0),
    gesturePill = Color(0xFF7F9AA4),
    trackWell = Color(0xFFE1EDF1),
    visibilityWell = Color(0xFFE1EDF1),
    scrim = Color(0x7300101A),
    onCanvas = Color(0xFF071A21),
    loadingHover = Color(0xFF5CC4DC),
    loadingInk = Color(0xFF0E748A),
    errorInk = Color(0xFFCA244D),
    errorActionHover = Color(0xFFFFCED6),
    notifSubCard = Color(0xFFE6F1F4),
    qsOn = Color(0xFF071A21),
    qsHover = Color(0xFFF1F8FA),
    mapContour = Color(0x382B9DB8),
    mapContourFar = Color(0x292B9DB8),
    waveSpoke = Color(0x1F0E748A),
    waveRing = Color(0x380E748A),
    waveEcho = Color(0x474ABBD5),
    waveMote = Color(0x5C0E748A),
    surfaceSheen = Color(0xEBFFFFFF),
    tonalSheen = Color(0x8CFFFFFF),
    sheetSheen = Color(0xF2FFFFFF),
    accentSheen = Color(0x94CDFAFF),
    accentShift = Color(0x6B60B2F0),
    accentTonalSheen = Color(0x00000000),
    thumbSheen = Color(0xE6FFFFFF),
    dangerSheen = Color(0x73FFD6DE),
    brandAuraInner = Color(0x3D4ABBD5),
    brandAuraMid = Color(0x125CBEE8),
    isDark = false,
)

val DarkKeynearPalette = KeynearPalette(
    canvas = Color(0xFF0D1115),
    surface = Color(0xFF141A20),
    surfaceTonal = Color(0xFF1C242C),
    textPrimary = Color(0xFFF4F8F9),
    textSecondary = Color(0xFFD6DEE1),
    textMuted = Color(0xFF97A5AD),
    accent = Color(0xFF4ABBD5),
    onAccent = Color(0xFFFFFFFF),
    focusRing = Color(0xFF4ABBD5),
    surfaceRim = Color(0x0FFFFFFF),
    surfaceTopRim = Color(0x14FFFFFF),
    tonalRim = Color(0x14FFFFFF),
    accentRim = Color(0x47AAF2FF),
    accentTopRim = Color(0x80FFFFFF),
    dangerRim = Color(0x38FF949D),
    errorTonal = Color(0xFF3A141E),
    onDangerTonal = Color(0xFFFED2E1),
    backdrop = Color(0xFF06090B),
    mapGround = Color(0xFF0D1115),
    mapBlue = Color(0xFF0D1115),
    mapRing = Color(0x0FF4FAFC),
    surfaceHover = Color(0xFF212B35),
    loading = Color(0xFF4ABBD5),
    loadingTonal = Color(0xFF132C35),
    onLoadingTonal = Color(0xFF9BE6F5),
    warningTonal = Color(0xFF381C00),
    onWarningTonal = Color(0xFFFFEAA5),
    error = Color(0xFFB81D46),
    errorAction = Color(0xFF3A141E),
    onDangerAction = Color(0xFFFED2E1),
    input = Color(0xFF0A0E12),
    switchTrackOff = Color(0xFF10161C),
    switchThumb = Color(0xFFF4F8F9),
    snackbar = Color(0xFF1B2229),
    onSnackbar = Color(0xFFF4F8F9),
    sheetHandle = Color(0xFF3F4D56),
    gesturePill = Color(0xFF3F4D56),
    trackWell = Color(0xFF0A0E12),
    visibilityWell = Color(0xFF0A0E12),
    scrim = Color(0x99000000),
    onCanvas = Color(0xFFF4F8F9),
    loadingHover = Color(0xFF5CC4DC),
    loadingInk = Color(0xFF4ABBD5),
    errorInk = Color(0xFFFF949D),
    errorActionHover = Color(0xFF4A1826),
    notifSubCard = Color(0xFF0E1318),
    qsOn = Color(0xFFE9EEF0),
    qsHover = Color(0xFF1B232A),
    mapContour = Color(0x384ABBD5),
    mapContourFar = Color(0x294ABBD5),
    waveSpoke = Color(0x1F9BE6F5),
    waveRing = Color(0x614ABBD5),
    waveEcho = Color(0x389BE6F5),
    waveMote = Color(0x709BE6F5),
    surfaceSheen = Color(0x0DFFFFFF),
    tonalSheen = Color(0x0AFFFFFF),
    sheetSheen = Color(0x0FFFFFFF),
    accentSheen = Color(0x7AD7FCFF),
    accentShift = Color(0x5C5CB6F0),
    accentTonalSheen = Color(0x00000000),
    thumbSheen = Color(0xB3FFFFFF),
    dangerSheen = Color(0x3DFFAAB6),
    brandAuraInner = Color(0x334ABBD5),
    brandAuraMid = Color(0x0D4ABBD5),
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
    val Dp1 = 1.dp
    val Dp2 = 2.dp
    val Dp3 = 3.dp
    val Dp4 = 4.dp
    val Dp5 = 5.dp
    val Dp6 = 6.dp
    val Dp10 = 10.dp
    val Dp11 = 11.dp
    val Dp12 = 12.dp
    val Dp13 = 13.dp
    val Dp14 = 14.dp
    val Dp16 = 16.dp
    val Dp17 = 17.dp
    val Dp18 = 18.dp
    val Dp20 = 20.dp
    val Dp22 = 22.dp
    val Dp24 = 24.dp
    val Dp26 = 26.dp
    val Dp32 = 32.dp
    val Dp34 = 34.dp
    val Dp44 = 44.dp
    val Dp46 = 46.dp
    val Dp48 = 48.dp
    val Dp52 = 52.dp
    val Dp58 = 58.dp
    val Dp62 = 62.dp
    val Dp128 = 128.dp
    val Dp132 = 132.dp
    val Dp240 = 240.dp
    val Dp290 = 290.dp
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
    val CompactButtonMinHeight = KeynearDimensions.Dp46
    val ContentGap = KeynearDimensions.Dp10
    val SelectionTileMinHeight = KeynearDimensions.Dp128
    val SwitchTrackWidth = KeynearDimensions.Dp58
    val SwitchTrackHeight = KeynearDimensions.Dp34
    val SwitchThumbSize = KeynearDimensions.Dp26
    val SwitchMinWidth = KeynearDimensions.Dp62
    val SwitchThumbTravel = KeynearSpacing.Xl
    val SwitchDragSlop = KeynearDimensions.Dp3
    val CompactGap = KeynearDimensions.Dp6
    val ProgressBarHeight = KeynearDimensions.Dp10
    val SwitchDragThreshold = KeynearDimensions.Dp18
    val SwitchThumbRadius = KeynearDimensions.Dp13
    val SwitchTrackRadius = KeynearDimensions.Dp17
    val TextSubtitleGap = KeynearDimensions.Dp2
    val ChevronIconSize = KeynearDimensions.Dp20
    val SheetCollapsedHeight = KeynearDimensions.Dp132
    val SheetExpandedHeight = KeynearDimensions.Dp240
    val RadarAuraSize = KeynearDimensions.Dp290
    val EmptyBadgeSize = KeynearDimensions.Dp44
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
    val Sp19 = 19.sp
}

object KeynearLineHeights {
    val Title = 30.sp
    val Heading = 26.sp
    val Body = 24.sp
    val Description = 20.sp
    val Caption = 16.sp
}

object KeynearFractions {
    const val HALF = 0.5f
}

object KeynearPressScale {
    const val PRESS_95 = 0.95f
    const val PRESS_97 = 0.97f
    const val PRESS_98 = 0.98f
    const val PRESS_99 = 0.99f
}

object KeynearMotion {
    const val SETTLE_DURATION_MS = 240
    const val FADE_DURATION_MS = 180
    const val SHORT_DURATION_MS = 150
    const val PRESS_DURATION_MS = 180
    const val FADE_IN_DURATION_MS = 200
    const val EXIT_DURATION_MS = 220
    const val MODAL_DURATION_MS = 260
    const val SCREEN_DURATION_MS = 280
    const val SHEET_DURATION_MS = 300
    const val MORPH_DURATION_MS = 320
    const val PIN_ENTER_DURATION_MS = 340
    const val SWITCH_DURATION_MS = 360
    const val DURATION_500_MS = 500
    const val RADAR_RAMP_MS = 800
    const val PIN_STAGGER_MS = 55
    const val WAVE_CYCLE_MS = 120_000
    const val ORBIT_CYCLE_MS = 40_000
    const val CLICK_SUPPRESS_MS = 80L

    val SpringEasing = CubicBezierEasing(0.22f, 1f, 0.36f, 1f)
    val TactileEasing = CubicBezierEasing(0.16f, 1f, 0.3f, 1f)
    val SettleEasing = CubicBezierEasing(0.05f, 0.7f, 0.1f, 1f)
    val FadeEasing = CubicBezierEasing(0.2f, 0f, 0f, 1f)
    val ExitEasing = CubicBezierEasing(0.4f, 0f, 1f, 1f)
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
        KeynearOuterShadow(6.dp, 18.dp, (-6).dp, Color(0x8C000000)),
    )
    val OuterSurfaceLight = listOf(
        KeynearOuterShadow(2.dp, 8.dp, (-2).dp, Color(0x1207232E)),
        KeynearOuterShadow(1.dp, 2.dp, 0.dp, Color(0x0A07232E)),
    )
    val OuterTonalDark = listOf(
        KeynearOuterShadow(2.dp, 8.dp, (-2).dp, Color(0x6B000000)),
    )
    val OuterTonalLight = listOf(
        KeynearOuterShadow(1.dp, 3.dp, 0.dp, Color(0x0D07232E)),
    )
    val OuterAccentDark = listOf(
        KeynearOuterShadow(8.dp, 24.dp, (-8).dp, Color(0x7A4ABBD5)),
    )
    val OuterAccentLight = listOf(
        KeynearOuterShadow(6.dp, 18.dp, (-6).dp, Color(0x854ABBD5)),
        KeynearOuterShadow(1.dp, 2.dp, 0.dp, Color(0x14051F27)),
    )
    val OuterSheetDark = listOf(
        KeynearOuterShadow((-8).dp, 24.dp, (-4).dp, Color(0x99000000)),
    )
    val OuterSheetLight = listOf(
        KeynearOuterShadow((-6).dp, 20.dp, (-4).dp, Color(0x1407232E)),
    )
    val OuterThumbDark = listOf(
        KeynearOuterShadow(1.dp, 4.dp, 0.dp, Color(0x8C000000)),
    )
    val OuterThumbLight = listOf(
        KeynearOuterShadow(1.dp, 3.dp, 0.dp, Color(0x4707232E)),
    )
    val OuterNone = emptyList<KeynearOuterShadow>()

    val InsetSurfaceDark = listOf(
        KeynearInsetShadow(1.dp, 0.dp, Color(0x14FFFFFF)),
    )
    val InsetSurfaceLight = emptyList<KeynearInsetShadow>()
    val InsetTonalDark = emptyList<KeynearInsetShadow>()
    val InsetTonalLight = emptyList<KeynearInsetShadow>()
    val InsetAccentDark = listOf(
        KeynearInsetShadow(1.dp, 0.dp, Color(0x80FFFFFF)),
    )
    val InsetAccentLight = listOf(
        KeynearInsetShadow(1.dp, 0.dp, Color(0x8CFFFFFF)),
    )
    val InsetSheetDark = emptyList<KeynearInsetShadow>()
    val InsetSheetLight = emptyList<KeynearInsetShadow>()
    val InsetThumbDark = emptyList<KeynearInsetShadow>()
    val InsetThumbLight = listOf(
        KeynearInsetShadow(1.dp, 0.dp, Color(0xE6FFFFFF)),
    )
    val InsetInsetDark = listOf(
        KeynearInsetShadow(1.dp, 3.dp, Color(0x80000000)),
    )
    val InsetInsetLight = listOf(
        KeynearInsetShadow(1.dp, 2.dp, Color(0x1A07232E)),
    )
    val InsetNone = emptyList<KeynearInsetShadow>()
}
