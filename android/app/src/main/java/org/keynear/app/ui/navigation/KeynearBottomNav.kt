package org.keynear.app.ui.navigation

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.designsystem.IconInbox
import org.keynear.designsystem.IconSend
import org.keynear.designsystem.IconSliders
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
fun KeynearBottomNav(
    activeTab: KeynearTab,
    subScreen: KeynearSubScreen?,
    missingPermissionsCount: Int = 0,
    onSelectTab: (KeynearTab) -> Unit,
) {
    val palette = LocalKeynearPalette.current
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .background(palette.canvas)
            .navigationBarsPadding()
            .padding(start = KeynearSpacing.Sm, end = KeynearSpacing.Sm, top = KeynearSpacing.Sm, bottom = KeynearSpacing.Sm),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceEvenly,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            KeynearTab.entries.forEach { tab ->
                val active = subScreen == null && activeTab == tab
                val pillWidth by animateDpAsState(
                    targetValue = if (active) ActivePillWidth else InactivePillWidth,
                    label = "navPillWidth",
                )
                val tint by animateColorAsState(
                    targetValue = if (active) palette.onLoadingTonal else palette.textMuted,
                    label = "navTint",
                )
                Column(
                    modifier = Modifier
                        .weight(1f)
                        .clickable(role = Role.Tab) { onSelectTab(tab) }
                        .padding(vertical = NavItemVerticalPadding),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(BadgeInset),
                ) {
                    Box(
                        modifier = Modifier
                            .width(pillWidth)
                            .height(NavPillHeight),
                        contentAlignment = Alignment.Center,
                    ) {
                        if (active) {
                            NavActivePill(pillWidth, palette)
                        }
                        Box(
                            modifier = Modifier.size(NavIconSize),
                            contentAlignment = Alignment.Center,
                        ) {
                            NavTabIcon(tab, tint, missingPermissionsCount, palette)
                        }
                    }
                    Text(
                        text = tab.label,
                        color = if (active) palette.textPrimary else palette.textMuted,
                        fontSize = KeynearTypeSizes.Caption,
                        fontWeight = FontWeight.Bold,
                    )
                }
            }
        }
    }
}

@Composable
private fun NavActivePill(pillWidth: androidx.compose.ui.unit.Dp, palette: org.keynear.designsystem.KeynearPalette) {
    TactileSurface(
        modifier = Modifier
            .width(pillWidth)
            .height(NavPillHeight),
        shape = CircleShape,
        containerColor = palette.loadingTonal,
        tone = TactileTone.TONAL,
        cornerRadius = NavPillRadius,
    ) {
        Box(modifier = Modifier.width(pillWidth).height(NavPillHeight))
    }
}

@Composable
private fun NavTabIcon(
    tab: KeynearTab,
    tint: androidx.compose.ui.graphics.Color,
    missingPermissionsCount: Int,
    palette: org.keynear.designsystem.KeynearPalette,
) {
    Box(
        modifier = Modifier.size(NavIconSize),
        contentAlignment = Alignment.Center,
    ) {
        when (tab) {
            KeynearTab.RECEIVE -> IconInbox(modifier = Modifier.size(NavIconSize), tint = tint)
            KeynearTab.SEND -> IconSend(modifier = Modifier.size(NavIconSize), tint = tint)
            KeynearTab.SETTINGS -> IconSliders(modifier = Modifier.size(NavIconSize), tint = tint)
        }
        if (tab == KeynearTab.SETTINGS && missingPermissionsCount > 0) {
            TactileSurface(
                modifier = Modifier
                    .align(Alignment.TopEnd)
                    .offset(x = BadgeOffsetX, y = BadgeOffsetY)
                    .size(ContentGap),
                shape = CircleShape,
                containerColor = palette.error,
                tone = TactileTone.TONAL,
                cornerRadius = BadgeRadius,
                isCircle = true,
            ) {
                Box(modifier = Modifier.size(ContentGap))
            }
        }
    }
}

