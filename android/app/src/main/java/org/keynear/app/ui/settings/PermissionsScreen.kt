package org.keynear.app.ui.settings

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import org.keynear.app.ui.model.PermissionsChecklistState
import org.keynear.designsystem.ActionButton
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette

@Composable
fun PermissionsScreen(
    permissions: PermissionsChecklistState,
    actions: KeynearActions,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    val rows = listOf(
        Triple(
            "Nearby devices",
            "Bluetooth and Wi-Fi discovery for nearby Linux desktops.",
            Pair(permissions.nearbyGranted, actions.onRequestNearbyOnboardingPermissions),
        ),
        Triple(
            "Local network access",
            "Direct peer-to-peer transfers over your local Wi-Fi.",
            Pair(permissions.localNetworkGranted, actions.onRequestLocalNetworkPermission),
        ),
        Triple(
            "Companion device presence",
            "Wakes the background service automatically when a paired PC is near.",
            Pair(permissions.companionAssociated, actions.onOpenAppSettings),
        ),
        Triple(
            "Notifications",
            "Shows transfer requests, 6-digit pairing codes, and live progress.",
            Pair(permissions.notificationsGranted, actions.onRequestNotificationsPermission),
        ),
        Triple(
            "Unrestricted battery",
            "Prevents Android from interrupting large transfers with the screen off.",
            Pair(permissions.batteryOptimizationIgnored, actions.onRequestBatteryOptimization),
        ),
    )

    Column(
        modifier = modifier
            .fillMaxSize()
            .background(palette.canvas)
            .verticalScroll(rememberScrollState())
            .padding(
                horizontal = KeynearLayout.ScreenPaddingHorizontal,
                vertical = KeynearLayout.ScreenPaddingVertical,
            ),
        verticalArrangement = Arrangement.spacedBy(KeynearLayout.CompactSectionGap),
    ) {
        TactileSurface(
            modifier = Modifier.fillMaxWidth(),
            shape = KeynearShapes.Card,
            containerColor = palette.surface,
            tone = TactileTone.SURFACE,
            cornerRadius = KeynearRadii.TwoXl,
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(ContentGap),
                verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
            ) {
                rows.forEach { (title, sub, actionPair) ->
                    val (granted, onClick) = actionPair
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(RowIconSize),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
                    ) {
                        Column(
                            modifier = Modifier.weight(1f),
                            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Xs),
                        ) {
                            Text(
                                text = title,
                                color = palette.textPrimary,
                                fontSize = KeynearTypeSizes.Body,
                                fontWeight = FontWeight.Bold,
                            )
                            Text(
                                text = sub,
                                color = palette.textMuted,
                                fontSize = KeynearTypeSizes.Caption,
                                lineHeight = ValueTextSize,
                            )
                        }
                        ActionButton(
                            label = if (granted) "Allowed" else "Allow",
                            primary = !granted,
                            minHeight = KeynearSpacing.MinTouchTarget,
                            onClick = onClick,
                        )
                    }
                }
            }
        }
    }
}
