package org.keynear.app.ui.receive

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.AnimationVector1D
import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.input.pointer.PointerInputScope
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.app.ui.navigation.OrbitPinUi
import org.keynear.designsystem.KeynearLayout
import org.keynear.designsystem.KeynearMotion
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone

private data class SheetBounds(
    val collapsedHeight: Dp,
    val expandedHeight: Dp,
    val targetHeight: Dp,
)

private suspend fun PointerInputScope.handleSheetVerticalDrag(
    bounds: SheetBounds,
    height: Animatable<Dp, AnimationVector1D>,
    scope: CoroutineScope,
    onSetExpanded: (Boolean) -> Unit,
) {
    val animSpec = tween<Dp>(durationMillis = KeynearMotion.MORPH_DURATION_MS, easing = KeynearMotion.SpringEasing)
    var dragHeight = 0.dp
    var snapJob: Job? = null
    detectVerticalDragGestures(
        onDragStart = { dragHeight = height.value },
        onVerticalDrag = { change, dragAmount ->
            change.consume()
            dragHeight = (dragHeight + (-dragAmount).toDp()).coerceIn(0.dp, bounds.expandedHeight)
            snapJob?.cancel()
            snapJob = scope.launch { height.snapTo(dragHeight) }
        },
        onDragEnd = {
            snapJob?.cancel()
            val shouldExpand = dragHeight >= (bounds.collapsedHeight + bounds.expandedHeight) / 2f
            onSetExpanded(shouldExpand)
            scope.launch {
                height.animateTo(if (shouldExpand) bounds.expandedHeight else bounds.collapsedHeight, animSpec)
            }
        },
        onDragCancel = {
            snapJob?.cancel()
            scope.launch { height.animateTo(bounds.targetHeight, animSpec) }
        },
    )
}

@Composable
internal fun DevicesBottomSheet(
    pins: List<OrbitPinUi>,
    hasBanner: Boolean,
    expanded: Boolean,
    onSetExpanded: (Boolean) -> Unit,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current
    val visiblePins = pins.filter { it.isVisible }

    TactileSurface(
        modifier = Modifier
            .fillMaxWidth()
            .padding(start = KeynearSpacing.Lg, end = KeynearSpacing.Lg, bottom = KeynearSpacing.Lg),
        shape = RoundedCornerShape(KeynearRadii.ThreeXl),
        containerColor = palette.surface,
        tone = TactileTone.SURFACE,
        cornerRadius = KeynearRadii.ThreeXl,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(
                    start = KeynearSpacing.Lg,
                    end = KeynearSpacing.Lg,
                    top = KeynearLayout.ContentGap,
                    bottom = KeynearSpacing.Md,
                ),
            verticalArrangement = Arrangement.spacedBy(KeynearLayout.CompactGap),
        ) {
            Crossfade(
                targetState = visiblePins.isEmpty(),
                animationSpec = tween(KeynearMotion.MORPH_DURATION_MS, easing = KeynearMotion.SpringEasing),
                label = "devicesSheetState",
            ) { isEmpty ->
                if (isEmpty) {
                    SheetHandle()
                    EmptyDevicesSheetContent()
                } else {
                    PopulatedDevicesSheetContent(
                        pins = pins,
                        visibleCount = visiblePins.size,
                        hasBanner = hasBanner,
                        expanded = expanded,
                        onSetExpanded = onSetExpanded,
                        actions = actions,
                    )
                }
            }
        }
    }
}

@Composable
private fun SheetHandle() {
    val palette = LocalKeynearPalette.current
    Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
        Box(
            modifier = Modifier
                .width(KeynearSpacing.MinTouchTarget)
                .height(KeynearLayout.CompactGap)
                .clip(CircleShape)
                .background(palette.sheetHandle),
        )
    }
}

@Composable
private fun PopulatedDevicesSheetContent(
    pins: List<OrbitPinUi>,
    visibleCount: Int,
    hasBanner: Boolean,
    expanded: Boolean,
    onSetExpanded: (Boolean) -> Unit,
    actions: KeynearActions,
) {
    val collapsedHeight = if (hasBanner) 0.dp else KeynearLayout.SheetCollapsedHeight
    val expandedHeight = KeynearLayout.SheetExpandedHeight
    val targetHeight = if (expanded) expandedHeight else collapsedHeight
    val height = remember { Animatable<Dp, AnimationVector1D>(targetHeight, Dp.VectorConverter) }
    val scope = rememberCoroutineScope()
    val animSpec = tween<Dp>(durationMillis = KeynearMotion.MORPH_DURATION_MS, easing = KeynearMotion.SpringEasing)

    LaunchedEffect(targetHeight) {
        height.animateTo(targetHeight, animSpec)
    }

    val bounds = SheetBounds(collapsedHeight, expandedHeight, targetHeight)
    val dragModifier = Modifier.pointerInput(bounds) {
        handleSheetVerticalDrag(bounds, height, scope, onSetExpanded)
    }

    SheetHeaderBar(
        count = visibleCount,
        expanded = expanded,
        onSetExpanded = onSetExpanded,
        modifier = dragModifier,
    )
    if (height.value > 0.dp) {
        val rowEnter = fadeIn(tween(KeynearMotion.MORPH_DURATION_MS, easing = KeynearMotion.SpringEasing)) +
            expandVertically(tween(KeynearMotion.MORPH_DURATION_MS, easing = KeynearMotion.SpringEasing))
        val rowExit = fadeOut(tween(KeynearMotion.MORPH_DURATION_MS, easing = KeynearMotion.SpringEasing)) +
            shrinkVertically(tween(KeynearMotion.MORPH_DURATION_MS, easing = KeynearMotion.SpringEasing))
        LazyColumn(
            modifier = Modifier
                .fillMaxWidth()
                .heightIn(max = height.value),
            verticalArrangement = Arrangement.spacedBy(KeynearLayout.CompactGap),
            contentPadding = PaddingValues(bottom = KeynearSpacing.Xs),
        ) {
            items(pins, key = { it.id }) { pin ->
                AnimatedVisibility(visible = pin.isVisible, enter = rowEnter, exit = rowExit) {
                    DeviceSheetRow(pin = pin, actions = actions)
                }
            }
        }
    }
}

@Composable
private fun SheetHeaderBar(
    count: Int,
    expanded: Boolean,
    onSetExpanded: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    Column(
        modifier = modifier
            .fillMaxWidth()
            .heightIn(min = KeynearLayout.EmptyBadgeSize)
            .padding(bottom = KeynearLayout.CompactGap),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        SheetHandle()
        Spacer(modifier = Modifier.height(KeynearLayout.ContentGap))
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Text(
                text = if (count == 1) "1 computer in range" else "$count computers in range",
                color = palette.textPrimary,
                fontSize = KeynearTypeSizes.Description,
                fontWeight = FontWeight.Bold,
            )
            Text(
                text = if (expanded) "Collapse" else "All devices",
                color = palette.loadingInk,
                fontSize = KeynearTypeSizes.Description,
                fontWeight = FontWeight.Bold,
                modifier = Modifier.clickable(role = Role.Button) { onSetExpanded(!expanded) },
            )
        }
    }
}
