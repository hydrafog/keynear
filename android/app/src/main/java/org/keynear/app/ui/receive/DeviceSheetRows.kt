package org.keynear.app.ui.receive

import androidx.compose.foundation.LocalIndication
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.style.TextAlign
import org.keynear.app.ui.model.DeviceRoleUi
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.app.ui.navigation.OrbitPinUi
import org.keynear.designsystem.IconChevronRight
import org.keynear.designsystem.IconLaptop
import org.keynear.designsystem.IconRadio
import org.keynear.designsystem.IconSmartphone
import org.keynear.designsystem.KeynearDimensions
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearPressScale
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.designsystem.tactilePressScale

@Composable
internal fun EmptyDevicesSheetContent(modifier: Modifier = Modifier) {
    val palette = LocalKeynearPalette.current
    Box(
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = KeynearSpacing.Md, vertical = KeynearSpacing.Lg),
        contentAlignment = Alignment.Center,
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
        ) {
            TactileSurface(
                modifier = Modifier.size(KeynearLayout.EmptyBadgeSize),
                shape = KeynearShapes.Xl,
                containerColor = palette.loadingTonal,
                tone = TactileTone.ACCENT_TONAL,
                cornerRadius = KeynearRadii.Xl,
                contentAlignment = Alignment.Center,
            ) {
                IconRadio(
                    modifier = Modifier.size(KeynearDimensions.Dp20),
                    tint = palette.onLoadingTonal,
                )
            }
            Text(
                text = "No computers nearby",
                color = palette.textPrimary,
                fontSize = KeynearTypeSizes.Description,
                fontWeight = FontWeight.Bold,
            )
            Text(
                text = "Switch visibility to Paired only or Everyone nearby",
                color = palette.textMuted,
                fontSize = KeynearTypeSizes.Caption,
                textAlign = TextAlign.Center,
            )
        }
    }
}

@Composable
internal fun DeviceSheetRow(
    pin: OrbitPinUi,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current
    val interactionSource = remember { MutableInteractionSource() }
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = KeynearDimensions.Dp62)
            .tactilePressScale(interactionSource, KeynearPressScale.PRESS_99)
            .clip(KeynearShapes.Xl)
            .clickable(interactionSource = interactionSource, indication = LocalIndication.current) {
                if (pin.isPaired) {
                    actions.onOpenDeviceDetail(pin.id)
                } else {
                    actions.onInitiatePairing(pin.peerAddress)
                }
            }
            .padding(horizontal = KeynearSpacing.Md, vertical = KeynearLayout.ContentGap),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
    ) {
        TactileSurface(
            modifier = Modifier.size(KeynearLayout.EmptyBadgeSize),
            shape = KeynearShapes.Xl,
            containerColor = palette.loadingTonal,
            tone = TactileTone.ACCENT_TONAL,
            cornerRadius = KeynearRadii.Xl,
            contentAlignment = Alignment.Center,
        ) {
            if (pin.role == DeviceRoleUi.DESKTOP) {
                IconLaptop(modifier = Modifier.size(KeynearDimensions.Dp20), tint = palette.onLoadingTonal)
            } else {
                IconSmartphone(modifier = Modifier.size(KeynearDimensions.Dp20), tint = palette.onLoadingTonal)
            }
        }
        Column(modifier = Modifier.weight(1f)) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
            ) {
                Text(
                    text = pin.displayName,
                    color = palette.textPrimary,
                    fontSize = KeynearTypeSizes.Body,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f, fill = false),
                )
                if (!pin.isPaired) {
                    TactileSurface(
                        shape = KeynearShapes.Lg,
                        containerColor = palette.loadingTonal,
                        tone = TactileTone.ACCENT_TONAL,
                        cornerRadius = KeynearRadii.Lg,
                    ) {
                        Text(
                            text = "Pair",
                            color = palette.onLoadingTonal,
                            fontSize = KeynearTypeSizes.Caption,
                            fontWeight = FontWeight.Bold,
                            modifier = Modifier.padding(
                                horizontal = KeynearLayout.ContentGap,
                                vertical = KeynearDimensions.Dp2,
                            ),
                        )
                    }
                }
            }
            Text(
                text = pin.rssi?.let { "${pin.statusLabel} · $it dBm" } ?: pin.statusLabel,
                color = palette.textMuted,
                fontSize = KeynearTypeSizes.Caption,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
        IconChevronRight(modifier = Modifier.size(KeynearLayout.ChevronIconSize), tint = palette.textMuted)
    }
}
