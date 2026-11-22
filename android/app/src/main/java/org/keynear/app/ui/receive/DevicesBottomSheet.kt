package org.keynear.app.ui.receive

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.AnimationVector1D
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.spring
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
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.app.ui.navigation.OrbitPinUi
import org.keynear.designsystem.KeynearRadii
import org.keynear.designsystem.KeynearShapes
import org.keynear.designsystem.KeynearSpacing
import org.keynear.designsystem.KeynearTypeSizes
import org.keynear.designsystem.LocalKeynearPalette
import org.keynear.designsystem.TactileSurface
import org.keynear.designsystem.TactileTone

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
internal fun DevicesBottomSheet(
    pins: List<OrbitPinUi>,
    expanded: Boolean,
    onSetExpanded: (Boolean) -> Unit,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current
    val density = LocalDensity.current
    val collapsedHeight = if (pins.isEmpty()) 0.dp else SheetCollapsedHeight
    val expandedHeight = if (pins.isEmpty()) EmptySheetExpandedHeight else SheetExpandedHeight
    val targetHeight = if (expanded) expandedHeight else collapsedHeight
    val height = remember {
        Animatable<Dp, AnimationVector1D>(targetHeight, Dp.VectorConverter)
    }
    val scope = rememberCoroutineScope()
    val currentHeight = height.value

    LaunchedEffect(targetHeight) {
        height.animateTo(targetHeight, spring(stiffness = Spring.StiffnessMediumLow))
    }

    val dragModifier = Modifier.pointerInput(collapsedHeight, expandedHeight) {
        var dragHeight = 0.dp
        var snapJob: Job? = null
        detectVerticalDragGestures(
            onDragStart = { dragHeight = height.value },
            onVerticalDrag = { change, dragAmount ->
                change.consume()
                dragHeight = (dragHeight + with(density) { (-dragAmount).toDp() })
                    .coerceIn(0.dp, expandedHeight)
                snapJob?.cancel()
                snapJob = scope.launch { height.snapTo(dragHeight) }
            },
            onDragEnd = {
                snapJob?.cancel()
                val midpoint = (collapsedHeight + expandedHeight) / 2f
                val shouldExpand = dragHeight >= midpoint
                onSetExpanded(shouldExpand)
                scope.launch {
                    height.animateTo(
                        targetValue = if (shouldExpand) expandedHeight else collapsedHeight,
                        animationSpec = spring(stiffness = Spring.StiffnessMediumLow),
                    )
                }
            },
            onDragCancel = {
                snapJob?.cancel()
                scope.launch {
                    height.animateTo(targetHeight, spring(stiffness = Spring.StiffnessMediumLow))
                }
            },
        )
    }

    TactileSurface(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = KeynearSpacing.Lg),
        shape = KeynearShapes.Sheet,
        containerColor = palette.surface.copy(alpha = 0.95f),
        tone = TactileTone.SHEET,
        cornerRadius = KeynearRadii.ThreeXl,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(start = KeynearSpacing.Lg, end = KeynearSpacing.Lg, top = ContentGap, bottom = KeynearSpacing.Md),
            verticalArrangement = Arrangement.spacedBy(CompactGap),
        ) {
            SheetHeaderBar(
                count = pins.size,
                expanded = expanded,
                onSetExpanded = onSetExpanded,
                dragModifier = dragModifier,
            )
            if (currentHeight > 0.dp) {
                if (pins.isEmpty()) {
                    EmptyDevicesSheetContent(currentHeight = currentHeight, dragModifier = dragModifier)
                } else {
                    LazyColumn(
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(currentHeight),
                        verticalArrangement = Arrangement.spacedBy(CompactGap),
                        contentPadding = PaddingValues(bottom = KeynearSpacing.Xs),
                    ) {
                        items(pins, key = { it.id }) { pin ->
                            DeviceSheetRow(pin = pin, actions = actions)
                        }
                    }
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
    dragModifier: Modifier,
) {
    val palette = LocalKeynearPalette.current
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = RowIconSize)
            .padding(bottom = CompactGap)
            .then(dragModifier),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(
            modifier = Modifier
                .width(KeynearSpacing.MinTouchTarget)
                .height(CompactGap)
                .clip(CircleShape)
                .background(palette.sheetHandle),
        )
        Spacer(modifier = Modifier.height(ContentGap))
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
