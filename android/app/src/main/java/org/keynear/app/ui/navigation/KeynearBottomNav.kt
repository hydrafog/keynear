package org.keynear.app.ui.navigation

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
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
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import org.keynear.designsystem.IconInbox
import org.keynear.designsystem.IconSend
import org.keynear.designsystem.IconSliders
import org.keynear.designsystem.KeynearDimensions
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearMotion
import org.keynear.designsystem.KeynearPalette
import org.keynear.designsystem.KeynearPressScale
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.designsystem.tactilePressScale

private val ActivePillWidth = 64.dp
private val BadgeOffsetX = 3.dp
private val BadgeOffsetY = (-3).dp
private val BadgeRadius = 5.dp
private val InactivePillWidth = 40.dp
private val NavPillHeight = 36.dp
private val NavPillRadius = 18.dp

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
            .padding(
                start = KeynearSpacing.Sm,
                end = KeynearSpacing.Sm,
                top = KeynearSpacing.Sm,
                bottom = KeynearSpacing.Sm,
            ),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceEvenly,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            KeynearTab.entries.forEach { tab ->
                BottomNavItem(
                    tab = tab,
                    active = activeTab == tab,
                    inSubScreen = subScreen != null,
                    missingPermissionsCount = missingPermissionsCount,
                    palette = palette,
                    onClick = { onSelectTab(tab) },
                    modifier = Modifier.weight(1f),
                )
            }
        }
    }
}

@Composable
private fun BottomNavItem(
    tab: KeynearTab,
    active: Boolean,
    inSubScreen: Boolean,
    missingPermissionsCount: Int,
    palette: KeynearPalette,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val pillWidth by animateDpAsState(
        targetValue = if (active) ActivePillWidth else InactivePillWidth,
        animationSpec = tween(durationMillis = KeynearMotion.MORPH_DURATION_MS, easing = KeynearMotion.SpringEasing),
        label = "navPillWidth",
    )
    val pillAlpha by animateFloatAsState(
        targetValue = if (active) 1f else 0f,
        animationSpec = tween(durationMillis = KeynearMotion.MORPH_DURATION_MS, easing = KeynearMotion.SpringEasing),
        label = "navPillAlpha",
    )
    val tint by animateColorAsState(
        targetValue = if (active) palette.onLoadingTonal else palette.textMuted,
        animationSpec = tween(durationMillis = KeynearMotion.MORPH_DURATION_MS, easing = KeynearMotion.SpringEasing),
        label = "navTint",
    )
    val labelColor by animateColorAsState(
        targetValue = if (active) palette.textPrimary else palette.textMuted,
        animationSpec = tween(durationMillis = KeynearMotion.SHORT_DURATION_MS, easing = KeynearMotion.FadeEasing),
        label = "navLabelColor",
    )
    val interactionSource = remember { MutableInteractionSource() }
    Column(
        modifier = modifier
            .tactilePressScale(interactionSource, KeynearPressScale.PRESS_95)
            .semantics {
                selected = active
                if (active && inSubScreen) {
                    stateDescription = "Parent section"
                }
            }
            .clickable(
                interactionSource = interactionSource,
                indication = null,
                role = Role.Tab,
                onClick = onClick,
            )
            .padding(vertical = KeynearDimensions.Dp2),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(KeynearLayout.CompactGap),
    ) {
        Box(
            modifier = Modifier
                .width(pillWidth)
                .height(NavPillHeight),
            contentAlignment = Alignment.Center,
        ) {
            if (pillAlpha > 0f) {
                NavActivePill(pillWidth = pillWidth, pillAlpha = pillAlpha, palette = palette)
            }
            Box(
                modifier = Modifier.size(KeynearDimensions.Dp20),
                contentAlignment = Alignment.Center,
            ) {
                NavTabIcon(tab, tint, missingPermissionsCount, palette)
            }
        }
        Text(
            text = tab.label,
            color = labelColor,
            fontSize = KeynearTypeSizes.Caption,
            fontWeight = FontWeight.Bold,
        )
    }
}

@Composable
private fun NavActivePill(
    pillWidth: Dp,
    pillAlpha: Float,
    palette: KeynearPalette,
) {
    TactileSurface(
        modifier = Modifier
            .width(pillWidth)
            .height(NavPillHeight)
            .graphicsLayer { alpha = pillAlpha },
        shape = CircleShape,
        containerColor = palette.loadingTonal,
        tone = TactileTone.ACCENT_TONAL,
        cornerRadius = NavPillRadius,
    ) {
        Box(modifier = Modifier.width(pillWidth).height(NavPillHeight))
    }
}

@Composable
private fun NavTabIcon(
    tab: KeynearTab,
    tint: Color,
    missingPermissionsCount: Int,
    palette: KeynearPalette,
) {
    Box(
        modifier = Modifier.size(KeynearDimensions.Dp20),
        contentAlignment = Alignment.Center,
    ) {
        when (tab) {
            KeynearTab.RECEIVE -> IconInbox(modifier = Modifier.size(KeynearDimensions.Dp20), tint = tint)
            KeynearTab.SEND -> IconSend(modifier = Modifier.size(KeynearDimensions.Dp20), tint = tint)
            KeynearTab.SETTINGS -> IconSliders(modifier = Modifier.size(KeynearDimensions.Dp20), tint = tint)
        }
        if (tab == KeynearTab.SETTINGS && missingPermissionsCount > 0) {
            TactileSurface(
                modifier = Modifier
                    .align(Alignment.TopEnd)
                    .offset(x = BadgeOffsetX, y = BadgeOffsetY)
                    .size(KeynearDimensions.Dp10),
                shape = CircleShape,
                containerColor = palette.error,
                tone = TactileTone.DANGER,
                cornerRadius = BadgeRadius,
                isCircle = true,
            ) {
                Box(modifier = Modifier.size(KeynearDimensions.Dp10))
            }
        }
    }
}

