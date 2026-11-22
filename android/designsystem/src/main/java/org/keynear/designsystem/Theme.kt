package org.keynear.designsystem

import android.app.Activity
import android.os.Build
import androidx.compose.foundation.IndicationNodeFactory
import androidx.compose.foundation.LocalIndication
import androidx.compose.foundation.interaction.FocusInteraction
import androidx.compose.foundation.interaction.InteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.ContentDrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.node.DelegatableNode
import androidx.compose.ui.node.DrawModifierNode
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.core.view.WindowCompat
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

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

private val PlusJakartaSans = FontFamily(
    Font(R.font.plus_jakarta_sans, FontWeight.Normal),
    Font(R.font.plus_jakarta_sans, FontWeight.Medium),
    Font(R.font.plus_jakarta_sans, FontWeight.SemiBold),
    Font(R.font.plus_jakarta_sans, FontWeight.Bold),
)

val KeynearTypography = Typography(
    headlineSmall = TextStyle(
        fontFamily = PlusJakartaSans,
        fontWeight = FontWeight.SemiBold,
        fontSize = KeynearTypeSizes.Title,
        lineHeight = KeynearLineHeights.Title,
    ),
    titleMedium = TextStyle(
        fontFamily = PlusJakartaSans,
        fontWeight = FontWeight.SemiBold,
        fontSize = KeynearTypeSizes.Heading,
        lineHeight = KeynearLineHeights.Heading,
    ),
    bodyLarge = TextStyle(
        fontFamily = PlusJakartaSans,
        fontWeight = FontWeight.Normal,
        fontSize = KeynearTypeSizes.Body,
        lineHeight = KeynearLineHeights.Body,
    ),
    bodyMedium = TextStyle(
        fontFamily = PlusJakartaSans,
        fontWeight = FontWeight.Normal,
        fontSize = KeynearTypeSizes.Description,
        lineHeight = KeynearLineHeights.Description,
    ),
    labelMedium = TextStyle(
        fontFamily = PlusJakartaSans,
        fontWeight = FontWeight.Medium,
        fontSize = KeynearTypeSizes.Caption,
        lineHeight = KeynearLineHeights.Caption,
    ),
)

private class KeynearIndicationNode(
    private val interactionSource: InteractionSource,
    private val focusColor: Color,
) : Modifier.Node(), DrawModifierNode {
    private val focused = mutableStateOf(false)
    private val pressed = mutableStateOf(false)
    private var job: Job? = null

    override fun onAttach() {
        job = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate).launch {
            interactionSource.interactions.collect { interaction ->
                when (interaction) {
                    is FocusInteraction.Focus -> focused.value = true
                    is FocusInteraction.Unfocus -> focused.value = false
                    is PressInteraction.Press -> pressed.value = true
                    is PressInteraction.Release, is PressInteraction.Cancel -> pressed.value = false
                }
            }
        }
    }

    override fun onDetach() {
        job?.cancel()
    }

    override fun ContentDrawScope.draw() {
        drawContent()
        if (pressed.value) {
            drawRect(color = focusColor.copy(alpha = KeynearOpacity.PRESSED_OVERLAY))
        }
        if (focused.value) {
            drawRoundRect(
                color = focusColor,
                cornerRadius = CornerRadius(KeynearStates.FocusCornerRadius.toPx()),
                style = Stroke(width = KeynearStates.FocusOutlineWidth.toPx()),
            )
        }
    }
}

private class KeynearIndication(private val focusColor: Color) : IndicationNodeFactory {
    override fun create(interactionSource: InteractionSource): DelegatableNode =
        KeynearIndicationNode(interactionSource, focusColor)

    override fun hashCode(): Int = focusColor.hashCode()

    override fun equals(other: Any?): Boolean = other is KeynearIndication && other.focusColor == focusColor
}

@Composable
fun KeynearTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    dynamicAccent: Boolean = true,
    content: @Composable () -> Unit,
) {
    val context = LocalContext.current
    val basePalette = if (darkTheme) DarkKeynearPalette else LightKeynearPalette
    val baseColorScheme = if (darkTheme) KeynearDarkColorScheme else KeynearLightColorScheme
    val dynamicScheme = if (dynamicAccent && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        if (darkTheme) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
    } else {
        null
    }
    val palette = if (dynamicScheme != null) basePalette.withMaterialAccent(dynamicScheme) else basePalette
    val colorScheme = if (dynamicScheme != null) {
        baseColorScheme.copy(
            primary = dynamicScheme.primary,
            onPrimary = dynamicScheme.onPrimary,
            primaryContainer = dynamicScheme.primaryContainer,
            onPrimaryContainer = dynamicScheme.onPrimaryContainer,
        )
    } else {
        baseColorScheme
    }
    val indication = remember(palette) { KeynearIndication(palette.focusRing) }
    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = (view.context as? Activity)?.window ?: return@SideEffect
            val controller = WindowCompat.getInsetsController(window, view)
            controller.isAppearanceLightStatusBars = !darkTheme
            controller.isAppearanceLightNavigationBars = !darkTheme
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                window.isNavigationBarContrastEnforced = false
            }
        }
    }

    CompositionLocalProvider(LocalKeynearPalette provides palette) {
        MaterialTheme(
            colorScheme = colorScheme,
            typography = KeynearTypography,
        ) {
            CompositionLocalProvider(LocalIndication provides indication) {
                content()
            }
        }
    }
}
