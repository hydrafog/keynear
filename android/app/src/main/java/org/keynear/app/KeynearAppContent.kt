package org.keynear.app

import android.os.Build
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.launch
import org.keynear.app.ui.model.CoreSnapshot
import org.keynear.app.ui.model.CoreState
import org.keynear.designsystem.ActionButton
import org.keynear.designsystem.TactileSurface
import org.keynear.app.ui.devices.DeviceDetailScreen
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.app.ui.navigation.KeynearBottomNav
import org.keynear.app.ui.navigation.KeynearSnackbarHost
import org.keynear.app.ui.navigation.KeynearSubScreen
import org.keynear.app.ui.navigation.KeynearTab
import org.keynear.app.ui.navigation.KeynearTopAppBar
import org.keynear.app.ui.receive.ReceiveScreen
import org.keynear.app.ui.send.SendScreen
import org.keynear.app.ui.settings.PermissionsScreen
import org.keynear.app.ui.settings.SettingsScreen
import org.keynear.designsystem.LocalKeynearPalette

@Suppress("CyclomaticComplexMethod")
@Composable
internal fun KeynearAppContent(
    snapshot: CoreSnapshot?,
    uiState: CoreState,
    activeTab: KeynearTab,
    subScreen: KeynearSubScreen?,
    confirmUnpairDeviceId: String?,
    snackbarMessage: String?,
    permissionsChecklist: org.keynear.app.ui.model.PermissionsChecklistState,
    actions: KeynearActions,
    sheetExpanded: Boolean,
    onSetSheetExpanded: (Boolean) -> Unit,
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
                CurrentScreenContent(
                    snapshot, activeTab, subScreen, confirmUnpairDeviceId,
                    permissionsChecklist, actions, localName, sheetExpanded, onSetSheetExpanded,
                )
            }


            Box(modifier = Modifier.align(Alignment.BottomCenter)) {
                KeynearSnackbarHost(message = snackbarMessage)
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
            .padding(24.dp),
        contentAlignment = Alignment.Center,
    ) {
        when (val current = uiState) {
            CoreState.Loading -> CircularProgressIndicator(color = palette.loading)
            is CoreState.Error -> {
                TactileSurface(
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(16.dp),
                    containerColor = palette.errorTonal,
                    shadowElevation = 0.dp,
                ) {
                    Column(
                        modifier = Modifier.padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        Text(
                            text = current.message,
                            color = palette.onDangerTonal,
                            fontSize = 14.sp,
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
