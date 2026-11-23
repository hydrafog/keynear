package org.keynear.app

import android.os.Build
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.ExitTransition
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import kotlinx.coroutines.launch
import org.keynear.app.ui.devices.DeviceDetailScreen
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.CoreState
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.app.ui.navigation.KeynearBottomNav
import org.keynear.app.ui.navigation.KeynearSubScreen
import org.keynear.app.ui.navigation.KeynearTab
import org.keynear.app.ui.navigation.KeynearTopAppBar
import org.keynear.app.ui.receive.ReceiveScreen
import org.keynear.app.ui.send.SendScreen
import org.keynear.app.ui.settings.PermissionsScreen
import org.keynear.app.ui.settings.SettingsScreen
import org.keynear.designsystem.ActionButton
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
internal fun KeynearAppContent(
    snapshot: CoreSnapshot?,
    uiState: CoreState,
    activeTab: KeynearTab,
    subScreen: KeynearSubScreen?,
    confirmUnpairDeviceId: String?,
    permissionsChecklist: org.keynear.app.ui.model.PermissionsChecklistState,
    actions: KeynearActions,
    sheetExpanded: Boolean,
    onSetSheetExpanded: (Boolean) -> Unit,
    dismissedTransferIds: Set<String>,
    onDismissTransfers: (List<String>) -> Unit,
    onBack: () -> Unit,
    onSelectTab: (KeynearTab) -> Unit,
    scope: kotlinx.coroutines.CoroutineScope,
    syncBleAndLoad: suspend () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val localName = remember { Build.MODEL?.takeIf { it.isNotBlank() } ?: "Android" }
    val selectedDevice = snapshot?.pairedDevices?.firstOrNull {
        (subScreen as? KeynearSubScreen.DeviceDetail)?.deviceId == it.deviceId
    }

    Scaffold(
        modifier = Modifier
            .fillMaxSize()
            .background(palette.canvas),
        containerColor = palette.canvas,
        topBar = {
            KeynearTopAppBar(
                activeTab = activeTab,
                subScreen = subScreen,
                deviceDisplayName = selectedDevice?.displayName,
                onBack = onBack,
            )
        },
        bottomBar = {
            KeynearBottomNav(
                activeTab = activeTab,
                subScreen = subScreen,
                missingPermissionsCount = permissionsChecklist.missingPermissionsCount,
                onSelectTab = onSelectTab,
            )
        },
    ) { innerPadding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding),
        ) {
            if (snapshot == null) {
                LoadingOrErrorContent(uiState, scope, syncBleAndLoad)
            } else {
                AnimatedContent(
                    targetState = subScreen ?: activeTab,
                    transitionSpec = {
                        fadeIn(
                            tween(
                                durationMillis = KeynearMotion.SHORT_DURATION_MS / 2,
                                easing = KeynearMotion.FadeEasing,
                            ),
                        ) togetherWith ExitTransition.None
                    },
                    label = "screenTransition",
                ) { target ->
                    val targetSub = target as? KeynearSubScreen
                    val targetTab = (target as? KeynearTab) ?: activeTab
                    CurrentScreenContent(
                        snapshot, targetTab, targetSub, confirmUnpairDeviceId,
                        permissionsChecklist, actions, localName, sheetExpanded,
                        onSetSheetExpanded, dismissedTransferIds, onDismissTransfers,
                    )
                }
            }
        }
    }
}

@Composable
private fun LoadingOrErrorContent(
    uiState: CoreState,
    scope: kotlinx.coroutines.CoroutineScope,
    syncBleAndLoad: suspend () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    Box(
        modifier = Modifier
            .fillMaxSize()
            .padding(KeynearSpacing.Xl),
        contentAlignment = Alignment.Center,
    ) {
        when (val current = uiState) {
            CoreState.Loading -> CircularProgressIndicator(color = palette.loading)
            is CoreState.Error -> {
                TactileSurface(
                    modifier = Modifier.fillMaxWidth(),
                    shape = KeynearShapes.Card,
                    containerColor = palette.errorTonal,
                    tone = TactileTone.DANGER_TONAL,
                    cornerRadius = KeynearRadii.TwoXl,
                ) {
                    Column(
                        modifier = Modifier.padding(KeynearLayout.CardPadding),
                        verticalArrangement = Arrangement.spacedBy(KeynearLayout.ContentGap),
                    ) {
                        Text(
                            text = current.message,
                            color = palette.onDangerTonal,
                            fontSize = KeynearTypeSizes.Description,
                            fontWeight = FontWeight.Medium,
                        )
                        ActionButton(
                            label = "Retry",
                            primary = true,
                            onClick = { scope.launch { syncBleAndLoad() } },
                        )
                    }
                }
            }
            else -> {}
        }
    }
}

@Composable
private fun CurrentScreenContent(
    snapshot: CoreSnapshot,
    activeTab: KeynearTab,
    subScreen: KeynearSubScreen?,
    confirmUnpairDeviceId: String?,
    permissionsChecklist: org.keynear.app.ui.model.PermissionsChecklistState,
    actions: KeynearActions,
    localName: String,
    sheetExpanded: Boolean,
    onSetSheetExpanded: (Boolean) -> Unit,
    dismissedTransferIds: Set<String>,
    onDismissTransfers: (List<String>) -> Unit,
) {
    when (val currentSub = subScreen) {
        KeynearSubScreen.Permissions -> PermissionsScreen(
            permissions = permissionsChecklist,
            actions = actions,
        )
        is KeynearSubScreen.DeviceDetail -> DeviceDetailScreen(
            device = snapshot.pairedDevices.firstOrNull { it.deviceId == currentSub.deviceId },
            confirmingUnpair = confirmUnpairDeviceId == currentSub.deviceId,
            actions = actions,
        )
        null -> when (activeTab) {
            KeynearTab.RECEIVE -> ReceiveScreen(
                snapshot = snapshot,
                localDeviceName = localName,
                sheetExpanded = sheetExpanded,
                onSetSheetExpanded = onSetSheetExpanded,
                dismissedTransferIds = dismissedTransferIds,
                onDismissTransfers = onDismissTransfers,
                actions = actions,
            )
            KeynearTab.SEND -> SendScreen(
                snapshot = snapshot,
                actions = actions,
            )
            KeynearTab.SETTINGS -> SettingsScreen(
                config = snapshot.daemonConfig,
                permissions = permissionsChecklist,
                actions = actions,
            )
        }
    }
}
