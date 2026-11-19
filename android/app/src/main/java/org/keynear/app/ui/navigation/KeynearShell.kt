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
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.ui.model.DeviceRoleUi
import org.keynear.app.ui.model.PairedDeviceUi
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.designsystem.IconArrowLeft
import org.keynear.designsystem.IconInbox
import org.keynear.designsystem.IconSend
import org.keynear.designsystem.IconSliders
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


enum class KeynearTab(val label: String) {
    RECEIVE("Receive"),
    SEND("Send"),
    SETTINGS("Settings"),
}

sealed interface KeynearSubScreen {
    data object Permissions : KeynearSubScreen
    data class DeviceDetail(val deviceId: String) : KeynearSubScreen
}

data class OrbitPinUi(
    val id: String,
    val displayName: String,
    val statusLabel: String,
    val role: DeviceRoleUi,
    val rssi: Int,
    val isPaired: Boolean,
    val isOnline: Boolean,
    val peerAddress: String,
)

data class KeynearActions(
    val onSetVisibility: (org.keynear.app.ui.model.VisibilityState) -> Unit,
    val onRequestBluetoothPermissions: () -> Unit,
    val onOpenBluetoothSettings: () -> Unit,
    val onRequestLocalNetworkPermission: () -> Unit,
    val onRequestNearbyOnboardingPermissions: () -> Unit,
    val onRequestNotificationsPermission: () -> Unit,
    val onRequestBatteryOptimization: () -> Unit,
    val onOpenAppSettings: () -> Unit,
    val onInitiatePairing: (String) -> Unit,
    val onApprovePairing: () -> Unit,
    val onRejectPairing: () -> Unit,
    val onToggleAutoAccept: (String, Boolean) -> Unit,
    val onRenameDevice: (String, String) -> Unit,
    val onResetBleBond: (String) -> Unit,
    val onAcceptTransfer: (String) -> Unit,
    val onDeclineTransfer: (String) -> Unit,
    val onCancelTransfer: (String) -> Unit,
    val onRequestUnpair: (String) -> Unit,
    val onCancelUnpair: () -> Unit,
    val onConfirmUnpair: (String) -> Unit,
    val onPickFiles: () -> Unit,
    val onPickFolder: () -> Unit,
    val onStageClipboard: () -> Unit,
    val onStageTextSnippet: (String) -> Unit,
    val onRemoveQueueItem: (String) -> Unit,
    val onClearQueue: () -> Unit,
    val onSendQueueToPeer: (PairedDeviceUi) -> Unit,
    val onUpdateConfig: ((org.keynear.app.ui.model.DaemonConfigUi) -> org.keynear.app.ui.model.DaemonConfigUi) -> Unit,
    val onPickSaveFolder: () -> Unit,
    val onOpenPermissions: () -> Unit,
    val onOpenDeviceDetail: (String) -> Unit,
    val onJumpToSendTab: () -> Unit,
)

@Composable
fun KeynearTopAppBar(
    activeTab: KeynearTab,
    subScreen: KeynearSubScreen?,
    deviceDisplayName: String?,
    onBack: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val title = when (subScreen) {
        KeynearSubScreen.Permissions -> "System permissions"
        is KeynearSubScreen.DeviceDetail -> deviceDisplayName ?: "Device details"
        null -> when (activeTab) {
            KeynearTab.RECEIVE -> "Keynear"
            KeynearTab.SEND -> "Send files"
            KeynearTab.SETTINGS -> "Settings"
        }
    }

    val isReceiveHome = subScreen == null && activeTab == KeynearTab.RECEIVE
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .background(if (isReceiveHome) palette.mapBlue else palette.canvas),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .clip(KeynearShapes.TopBar)
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
}

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
