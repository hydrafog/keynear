package org.keynear.app.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

val LocalKeynearPalette = staticCompositionLocalOf { LightKeynearPalette }

private val KeynearLightColorScheme = lightColorScheme(
    primary = LightKeynearPalette.accent,
    onPrimary = LightKeynearPalette.onAccent,
    background = LightKeynearPalette.canvas,
    onBackground = LightKeynearPalette.textPrimary,
    surface = LightKeynearPalette.surface,
    onSurface = LightKeynearPalette.textPrimary,
    surfaceVariant = LightKeynearPalette.surfaceTonal,
    onSurfaceVariant = LightKeynearPalette.textSecondary,
    errorContainer = LightKeynearPalette.errorTonal,
    onErrorContainer = LightKeynearPalette.onDangerTonal,
)

private val KeynearDarkColorScheme = darkColorScheme(
    primary = DarkKeynearPalette.accent,
    onPrimary = DarkKeynearPalette.onAccent,
    background = DarkKeynearPalette.canvas,
    onBackground = DarkKeynearPalette.textPrimary,
    surface = DarkKeynearPalette.surface,
    onSurface = DarkKeynearPalette.textPrimary,
    surfaceVariant = DarkKeynearPalette.surfaceTonal,
    onSurfaceVariant = DarkKeynearPalette.textSecondary,
    errorContainer = DarkKeynearPalette.errorTonal,
    onErrorContainer = DarkKeynearPalette.onDangerTonal,
)

private val PlusJakartaSansFallback = FontFamily.SansSerif

val KeynearTypography = Typography(
    headlineSmall = TextStyle(
        fontFamily = PlusJakartaSansFallback,
        fontWeight = FontWeight.SemiBold,
        fontSize = KeynearTypeSizes.Title,
        lineHeight = 30.sp,
    ),
    titleMedium = TextStyle(
        fontFamily = PlusJakartaSansFallback,
        fontWeight = FontWeight.SemiBold,
        fontSize = KeynearTypeSizes.Heading,
        lineHeight = 26.sp,
    ),
    bodyLarge = TextStyle(
        fontFamily = PlusJakartaSansFallback,
        fontWeight = FontWeight.Normal,
        fontSize = KeynearTypeSizes.Body,
        lineHeight = 24.sp,
    ),
    bodyMedium = TextStyle(
        fontFamily = PlusJakartaSansFallback,
        fontWeight = FontWeight.Normal,
        fontSize = KeynearTypeSizes.Description,
        lineHeight = 20.sp,
    ),
    labelMedium = TextStyle(
        fontFamily = PlusJakartaSansFallback,
        fontWeight = FontWeight.Medium,
        fontSize = KeynearTypeSizes.Caption,
        lineHeight = 16.sp,
    ),
)

@Composable
fun KeynearTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    val palette = if (darkTheme) DarkKeynearPalette else LightKeynearPalette
    val colorScheme = if (darkTheme) KeynearDarkColorScheme else KeynearLightColorScheme

    CompositionLocalProvider(LocalKeynearPalette provides palette) {
        MaterialTheme(
            colorScheme = colorScheme,
            typography = KeynearTypography,
            content = content,
        )
    }
}

