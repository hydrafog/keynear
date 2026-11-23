package org.keynear.app.ui.navigation

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearMotion
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone

@Composable
fun KeynearSnackbarHost(message: String?) {
    var retainedMessage by remember { mutableStateOf(message) }
    if (message != null) {
        retainedMessage = message
    }
    val palette = LocalKeynearPalette.current
    val slideOffsetPx = with(LocalDensity.current) { KeynearSpacing.Xl.roundToPx() }
    val enterTransition = fadeIn(
        tween(durationMillis = KeynearMotion.SHEET_DURATION_MS, easing = KeynearMotion.SpringEasing),
    ) + slideInVertically(
        animationSpec = tween(durationMillis = KeynearMotion.SHEET_DURATION_MS, easing = KeynearMotion.SpringEasing),
    ) { slideOffsetPx }
    val exitTransition = fadeOut(
        tween(durationMillis = KeynearMotion.EXIT_DURATION_MS, easing = KeynearMotion.ExitEasing),
    ) + slideOutVertically(
        animationSpec = tween(durationMillis = KeynearMotion.EXIT_DURATION_MS, easing = KeynearMotion.ExitEasing),
    ) { slideOffsetPx }

    AnimatedVisibility(
        visible = message != null,
        enter = enterTransition,
        exit = exitTransition,
    ) {
        val displayMessage = retainedMessage ?: return@AnimatedVisibility
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = KeynearSpacing.Xl, vertical = KeynearSpacing.Md),
            contentAlignment = Alignment.Center,
        ) {
            TactileSurface(
                shape = KeynearShapes.Xl,
                containerColor = palette.snackbar,
                tone = TactileTone.SURFACE,
                cornerRadius = KeynearRadii.Xl,
            ) {
                Text(
                    text = displayMessage,
                    color = palette.onSnackbar,
                    fontSize = KeynearTypeSizes.Description,
                    fontWeight = FontWeight.SemiBold,
                    modifier = Modifier.padding(
                        horizontal = KeynearLayout.CardPaddingLarge,
                        vertical = KeynearSpacing.Md,
                    ),
                )
            }
        }
    }
}
