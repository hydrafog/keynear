package org.keynear.app.ui.navigation

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.core.DeviceRoleUi
import org.keynear.app.core.PairedDeviceUi
import org.keynear.app.ui.components.TactileSurface
import org.keynear.app.ui.icons.IconArrowLeft
import org.keynear.app.ui.icons.IconRadar
import org.keynear.app.ui.icons.IconSend
import org.keynear.app.ui.icons.IconSliders
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

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .background(if (subScreen == null && activeTab == KeynearTab.RECEIVE) palette.mapGround else palette.canvas)
            .statusBarsPadding()
            .height(64.dp)
            .padding(horizontal = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        if (subScreen != null) {
            TactileSurface(
                modifier = Modifier.size(48.dp),
                shape = RoundedCornerShape(16.dp),
                containerColor = palette.surface,
                shadowElevation = 3.dp,
                onClick = onBack,
            ) {
                Box(contentAlignment = Alignment.Center) {
                    IconArrowLeft(tint = palette.textPrimary)
                }
            }
        }
        Text(
            text = title,
            color = palette.textPrimary,
            fontSize = 24.sp,
            fontWeight = FontWeight.Bold,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

@Composable
fun KeynearBottomNav(
    activeTab: KeynearTab,
    subScreen: KeynearSubScreen?,
    onSelectTab: (KeynearTab) -> Unit,
) {
    val palette = LocalKeynearPalette.current
    Surface(
        modifier = Modifier.fillMaxWidth(),
        color = palette.surface,
        shadowElevation = 8.dp,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .navigationBarsPadding()
                .padding(top = 8.dp, bottom = 10.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp),
                horizontalArrangement = Arrangement.SpaceEvenly,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                KeynearTab.entries.forEach { tab ->
                    val active = subScreen == null && activeTab == tab
                    Column(
                        modifier = Modifier
                            .weight(1f)
                            .clickable(role = Role.Tab) { onSelectTab(tab) }
                            .padding(vertical = 2.dp),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        Box(
                            modifier = Modifier
                                .width(64.dp)
                                .height(36.dp)
                                .clip(CircleShape)
                                .background(if (active) palette.loadingTonal else Color.Transparent),
                            contentAlignment = Alignment.Center,
                        ) {
                            val tint = if (active) palette.onLoadingTonal else palette.textMuted
                            when (tab) {
                                KeynearTab.RECEIVE -> IconRadar(modifier = Modifier.size(20.dp), tint = tint)
                                KeynearTab.SEND -> IconSend(modifier = Modifier.size(20.dp), tint = tint)
                                KeynearTab.SETTINGS -> IconSliders(modifier = Modifier.size(20.dp), tint = tint)
                            }
                        }
                        Text(
                            text = tab.label,
                            color = if (active) palette.textPrimary else palette.textMuted,
                            fontSize = 12.sp,
                            fontWeight = if (active) FontWeight.Bold else FontWeight.Medium,
                        )
                    }
                }
            }
            Spacer(modifier = Modifier.height(8.dp))
            Box(
                modifier = Modifier
                    .width(128.dp)
                    .height(6.dp)
                    .clip(CircleShape)
                    .background(palette.gesturePill),
            )
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
        Surface(
            shape = RoundedCornerShape(14.dp),
            color = palette.snackbar,
            shadowElevation = 6.dp,
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
