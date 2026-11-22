package org.keynear.app.ui.receive

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.ui.model.DeviceRoleUi
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.app.ui.navigation.OrbitPinUi
import org.keynear.designsystem.IconChevronRight
import org.keynear.designsystem.IconLaptop
import org.keynear.designsystem.IconRadio
import org.keynear.designsystem.IconSmartphone
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone
import org.keynear.designsystem.tactileCushion

private val BadgeHorizontalPadding = 2.dp
private val ChevronSize = 20.dp
private val CompactGap = 6.dp
private val ContentGap = 10.dp
private val DeviceRowMinHeight = 62.dp
private val EchoStrokeWidth = 4.5.dp
private val EmptyIconSize = 22.dp
private val EmptySheetExpandedHeight = 156.dp
private const val ORBIT_CYCLE_MS = 40_000
private val PinIconSize = 28.dp
private val PinLabelTextSize = 13.sp
private val PinLabelVerticalPadding = 7.dp
private val PinNodeSize = 64.dp
private val RowHorizontalPadding = 14.dp
private val RowIconSize = 44.dp
private val SelfIconSize = 36.dp
private val SelfNodeRadius = 41.dp
private val SelfNodeSize = 82.dp
private val SheetCollapsedHeight = 132.dp
private val SheetExpandedHeight = 252.dp
private val SpokeStrokeWidth = 1.dp
private val VisibilityControlHeight = 56.dp
private val VisibilityIconSize = 18.dp
private const val WAVE_CYCLE_MS = 120_000
private const val WAVE_MOTE_ALPHA = 0.5f


@Composable
internal fun EmptyDevicesSheetContent(
    currentHeight: Dp,
    dragModifier: Modifier,
) {
    val palette = LocalKeynearPalette.current
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .height(currentHeight)
            .then(dragModifier)
            .padding(KeynearSpacing.Lg),
        contentAlignment = Alignment.Center,
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
        ) {
            Box(
                modifier = Modifier
                    .size(RowIconSize)
                    .clip(KeynearShapes.Xl)
                    .background(palette.trackWell)
                    .tactileCushion(
                        tone = TactileTone.INSET,
                        isDark = palette.isDark,
                        cornerRadius = KeynearRadii.Xl,
                    ),
                contentAlignment = Alignment.Center,
            ) {
                IconRadio(modifier = Modifier.size(EmptyIconSize), tint = palette.textMuted)
            }
            Text(
                text = "No computers nearby",
                color = palette.textPrimary,
                fontSize = KeynearTypeSizes.Description,
                fontWeight = FontWeight.Bold,
            )
            Text(
                text = "Switch visibility to Paired or Everyone",
                color = palette.textMuted,
                fontSize = KeynearTypeSizes.Caption,
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
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = DeviceRowMinHeight)
            .clip(KeynearShapes.Xl)
            .clickable {
                if (pin.isPaired) {
                    actions.onOpenDeviceDetail(pin.id)
                } else {
                    actions.onInitiatePairing(pin.peerAddress)
                }
            }
            .padding(horizontal = RowHorizontalPadding, vertical = ContentGap),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(RowHorizontalPadding),
    ) {
        TactileSurface(
            modifier = Modifier.size(RowIconSize),
            shape = KeynearShapes.Xl,
            containerColor = palette.loadingTonal,
            tone = TactileTone.TONAL,
            cornerRadius = KeynearRadii.Xl,
        ) {
            Box(modifier = Modifier.size(RowIconSize), contentAlignment = Alignment.Center) {
                if (pin.role == DeviceRoleUi.DESKTOP) {
                    IconLaptop(modifier = Modifier.size(ChevronSize), tint = palette.onLoadingTonal)
                } else {
                    IconSmartphone(modifier = Modifier.size(ChevronSize), tint = palette.onLoadingTonal)
                }
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
                    fontWeight = FontWeight.Bold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                if (!pin.isPaired) {
                    TactileSurface(
                        shape = KeynearShapes.Lg,
                        containerColor = palette.loadingTonal,
                        tone = TactileTone.TONAL,
                        cornerRadius = KeynearRadii.Lg,
                    ) {
                        Text(
                            text = "Pair",
                            color = palette.onLoadingTonal,
                            fontSize = KeynearTypeSizes.Caption,
                            fontWeight = FontWeight.Bold,
                            modifier = Modifier.padding(horizontal = ContentGap, vertical = BadgeHorizontalPadding),
                        )
                    }
                }
            }
            Text(
                text = "${pin.statusLabel} ${pin.rssi} dBm",
                color = palette.textMuted,
                fontSize = KeynearTypeSizes.Caption,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
        IconChevronRight(modifier = Modifier.size(ChevronSize), tint = palette.textMuted)
    }
}
