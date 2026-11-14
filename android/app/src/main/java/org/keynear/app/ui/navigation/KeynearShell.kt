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
import org.keynear.app.core.DeviceRoleUi
import org.keynear.app.core.PairedDeviceUi
import org.keynear.app.ui.components.TactileSurface
import org.keynear.app.ui.components.TactileTone
import org.keynear.app.ui.icons.IconArrowLeft
import org.keynear.app.ui.icons.IconInbox
import org.keynear.app.ui.icons.IconSend
import org.keynear.app.ui.icons.IconSliders
import org.keynear.app.ui.theme.KeynearRadii
import org.keynear.app.ui.theme.KeynearShapes
import org.keynear.app.ui.theme.LocalKeynearPalette

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
    val onSetVisibility: (org.keynear.app.core.VisibilityState) -> Unit,
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
    val onUpdateConfig: ((org.keynear.app.core.DaemonConfigUi) -> org.keynear.app.core.DaemonConfigUi) -> Unit,
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
                .height(64.dp)
                .padding(horizontal = 16.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            if (subScreen != null) {
                TactileSurface(
                    modifier = Modifier.size(48.dp),
                    shape = KeynearShapes.Card,
                    containerColor = palette.surface,
                    tone = TactileTone.SURFACE,
                    cornerRadius = KeynearRadii.TwoXl,
                    onClick = onBack,
                ) {
                    Box(
                        modifier = Modifier.size(48.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        IconArrowLeft(modifier = Modifier.size(24.dp), tint = palette.textPrimary)
                    }
                }
            }

            Text(
                text = title,
                color = palette.textPrimary,
                fontSize = 24.sp,
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
            .padding(start = 8.dp, end = 8.dp, top = 8.dp, bottom = 8.dp),
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
                    targetValue = if (active) 64.dp else 40.dp,
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
                        .padding(vertical = 2.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    Box(
                        modifier = Modifier
                            .width(pillWidth)
                            .height(36.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        if (active) {
                            TactileSurface(
                                modifier = Modifier
                                    .width(pillWidth)
                                    .height(36.dp),
                                shape = CircleShape,
                                containerColor = palette.loadingTonal,
                                tone = TactileTone.TONAL,
                                cornerRadius = 18.dp,
                            ) {
                                Box(modifier = Modifier.width(pillWidth).height(36.dp))
                            }
                        }
                        when (tab) {
                            KeynearTab.RECEIVE -> IconInbox(modifier = Modifier.size(20.dp), tint = tint)
                            KeynearTab.SEND -> IconSend(modifier = Modifier.size(20.dp), tint = tint)
                            KeynearTab.SETTINGS -> IconSliders(modifier = Modifier.size(20.dp), tint = tint)
                        }
                        if (tab == KeynearTab.SETTINGS && missingPermissionsCount > 0) {
                            TactileSurface(
                                modifier = Modifier
                                    .align(Alignment.TopEnd)
                                    .padding(top = 4.dp, end = 6.dp)
                                    .size(10.dp),
                                shape = CircleShape,
                                containerColor = palette.error,
                                tone = TactileTone.TONAL,
                                cornerRadius = 5.dp,
                                isCircle = true,
                            ) {
                                Box(modifier = Modifier.size(10.dp))
                            }
                        }
                    }
                    Text(
                        text = tab.label,
                        color = if (active) palette.textPrimary else palette.textMuted,
                        fontSize = 12.sp,
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
            .padding(horizontal = 24.dp, vertical = 12.dp),
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
                fontSize = 13.sp,
                fontWeight = FontWeight.SemiBold,
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 10.dp),
            )
        }
    }
}
