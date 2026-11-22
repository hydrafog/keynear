package org.keynear.designsystem

import androidx.compose.material3.ColorScheme
import androidx.compose.ui.graphics.lerp

internal fun KeynearPalette.withMaterialAccent(scheme: ColorScheme): KeynearPalette =
    withMaterialSurfaceRoles(scheme).withMaterialAccentRoles(scheme)

private fun KeynearPalette.withMaterialSurfaceRoles(scheme: ColorScheme): KeynearPalette = copy(
    canvas = scheme.background,
    onCanvas = scheme.onBackground,
    surface = scheme.surface,
    surfaceTonal = scheme.surfaceVariant,
    surfaceHover = scheme.surfaceContainerLow,
    textPrimary = scheme.onSurface,
    textSecondary = scheme.onSurfaceVariant,
    textMuted = scheme.outline,
    errorTonal = scheme.errorContainer,
    onDangerTonal = scheme.onErrorContainer,
    backdrop = scheme.surfaceDim,
    mapGround = scheme.surfaceContainerLow,
    mapBlue = scheme.secondaryContainer,
    input = scheme.surfaceContainerHighest,
    switchTrackOff = scheme.surfaceVariant,
    switchThumb = scheme.surfaceBright,
    snackbar = scheme.inverseSurface,
    onSnackbar = scheme.inverseOnSurface,
    sheetHandle = scheme.outlineVariant,
    gesturePill = scheme.outline,
    trackWell = scheme.surfaceContainerHigh,
    visibilityWell = scheme.surfaceContainerHighest,
    scrim = scheme.scrim,
    error = scheme.error,
    errorInk = scheme.error,
    errorAction = scheme.errorContainer,
    errorActionHover = lerp(scheme.errorContainer, scheme.error, 0.18f),
    onDangerAction = scheme.onErrorContainer,
    notifSubCard = scheme.surfaceContainerLow,
    qsOn = scheme.onSurface,
    qsHover = scheme.surfaceContainerLow,
    warningTonal = scheme.tertiaryContainer,
    onWarningTonal = scheme.onTertiaryContainer,
)

private fun KeynearPalette.withMaterialAccentRoles(scheme: ColorScheme): KeynearPalette = copy(
    accent = scheme.primary,
    onAccent = scheme.onPrimary,
    focusRing = scheme.primary,
    tonalRim = scheme.outlineVariant,
    accentRim = scheme.primary.copy(alpha = if (isDark) 0.28f else 0.40f),
    loading = scheme.primary,
    loadingHover = lerp(scheme.primary, scheme.onPrimary, 0.12f),
    loadingInk = scheme.primary,
    loadingTonal = scheme.primaryContainer,
    onLoadingTonal = scheme.onPrimaryContainer,
    mapRing = scheme.primary.copy(alpha = 0.06f),
    mapContour = scheme.primary.copy(alpha = 0.20f),
    mapContourFar = scheme.primary.copy(alpha = 0.15f),
    waveSpoke = scheme.primary.copy(alpha = 0.15f),
    waveRing = scheme.primary.copy(alpha = 0.28f),
    waveEcho = scheme.primary.copy(alpha = 0.22f),
)
