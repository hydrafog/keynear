package org.keynear.app.ui.navigation

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.LocalKeynearPalette

private val ActivePillWidth = 64.dp
private val BadgeInset = 6.dp
private val BadgeOffsetX = 3.dp
private val BadgeOffsetY = (-3).dp
private val BadgeRadius = 5.dp
private val ContentGap = 10.dp
private val InactivePillWidth = 40.dp
private val NavIconSize = 20.dp
private val NavItemVerticalPadding = 2.dp
private val NavPillHeight = 36.dp
private val NavPillRadius = 18.dp
private val SnackbarTextSize = 13.sp

@Composable
fun KeynearSnackbarHost(message: String?) {
    if (message == null) return
    val palette = LocalKeynearPalette.current
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = KeynearSpacing.Xl, vertical = KeynearSpacing.Md),
        contentAlignment = Alignment.Center,
    ) {
        TactileSurface(
            shape = KeynearShapes.Card,
            containerColor = palette.snackbar,
            tone = TactileTone.SURFACE,
            cornerRadius = KeynearRadii.TwoXl,
        ) {
            Text(
                text = message,
                color = palette.onSnackbar,
                fontSize = SnackbarTextSize,
                fontWeight = FontWeight.SemiBold,
                modifier = Modifier.padding(horizontal = KeynearSpacing.Lg, vertical = ContentGap),
            )
        }
    }
}
