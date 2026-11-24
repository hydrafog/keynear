package org.keynear.app.ui.devices

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import org.keynear.app.ui.model.PairedDeviceUi
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.designsystem.ActionButton
import org.keynear.designsystem.IconRefreshCw
import org.keynear.designsystem.IconTrash2
import org.keynear.designsystem.KeynearDimensions
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearLineHeights
import org.keynear.designsystem.KeynearMotion
import org.keynear.designsystem.KeynearPressScale
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone

private const val MODAL_ENTER_SCALE = 0.96f
private val DetailRowMinHeight = 54.dp

@Composable
internal fun DeviceActionsColumn(
    device: PairedDeviceUi,
    confirmingUnpair: Boolean,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current

    Column(verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Md)) {
        TactileSurface(
            modifier = Modifier
                .fillMaxWidth()
                .heightIn(min = DetailRowMinHeight),
            shape = KeynearShapes.Xl,
            containerColor = palette.surface,
            tone = TactileTone.SURFACE,
            cornerRadius = KeynearRadii.Xl,
            onClick = { actions.onResetBleBond(device.deviceId) },
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .heightIn(min = DetailRowMinHeight)
                    .padding(horizontal = KeynearDimensions.Dp20),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.Center,
            ) {
                IconRefreshCw(modifier = Modifier.size(KeynearDimensions.Dp20), tint = palette.textPrimary)
                Spacer(modifier = Modifier.size(KeynearLayout.ContentGap))
                Text(
                    text = "Reset Bluetooth bond",
                    color = palette.textPrimary,
                    fontSize = KeynearTypeSizes.Description,
                    fontWeight = FontWeight.Bold,
                )
            }
        }

        AnimatedContent(
            targetState = confirmingUnpair,
            transitionSpec = {
                val enter = fadeIn(
                    tween(durationMillis = KeynearMotion.MODAL_DURATION_MS, easing = KeynearMotion.SpringEasing),
                ) + scaleIn(
                    initialScale = MODAL_ENTER_SCALE,
                    animationSpec = tween(durationMillis = KeynearMotion.MODAL_DURATION_MS, easing = KeynearMotion.SpringEasing),
                )
                val exit = fadeOut(
                    tween(durationMillis = KeynearMotion.FADE_DURATION_MS, easing = KeynearMotion.ExitEasing),
                )
                enter togetherWith exit
            },
            label = "unpairConfirmation",
        ) { isConfirming ->
            if (!isConfirming) {
                TactileSurface(
                    modifier = Modifier
                        .fillMaxWidth()
                        .heightIn(min = DetailRowMinHeight),
                    shape = KeynearShapes.Xl,
                    containerColor = palette.surfaceTonal,
                    tone = TactileTone.TONAL,
                    cornerRadius = KeynearRadii.Xl,
                    onClick = { actions.onRequestUnpair(device.deviceId) },
                ) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .heightIn(min = DetailRowMinHeight)
                            .padding(horizontal = KeynearDimensions.Dp20),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.Center,
                    ) {
                        IconTrash2(modifier = Modifier.size(KeynearDimensions.Dp20), tint = palette.textSecondary)
                        Spacer(modifier = Modifier.size(KeynearLayout.ContentGap))
                        Text(
                            text = "Remove device",
                            color = palette.textSecondary,
                            fontSize = KeynearTypeSizes.Description,
                            fontWeight = FontWeight.Bold,
                        )
                    }
                }
            } else {
                TactileSurface(
                    modifier = Modifier.fillMaxWidth(),
                    shape = KeynearShapes.Card,
                    containerColor = palette.surfaceTonal,
                    tone = TactileTone.TONAL,
                    cornerRadius = KeynearRadii.TwoXl,
                ) {
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(KeynearLayout.CardPadding),
                        verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Lg),
                    ) {
                        Text(
                            text = "Remove ${device.displayName}? You will need to confirm a new " +
                                "6-digit code on both devices to pair again.",
                            color = palette.textPrimary,
                            fontSize = KeynearTypeSizes.Description,
                            fontWeight = FontWeight.SemiBold,
                            lineHeight = KeynearLineHeights.Body,
                        )
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
                        ) {
                            ActionButton(
                                label = "Remove device",
                                danger = true,
                                minHeight = KeynearLayout.ControlMinHeight,
                                onClick = { actions.onConfirmUnpair(device.deviceId) },
                                modifier = Modifier.weight(1f),
                            )
                            TactileSurface(
                                modifier = Modifier
                                    .weight(1f)
                                    .heightIn(min = KeynearLayout.ControlMinHeight),
                                shape = KeynearShapes.Xl,
                                containerColor = palette.surface,
                                tone = TactileTone.SURFACE,
                                cornerRadius = KeynearRadii.Xl,
                                pressScale = KeynearPressScale.PRESS_95,
                                contentAlignment = Alignment.Center,
                                onClick = actions.onCancelUnpair,
                            ) {
                                Text(
                                    text = "Cancel",
                                    color = palette.textPrimary,
                                    fontSize = KeynearTypeSizes.Description,
                                    fontWeight = FontWeight.Bold,
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}
