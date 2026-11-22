package org.keynear.app.ui.navigation

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.designsystem.IconArrowLeft
import org.keynear.designsystem.IconKeynearSymbolic
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
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
fun KeynearTopAppBar(
    activeTab: KeynearTab,
    subScreen: KeynearSubScreen?,
    deviceDisplayName: String?,
    onBack: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val title = topAppBarTitle(activeTab, subScreen, deviceDisplayName)

    val isReceiveHome = subScreen == null && activeTab == KeynearTab.RECEIVE
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .background(palette.canvas)
            .statusBarsPadding()
            .height(ActivePillWidth)
            .padding(horizontal = KeynearSpacing.Lg),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
    ) {
        if (subScreen != null) {
            TactileSurface(
                modifier = Modifier.size(KeynearSpacing.MinTouchTarget),
                shape = KeynearShapes.Card,
                containerColor = palette.surface,
                tone = TactileTone.SURFACE,
                cornerRadius = KeynearRadii.TwoXl,
                onClick = onBack,
            ) {
                Box(
                    modifier = Modifier.size(KeynearSpacing.MinTouchTarget),
                    contentAlignment = Alignment.Center,
                ) {
                    IconArrowLeft(modifier = Modifier.size(KeynearSpacing.Xl), tint = palette.textPrimary)
                }
            }
        } else if (isReceiveHome) {
            IconKeynearSymbolic(
                modifier = Modifier.size(KeynearSpacing.Xl),
                tint = palette.textPrimary,
            )
        }

        Text(
            text = title,
            color = palette.textPrimary,
            fontSize = KeynearTypeSizes.Title,
            fontWeight = FontWeight.Bold,
            letterSpacing = (-0.6).sp,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

private fun topAppBarTitle(
    activeTab: KeynearTab,
    subScreen: KeynearSubScreen?,
    deviceDisplayName: String?,
): String = when (subScreen) {
    KeynearSubScreen.Permissions -> "System permissions"
    is KeynearSubScreen.DeviceDetail -> deviceDisplayName ?: "Device details"
    null -> when (activeTab) {
        KeynearTab.RECEIVE -> "Keynear"
        KeynearTab.SEND -> "Send files"
        KeynearTab.SETTINGS -> "Settings"
    }
}
