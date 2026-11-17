package org.keynear.app.ui.components

import android.graphics.BlurMaskFilter
import android.graphics.Paint as AndroidPaint
import android.graphics.Path as AndroidPath
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.app.core.BluetoothStatus
import org.keynear.app.core.CoreSnapshot
import org.keynear.app.core.LocalNetworkStatus
import org.keynear.app.core.PendingPairingUi
import org.keynear.app.core.TransferDirectionUi
import org.keynear.app.core.TransferStateUi
import org.keynear.app.core.TransferUi
import org.keynear.app.core.formatBytes
import org.keynear.app.ui.navigation.KeynearActions
import org.keynear.app.ui.theme.KeynearElevation
import org.keynear.app.ui.theme.KeynearLayout
import org.keynear.app.ui.theme.KeynearRadii
import org.keynear.app.ui.theme.KeynearShapes
import org.keynear.app.ui.theme.KeynearSpacing
import org.keynear.app.ui.theme.KeynearTypeSizes
import org.keynear.app.ui.theme.LocalKeynearPalette
import kotlin.math.abs

private val ActionButtonHorizontalPadding = 20.dp
private val ControlMinHeight = 52.dp
private val SwitchTrackWidth = 58.dp
private val CaptionLetterSpacing = 0.8.sp
private val CodeLetterSpacing = 4.sp
private val CompactGap = 6.dp
private val ContentGap = 10.dp
private val DefaultElevation = 5.dp
private val SelectionTileMinHeight = 128.dp
private val SwitchDragThreshold = 18.dp
private val SwitchMinWidth = 62.dp
private val SwitchThumbRadius = 13.dp
private val SwitchThumbSize = 26.dp
private val SwitchTrackHeight = 34.dp
private val SwitchTrackRadius = 17.dp
private val TextSubtitleGap = 2.dp
private val TransferActionMinHeight = 44.dp
private val DisabledContainerAlpha = 0.45f
private val DisabledContentAlpha = 0.65f


enum class TactileTone {
    SURFACE,
    TONAL,
    ACCENT,
    SHEET,
    INSET,
    THUMB,
    NONE,
}

private data class ShadowSpec(
    val offsetY: Dp,
    val blurRadius: Dp,
    val spread: Dp,
    val color: Color,
)

private data class InsetShadowSpec(
    val offsetY: Dp,
    val blurRadius: Dp,
    val color: Color,
)

private fun outerShadowSpecs(tone: TactileTone, isDark: Boolean): List<ShadowSpec> {
    val specs = when (tone) {
        TactileTone.SURFACE -> if (isDark) KeynearElevation.OuterSurfaceDark else KeynearElevation.OuterSurfaceLight
        TactileTone.TONAL -> if (isDark) KeynearElevation.OuterTonalDark else KeynearElevation.OuterTonalLight
        TactileTone.ACCENT -> if (isDark) KeynearElevation.OuterAccentDark else KeynearElevation.OuterAccentLight
        TactileTone.SHEET -> if (isDark) KeynearElevation.OuterSheetDark else KeynearElevation.OuterSheetLight
        TactileTone.THUMB -> if (isDark) KeynearElevation.OuterThumbDark else KeynearElevation.OuterThumbLight
        TactileTone.INSET, TactileTone.NONE -> KeynearElevation.OuterNone
    }
    return specs.map { ShadowSpec(it.offsetY, it.blurRadius, it.spread, it.color) }
}

private fun insetShadowSpecs(tone: TactileTone, isDark: Boolean): List<InsetShadowSpec> {
    val specs = when (tone) {
        TactileTone.SURFACE -> if (isDark) KeynearElevation.InsetSurfaceDark else KeynearElevation.InsetSurfaceLight
        TactileTone.TONAL -> if (isDark) KeynearElevation.InsetTonalDark else KeynearElevation.InsetTonalLight
        TactileTone.ACCENT -> if (isDark) KeynearElevation.InsetAccentDark else KeynearElevation.InsetAccentLight
        TactileTone.SHEET -> if (isDark) KeynearElevation.InsetSheetDark else KeynearElevation.InsetSheetLight
        TactileTone.THUMB -> if (isDark) KeynearElevation.InsetThumbDark else KeynearElevation.InsetThumbLight
        TactileTone.INSET -> if (isDark) KeynearElevation.InsetInsetDark else KeynearElevation.InsetInsetLight
        TactileTone.NONE -> KeynearElevation.InsetNone
    }
    return specs.map { InsetShadowSpec(it.offsetY, it.blurRadius, it.color) }
}

fun Modifier.tactileShadow(
    tone: TactileTone,
    isDark: Boolean,
    cornerRadius: Dp = KeynearRadii.TwoXl,
    isCircle: Boolean = false,
): Modifier {
    val specs = outerShadowSpecs(tone, isDark)
    if (specs.isEmpty()) return this
    return drawBehind {
        drawIntoCanvas { canvas ->
            val native = canvas.nativeCanvas
            val radiusPx = cornerRadius.toPx()
            specs.forEach { spec ->
                val blurPx = spec.blurRadius.toPx()
                val spreadPx = spec.spread.toPx()
                val offsetYPx = spec.offsetY.toPx()
                val paint = AndroidPaint(AndroidPaint.ANTI_ALIAS_FLAG).apply {
                    color = spec.color.toArgb()
                    if (blurPx > 0.5f) {
                        maskFilter = BlurMaskFilter(blurPx, BlurMaskFilter.Blur.NORMAL)
                    }
                }
                if (isCircle) {
                    val cx = size.width * 0.5f
                    val cy = size.height * 0.5f + offsetYPx
                    val r = (size.minDimension * 0.5f + spreadPx).coerceAtLeast(1f)
                    native.drawCircle(cx, cy, r, paint)
                } else if (tone == TactileTone.SHEET) {
                    native.save()
                    native.clipRect(-200f, -200f, size.width + 200f, size.height)
                    val path = AndroidPath().apply {
                        addRoundRect(
                            -spreadPx,
                            -spreadPx + offsetYPx,
                            size.width + spreadPx,
                            size.height + spreadPx,
                            floatArrayOf(radiusPx, radiusPx, radiusPx, radiusPx, 0f, 0f, 0f, 0f),
                            AndroidPath.Direction.CW,
                        )
                    }
                    native.drawPath(path, paint)
                    native.restore()
                } else {
                    native.drawRoundRect(
                        -spreadPx,
                        -spreadPx + offsetYPx,
                        size.width + spreadPx,
                        size.height + spreadPx + offsetYPx,
                        radiusPx,
                        radiusPx,
                        paint,
                    )
                }
            }
        }
    }
}

fun Modifier.tactileCushion(
    tone: TactileTone,
    isDark: Boolean,
    cornerRadius: Dp = KeynearRadii.TwoXl,
    isCircle: Boolean = false,
): Modifier {
    val specs = insetShadowSpecs(tone, isDark)
    if (specs.isEmpty()) return this
    return drawWithContent {
        val radiusPx = if (isCircle) {
            size.minDimension * 0.5f
        } else {
            cornerRadius.toPx().coerceAtMost(size.minDimension * 0.5f)
        }
        drawIntoCanvas { canvas ->
            val native = canvas.nativeCanvas
            specs.forEach { spec ->
                val blurPx = spec.blurRadius.toPx()
                val offsetYPx = spec.offsetY.toPx()
                val pad = blurPx * 2f + abs(offsetYPx) + 8f
                val path = AndroidPath().apply {
                    fillType = AndroidPath.FillType.EVEN_ODD
                    addRect(-pad, -pad, size.width + pad, size.height + pad, AndroidPath.Direction.CW)
                    val bottomRadius = if (tone == TactileTone.SHEET) 0f else radiusPx
                    addRoundRect(
                        0f,
                        offsetYPx,
                        size.width,
                        size.height + offsetYPx,
                        floatArrayOf(radiusPx, radiusPx, radiusPx, radiusPx, bottomRadius, bottomRadius, bottomRadius, bottomRadius),
                        AndroidPath.Direction.CW,
                    )
                }
                val paint = AndroidPaint(AndroidPaint.ANTI_ALIAS_FLAG).apply {
                    color = spec.color.toArgb()
                    if (blurPx > 0.4f) {
                        maskFilter = BlurMaskFilter(blurPx, BlurMaskFilter.Blur.NORMAL)
                    }
                }
                native.drawPath(path, paint)
            }
        }
        drawContent()
    }
}

@Composable
fun TactileSurface(
    modifier: Modifier = Modifier,
    shape: Shape = KeynearShapes.Card,
    containerColor: Color = LocalKeynearPalette.current.surface,
    shadowElevation: Dp = DefaultElevation,
    tone: TactileTone = if (shadowElevation == 0.dp) TactileTone.NONE else TactileTone.SURFACE,
    isAccent: Boolean = false,
    cornerRadius: Dp = KeynearRadii.TwoXl,
    isCircle: Boolean = false,
    onClick: (() -> Unit)? = null,
    enabled: Boolean = true,
    content: @Composable () -> Unit,
) {
    val isDark = LocalKeynearPalette.current.isDark
    val effectiveTone = if (isAccent) TactileTone.ACCENT else tone
    Box(
        modifier = modifier
            .tactileShadow(effectiveTone, isDark, cornerRadius, isCircle)
            .clip(shape)
            .background(containerColor)
            .tactileCushion(effectiveTone, isDark, cornerRadius, isCircle)
            .let { m -> if (onClick != null && enabled) m.clickable(onClick = onClick) else m },
    ) { content() }
}

@Composable
fun SectionHeading(
    title: String,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    Text(
        text = title.uppercase(),
        color = palette.textMuted,
        fontSize = KeynearTypeSizes.Caption,
        fontWeight = FontWeight.Bold,
        letterSpacing = CaptionLetterSpacing,
        modifier = modifier.padding(horizontal = KeynearSpacing.Xs),
    )
}

@Composable
fun KeynearSwitch(
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    val trackColor by animateColorAsState(
        targetValue = if (checked) palette.loading else palette.switchTrackOff,
        label = "switchTrack",
    )
    val thumbOffset by animateDpAsState(
        targetValue = if (checked) KeynearSpacing.Xl else 0.dp,
        label = "switchThumb",
    )
    val trackTone = if (checked) TactileTone.ACCENT else TactileTone.INSET

    Box(
        modifier = modifier
            .defaultMinSize(minWidth = SwitchMinWidth, minHeight = ControlMinHeight)
            .pointerInput(checked, onCheckedChange) {
                var dragOffset = 0f
                detectHorizontalDragGestures(
                    onDragStart = { dragOffset = 0f },
                    onHorizontalDrag = { change, dragAmount ->
                        change.consume()
                        dragOffset += dragAmount
                        val threshold = SwitchDragThreshold.toPx()
                        when {
                            dragOffset > threshold && !checked -> {
                                onCheckedChange(true)
                                dragOffset = 0f
                            }
                            dragOffset < -threshold && checked -> {
                                onCheckedChange(false)
                                dragOffset = 0f
                            }
                        }
                    },
                    onDragEnd = { dragOffset = 0f },
                    onDragCancel = { dragOffset = 0f },
                )
            }
            .clickable(role = Role.Switch) { onCheckedChange(!checked) },
        contentAlignment = Alignment.CenterEnd,
    ) {
        Box(
            modifier = Modifier
                .width(SwitchTrackWidth)
                .height(SwitchTrackHeight)
                .tactileShadow(
                    tone = trackTone,
                    isDark = palette.isDark,
                    cornerRadius = SwitchTrackRadius,
                )
                .clip(CircleShape)
                .background(trackColor)
                .tactileCushion(tone = trackTone, isDark = palette.isDark, cornerRadius = SwitchTrackRadius)
                .padding(KeynearSpacing.Xs),
            contentAlignment = Alignment.CenterStart,
        ) {
            Box(
                modifier = Modifier
                    .offset(x = thumbOffset)
                    .size(SwitchThumbSize)
                    .tactileShadow(
                        tone = TactileTone.THUMB,
                        isDark = palette.isDark,
                        cornerRadius = SwitchThumbRadius,
                        isCircle = true,
                    )
                    .clip(CircleShape)
                    .background(palette.switchThumb)
                    .tactileCushion(
                        tone = TactileTone.THUMB,
                        isDark = palette.isDark,
                        cornerRadius = SwitchThumbRadius,
                        isCircle = true,
                    ),
            )
        }
    }
}

@Composable
fun SelectionTile(
    label: String,
    active: Boolean = false,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    icon: @Composable (Color) -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val bg = if (active) palette.loadingTonal else palette.surface
    val iconColor = if (active) palette.onLoadingTonal else palette.textSecondary
    val textColor = if (active) palette.onLoadingTonal else palette.textPrimary

    TactileSurface(
        modifier = modifier.heightIn(min = SelectionTileMinHeight),
        shape = KeynearShapes.Card,
        containerColor = bg,
        tone = if (active) TactileTone.TONAL else TactileTone.SURFACE,
        cornerRadius = KeynearRadii.TwoXl,
        onClick = onClick,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .heightIn(min = SelectionTileMinHeight)
                .padding(KeynearLayout.CardPaddingLarge),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            icon(iconColor)
            Spacer(modifier = Modifier.height(KeynearSpacing.Md))
            Text(
                text = label,
                color = textColor,
                fontSize = KeynearTypeSizes.Body,
                fontWeight = FontWeight.Bold,
            )
        }
    }
}

@Composable
fun SettingsSection(
    title: String,
    modifier: Modifier = Modifier,
    content: @Composable ColumnScope.() -> Unit,
) {
    val palette = LocalKeynearPalette.current
    Column(
        modifier = modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(KeynearLayout.SectionHeadingBottom),
    ) {
        SectionHeading(title = title)
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
                    .padding(KeynearLayout.SectionInnerPadding),
                verticalArrangement = Arrangement.spacedBy(KeynearLayout.SectionRowGap),
                content = content,
            )
        }
    }
}

@Composable
fun SettingRow(
    title: String,
    subtitle: String? = null,
    onClick: (() -> Unit)? = null,
    trailing: @Composable () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    val baseModifier = Modifier
        .fillMaxWidth()
        .defaultMinSize(minHeight = KeynearLayout.RowMinHeight)
        .clip(KeynearShapes.Xl)
        .let { m -> if (onClick != null) m.clickable(onClick = onClick) else m }
        .padding(
            horizontal = KeynearLayout.RowHorizontalPadding,
            vertical = KeynearLayout.RowVerticalPadding,
        )

    Row(
        modifier = baseModifier,
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
    ) {
        Column(
            modifier = Modifier
                .weight(1f)
                .padding(end = KeynearSpacing.Sm),
            verticalArrangement = Arrangement.spacedBy(TextSubtitleGap),
        ) {
            Text(
                text = title,
                color = palette.textPrimary,
                fontSize = KeynearTypeSizes.Body,
                fontWeight = FontWeight.SemiBold,
            )
            if (!subtitle.isNullOrBlank()) {
                Text(
                    text = subtitle,
                    color = palette.textMuted,
                    fontSize = KeynearTypeSizes.Caption,
                )
            }
        }
        trailing()
    }
}

@Composable
fun ActionButton(
    label: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    primary: Boolean = false,
    danger: Boolean = false,
    errorTonal: Boolean = false,
    tonalSurface: Boolean = false,
    enabled: Boolean = true,
    minHeight: Dp = KeynearSpacing.MinTouchTarget,
    leadingIcon: (@Composable (Color) -> Unit)? = null,
) {
    val palette = LocalKeynearPalette.current
    val container = when {
        danger -> palette.error
        errorTonal -> palette.errorAction
        tonalSurface -> palette.surfaceTonal
        primary -> palette.loading
        else -> palette.loadingTonal
    }
    val content = when {
        danger || primary -> palette.onAccent
        errorTonal -> palette.onDangerAction
        tonalSurface -> palette.textPrimary
        else -> palette.onLoadingTonal
    }
    val effectiveContainer = if (enabled) container else container.copy(alpha = DisabledContainerAlpha)
    val effectiveContent = if (enabled) content else content.copy(alpha = DisabledContentAlpha)
    val buttonTone = when {
        !enabled -> TactileTone.NONE
        primary -> TactileTone.ACCENT
        else -> TactileTone.TONAL
    }

    TactileSurface(
        modifier = modifier.heightIn(min = minHeight),
        shape = KeynearShapes.Xl,
        containerColor = effectiveContainer,
        tone = buttonTone,
        cornerRadius = KeynearRadii.Xl,
        onClick = onClick,
        enabled = enabled,
    ) {
        Row(
            modifier = Modifier
                .heightIn(min = minHeight)
                .padding(horizontal = ActionButtonHorizontalPadding, vertical = ContentGap),
            horizontalArrangement = Arrangement.Center,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (leadingIcon != null) {
                leadingIcon(effectiveContent)
                Spacer(modifier = Modifier.width(KeynearSpacing.Sm))
            }
            Text(
                text = label,
                color = effectiveContent,
                fontSize = KeynearTypeSizes.Description,
                fontWeight = FontWeight.Bold,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

@Composable
fun RecessedInput(
    value: String,
    onValueChange: (String) -> Unit,
    placeholder: String,
    modifier: Modifier = Modifier,
    singleLine: Boolean = true,
) {
    val palette = LocalKeynearPalette.current
    Box(
        modifier = modifier
            .heightIn(min = ControlMinHeight)
            .clip(KeynearShapes.Xl)
            .background(palette.input)
            .tactileCushion(
                tone = TactileTone.INSET,
                isDark = palette.isDark,
                cornerRadius = KeynearRadii.Xl,
            )
            .padding(horizontal = KeynearSpacing.Lg, vertical = KeynearSpacing.Md),
        contentAlignment = Alignment.CenterStart,
    ) {
        if (value.isEmpty()) {
            Text(
                text = placeholder,
                color = palette.textMuted,
                fontSize = KeynearTypeSizes.Body,
            )
        }
        BasicTextField(
            value = value,
            onValueChange = onValueChange,
            singleLine = singleLine,
            cursorBrush = SolidColor(palette.loading),
            textStyle = TextStyle(
                color = palette.textPrimary,
                fontSize = KeynearTypeSizes.Body,
                fontWeight = FontWeight.Medium,
            ),
            modifier = Modifier.fillMaxWidth(),
        )
    }
}

@Composable
fun TransportWarningBanner(
    snapshot: CoreSnapshot,
    actions: KeynearActions,
) {
    val palette = LocalKeynearPalette.current
    val (msg, ctaLabel, onClick) = when {
        snapshot.bluetoothStatus == BluetoothStatus.MISSING_PERMISSION ->
            Triple("Bluetooth permission required for discovery", "Grant", actions.onRequestBluetoothPermissions)
        snapshot.bluetoothStatus == BluetoothStatus.ADAPTER_DISABLED ->
            Triple("Bluetooth is turned off", "Settings", actions.onOpenBluetoothSettings)
        snapshot.localNetworkStatus == LocalNetworkStatus.MISSING_PERMISSION ->
            Triple("Local network permission required", "Allow", actions.onRequestLocalNetworkPermission)
        snapshot.localNetworkStatus == LocalNetworkStatus.BLOCKED_LNP ->
            Triple(snapshot.localNetworkMessage ?: "Local network discovery blocked", "Retry", actions.onRequestLocalNetworkPermission)
        else -> return
    }

    TactileSurface(
        modifier = Modifier.fillMaxWidth(),
        shape = KeynearShapes.Card,
        containerColor = palette.warningTonal,
        tone = TactileTone.TONAL,
        cornerRadius = KeynearRadii.TwoXl,
        onClick = onClick,
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = KeynearSpacing.Lg, vertical = KeynearSpacing.Md),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(ContentGap),
        ) {
            Text(
                text = msg,
                color = palette.onWarningTonal,
                fontSize = KeynearTypeSizes.Caption,
                fontWeight = FontWeight.Medium,
                modifier = Modifier.weight(1f),
            )
            Text(
                text = ctaLabel,
                color = palette.onWarningTonal,
                fontSize = KeynearTypeSizes.Caption,
                fontWeight = FontWeight.Bold,
            )
        }
    }
}

@Composable
fun PendingPairingBanner(
    prompt: PendingPairingUi,
    onApprove: () -> Unit,
    onReject: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    TactileSurface(
        modifier = modifier.fillMaxWidth(),
        shape = KeynearShapes.Card,
        containerColor = palette.surface,
        tone = TactileTone.SURFACE,
        cornerRadius = KeynearRadii.TwoXl,
    ) {
        Column(
            modifier = Modifier.padding(KeynearLayout.CardPadding),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
        ) {
            Text(
                text = "PAIR WITH ${prompt.peerName.uppercase()}",
                color = palette.textMuted,
                fontSize = KeynearTypeSizes.Caption,
                fontWeight = FontWeight.Bold,
                letterSpacing = CaptionLetterSpacing,
            )
            Text(
                text = prompt.verificationCode,
                color = palette.textPrimary,
                fontSize = KeynearTypeSizes.Title,
                fontWeight = FontWeight.Bold,
                letterSpacing = CodeLetterSpacing,
            )
            Text(
                text = "Confirm this code matches (${prompt.remainingSecs}s)",
                color = palette.textMuted,
                fontSize = KeynearTypeSizes.Caption,
            )
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
            ) {
                ActionButton(
                    label = "Decline",
                    onClick = onReject,
                    errorTonal = true,
                    modifier = Modifier.weight(1f),
                )
                ActionButton(
                    label = "Pair",
                    onClick = onApprove,
                    primary = true,
                    modifier = Modifier.weight(1f),
                )
            }
        }
    }
}

@Composable
fun ActiveTransfersStrip(
    transfers: List<TransferUi>,
    onAccept: (String) -> Unit,
    onDecline: (String) -> Unit,
    onCancel: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val palette = LocalKeynearPalette.current
    TactileSurface(
        modifier = modifier.fillMaxWidth(),
        shape = KeynearShapes.Card,
        containerColor = palette.surface,
        tone = TactileTone.SURFACE,
        cornerRadius = KeynearRadii.TwoXl,
    ) {
        Column(
            modifier = Modifier.padding(KeynearLayout.CardPadding),
            verticalArrangement = Arrangement.spacedBy(ContentGap),
        ) {
            transfers.take(2).forEach { tx ->
                Column(verticalArrangement = Arrangement.spacedBy(CompactGap)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = tx.summaryName,
                                color = palette.textPrimary,
                                fontSize = KeynearTypeSizes.Description,
                                fontWeight = FontWeight.SemiBold,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                            val dir = if (tx.direction == TransferDirectionUi.SEND) "To" else "From"
                            Text(
                                text = "$dir ${tx.peerDisplayName} · ${formatBytes(tx.totalBytes)}",
                                color = palette.textMuted,
                                fontSize = KeynearTypeSizes.Caption,
                            )
                        }
                        when (tx.state) {
                            TransferStateUi.PENDING_CONSENT -> {
                                ActionButton(
                                    label = "Decline",
                                    onClick = { onDecline(tx.transferId) },
                                    errorTonal = true,
                                    minHeight = TransferActionMinHeight,
                                )
                                ActionButton(
                                    label = "Accept",
                                    onClick = { onAccept(tx.transferId) },
                                    primary = true,
                                    minHeight = TransferActionMinHeight,
                                )
                            }
                            TransferStateUi.ACTIVE -> {
                                ActionButton(
                                    label = "Cancel",
                                    onClick = { onCancel(tx.transferId) },
                                    errorTonal = true,
                                    minHeight = TransferActionMinHeight,
                                )
                            }
                            else -> {}
                        }
                    }
                    val fraction = if (tx.totalBytes > 0L) {
                        (tx.transferredBytes.toFloat() / tx.totalBytes.toFloat()).coerceIn(0f, 1f)
                    } else 0f
                    LinearProgressIndicator(
                        progress = { fraction },
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(CompactGap)
                            .clip(CircleShape),
                        color = palette.loading,
                        trackColor = palette.switchTrackOff,
                    )
                }
            }
        }
    }
}
