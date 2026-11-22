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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import org.keynear.app.ui.model.PermissionsChecklistState
import org.keynear.designsystem.ActionButton
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.designsystem.KeynearDimensions
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette

private data class PermissionRow(
    val title: String,
    val description: String,
    val granted: Boolean,
    val action: () -> Unit,
)

@Composable
fun PermissionsScreen(
    permissions: PermissionsChecklistState,
    actions: KeynearActions,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    val rows = listOf(
        PermissionRow(
            title = "Nearby devices",
            description = "Bluetooth and Wi-Fi discovery for nearby Linux desktops.",
            granted = permissions.nearbyGranted,
            action = actions.onRequestNearbyOnboardingPermissions,
        ),
        PermissionRow(
            title = "Local network access",
            description = "Direct peer-to-peer transfers over your local Wi-Fi.",
            granted = permissions.localNetworkGranted,
            action = actions.onRequestLocalNetworkPermission,
        ),
        PermissionRow(
            title = "Notifications",
            description = "Shows transfer requests, 6-digit pairing codes, and live progress.",
            granted = permissions.notificationsGranted,
            action = actions.onRequestNotificationsPermission,
        ),
        PermissionRow(
            title = "Unrestricted battery",
            description = "Prevents Android from interrupting large transfers with the screen off.",
            granted = permissions.batteryOptimizationIgnored,
            action = actions.onRequestBatteryOptimization,
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
                    .padding(KeynearDimensions.Dp10),
                verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
            ) {
                rows.forEach { row ->
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(KeynearLayout.GridGap),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
                    ) {
                        Column(
                            modifier = Modifier.weight(1f),
                            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Xs),
                        ) {
                            Text(
                                text = row.title,
                                color = palette.textPrimary,
                                fontSize = KeynearTypeSizes.Body,
                                fontWeight = FontWeight.Bold,
                            )
                            Text(
                                text = row.description,
                                color = palette.textMuted,
                                fontSize = KeynearTypeSizes.Caption,
                                lineHeight = KeynearTypeSizes.Sp19,
                            )
                        }
                        ActionButton(
                            label = if (row.granted) "Allowed" else "Allow",
                            primary = !row.granted,
                            minHeight = KeynearSpacing.MinTouchTarget,
                            onClick = row.action,
                        )
                    }
                }
            }
        }
    }
}
