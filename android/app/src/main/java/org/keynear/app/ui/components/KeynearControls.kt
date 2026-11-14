package org.keynear.app.ui.components

import android.graphics.BlurMaskFilter
import android.graphics.Paint as AndroidPaint
import android.graphics.Path as AndroidPath
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
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
import org.keynear.app.ui.theme.KeynearLayout
import org.keynear.app.ui.theme.KeynearRadii
import org.keynear.app.ui.theme.KeynearShapes
import org.keynear.app.ui.theme.LocalKeynearPalette
import kotlin.math.abs

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

private fun outerShadowSpecs(tone: TactileTone, isDark: Boolean): List<ShadowSpec> = when (tone) {
    TactileTone.SURFACE -> if (isDark) {
        listOf(
            ShadowSpec(6.dp, 11.2.dp, (-3).dp, Color(0x73000000)),
            ShadowSpec(2.dp, 2.6.dp, (-1).dp, Color(0x40000000)),
        )
    } else {
        listOf(
            ShadowSpec(5.dp, 9.5.dp, (-3).dp, Color(0x1405112F)),
            ShadowSpec(2.dp, 2.6.dp, (-1).dp, Color(0x0A05112F)),
        )
    }
    TactileTone.TONAL -> if (isDark) {
        listOf(ShadowSpec(4.dp, 7.8.dp, (-2).dp, Color(0x5E000000)))
    } else {
        listOf(ShadowSpec(3.dp, 6.dp, (-2).dp, Color(0x1205112F)))
    }
    TactileTone.ACCENT -> if (isDark) {
        listOf(ShadowSpec(6.dp, 13.dp, (-3).dp, Color(0x5E0053E2)))
    } else {
        listOf(ShadowSpec(6.dp, 11.2.dp, (-3).dp, Color(0x52003DCF)))
    }
    TactileTone.SHEET -> if (isDark) {
        listOf(ShadowSpec((-8).dp, 19.5.dp, (-4).dp, Color(0x80000000)))
    } else {
        listOf(ShadowSpec((-6).dp, 16.5.dp, (-4).dp, Color(0x1405112F)))
    }
    TactileTone.THUMB -> if (isDark) {
        listOf(ShadowSpec(2.dp, 4.3.dp, 0.dp, Color(0x7A000000)))
    } else {
        listOf(ShadowSpec(2.dp, 3.6.dp, 0.dp, Color(0x3805112F)))
    }
    TactileTone.INSET, TactileTone.NONE -> emptyList()
}

private fun insetShadowSpecs(tone: TactileTone, isDark: Boolean): List<InsetShadowSpec> = when (tone) {
    TactileTone.SURFACE -> if (isDark) listOf(
        InsetShadowSpec(1.5.dp, 0.9.dp, Color(0x14FFFFFE)),
        InsetShadowSpec((-2).dp, 2.6.dp, Color(0x80000000)),
    ) else listOf(
        InsetShadowSpec(1.5.dp, 0.9.dp, Color(0xF2FFFFFE)),
        InsetShadowSpec((-2).dp, 1.8.dp, Color(0x0F05112F)),
    )
    TactileTone.TONAL -> if (isDark) listOf(
        InsetShadowSpec(1.5.dp, 0.9.dp, Color(0x12FFFFFE)),
        InsetShadowSpec((-1.5).dp, 1.8.dp, Color(0x73000000)),
    ) else listOf(
        InsetShadowSpec(1.5.dp, 0.9.dp, Color(0xCCFFFFFE)),
        InsetShadowSpec((-1.5).dp, 1.5.dp, Color(0x1405112F)),
    )
    TactileTone.ACCENT -> if (isDark) listOf(
        InsetShadowSpec(1.8.dp, 1.1.dp, Color(0x52FFFFFE)),
        InsetShadowSpec((-2.5).dp, 2.6.dp, Color(0x80000000)),
    ) else listOf(
        InsetShadowSpec(1.8.dp, 1.1.dp, Color(0x70FFFFFE)),
        InsetShadowSpec((-2.5).dp, 2.6.dp, Color(0x61000269)),
    )
    TactileTone.SHEET -> if (isDark) listOf(
        InsetShadowSpec(1.5.dp, 0.9.dp, Color(0x17FFFFFE)),
    ) else listOf(
        InsetShadowSpec(2.dp, 1.1.dp, Color(0xF2FFFFFE)),
    )
    TactileTone.THUMB -> if (isDark) listOf(
        InsetShadowSpec(1.5.dp, 0.9.dp, Color(0xE6FFFFFE)),
        InsetShadowSpec((-1.5).dp, 1.2.dp, Color(0x40000000)),
    ) else listOf(
        InsetShadowSpec(1.5.dp, 0.9.dp, Color(0xFFFFFFFF)),
        InsetShadowSpec((-1.5).dp, 1.2.dp, Color(0x2605112F)),
    )
    TactileTone.INSET -> if (isDark) listOf(
        InsetShadowSpec(2.2.dp, 3.2.dp, Color(0x99000000)),
        InsetShadowSpec((-1.2).dp, 0.9.dp, Color(0x1FFFFFFE)),
    ) else listOf(
        InsetShadowSpec(2.2.dp, 2.6.dp, Color(0x2405112F)),
        InsetShadowSpec((-1.2).dp, 0.9.dp, Color(0xE6FFFFFE)),
    )
    TactileTone.NONE -> emptyList()
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
    shadowElevation: Dp = 5.dp,
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
        fontSize = 12.sp,
        fontWeight = FontWeight.Bold,
        letterSpacing = 0.8.sp,
        modifier = modifier.padding(horizontal = 4.dp),
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
        targetValue = if (checked) 24.dp else 0.dp,
        label = "switchThumb",
    )
    val trackTone = if (checked) TactileTone.ACCENT else TactileTone.INSET

    Box(
        modifier = modifier
            .defaultMinSize(minWidth = 62.dp, minHeight = 52.dp)
            .clickable(role = Role.Switch) { onCheckedChange(!checked) },
        contentAlignment = Alignment.CenterEnd,
    ) {
        Box(
            modifier = Modifier
                .width(58.dp)
                .height(34.dp)
                .tactileShadow(
                    tone = trackTone,
                    isDark = palette.isDark,
                    cornerRadius = 17.dp,
                )
                .clip(CircleShape)
                .background(trackColor)
                .tactileCushion(tone = trackTone, isDark = palette.isDark, cornerRadius = 17.dp)
                .padding(4.dp),
            contentAlignment = Alignment.CenterStart,
        ) {
            Box(
                modifier = Modifier
                    .offset(x = thumbOffset)
                    .size(26.dp)
                    .tactileShadow(
                        tone = TactileTone.THUMB,
                        isDark = palette.isDark,
                        cornerRadius = 13.dp,
                        isCircle = true,
                    )
                    .clip(CircleShape)
                    .background(palette.switchThumb)
                    .tactileCushion(
                        tone = TactileTone.THUMB,
                        isDark = palette.isDark,
                        cornerRadius = 13.dp,
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
        modifier = modifier.heightIn(min = 128.dp),
        shape = KeynearShapes.Card,
        containerColor = bg,
        tone = if (active) TactileTone.TONAL else TactileTone.SURFACE,
        cornerRadius = KeynearRadii.TwoXl,
        onClick = onClick,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .heightIn(min = 128.dp)
                .padding(KeynearLayout.CardPaddingLarge),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            icon(iconColor)
            Spacer(modifier = Modifier.height(12.dp))
            Text(
                text = label,
                color = textColor,
                fontSize = 16.sp,
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
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Column(
            modifier = Modifier
                .weight(1f)
                .padding(end = 8.dp),
            verticalArrangement = Arrangement.spacedBy(2.dp),
        ) {
            Text(
                text = title,
                color = palette.textPrimary,
                fontSize = 16.sp,
                fontWeight = FontWeight.SemiBold,
            )
            if (!subtitle.isNullOrBlank()) {
                Text(
                    text = subtitle,
                    color = palette.textMuted,
                    fontSize = 12.sp,
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
    minHeight: Dp = 48.dp,
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
        danger || primary -> Color(0xFFFFFFFE)
        errorTonal -> palette.onDangerAction
        tonalSurface -> palette.textPrimary
        else -> palette.onLoadingTonal
    }
    val effectiveContainer = if (enabled) container else container.copy(alpha = 0.45f)
    val effectiveContent = if (enabled) content else content.copy(alpha = 0.65f)
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
                .padding(horizontal = 20.dp, vertical = 10.dp),
            horizontalArrangement = Arrangement.Center,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (leadingIcon != null) {
                leadingIcon(effectiveContent)
                Spacer(modifier = Modifier.width(8.dp))
            }
            Text(
                text = label,
                color = effectiveContent,
                fontSize = 14.sp,
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
            .heightIn(min = 52.dp)
            .clip(KeynearShapes.Xl)
            .background(palette.input)
            .tactileCushion(
                tone = TactileTone.INSET,
                isDark = palette.isDark,
                cornerRadius = KeynearRadii.Xl,
            )
            .padding(horizontal = 16.dp, vertical = 12.dp),
        contentAlignment = Alignment.CenterStart,
    ) {
        if (value.isEmpty()) {
            Text(
                text = placeholder,
                color = palette.textMuted,
                fontSize = 16.sp,
            )
        }
        BasicTextField(
            value = value,
            onValueChange = onValueChange,
            singleLine = singleLine,
            cursorBrush = SolidColor(palette.loading),
            textStyle = TextStyle(
                color = palette.textPrimary,
                fontSize = 16.sp,
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
                .padding(horizontal = 16.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Text(
                text = msg,
                color = palette.onWarningTonal,
                fontSize = 12.sp,
                fontWeight = FontWeight.Medium,
                modifier = Modifier.weight(1f),
            )
            Text(
                text = ctaLabel,
                color = palette.onWarningTonal,
                fontSize = 12.sp,
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
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(
                text = "PAIR WITH ${prompt.peerName.uppercase()}",
                color = palette.textMuted,
                fontSize = 12.sp,
                fontWeight = FontWeight.Bold,
                letterSpacing = 0.8.sp,
            )
            Text(
                text = prompt.verificationCode,
                color = palette.textPrimary,
                fontSize = 24.sp,
                fontWeight = FontWeight.Bold,
                letterSpacing = 4.sp,
            )
            Text(
                text = "Confirm this code matches (${prompt.remainingSecs}s)",
                color = palette.textMuted,
                fontSize = 12.sp,
            )
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
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
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            transfers.take(2).forEach { tx ->
                Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = tx.summaryName,
                                color = palette.textPrimary,
                                fontSize = 14.sp,
                                fontWeight = FontWeight.SemiBold,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                            val dir = if (tx.direction == TransferDirectionUi.SEND) "To" else "From"
                            Text(
                                text = "$dir ${tx.peerDisplayName} · ${formatBytes(tx.totalBytes)}",
                                color = palette.textMuted,
                                fontSize = 12.sp,
                            )
                        }
                        when (tx.state) {
                            TransferStateUi.PENDING_CONSENT -> {
                                ActionButton(
                                    label = "Decline",
                                    onClick = { onDecline(tx.transferId) },
                                    errorTonal = true,
                                    minHeight = 44.dp,
                                )
                                ActionButton(
                                    label = "Accept",
                                    onClick = { onAccept(tx.transferId) },
                                    primary = true,
                                    minHeight = 44.dp,
                                )
                            }
                            TransferStateUi.ACTIVE -> {
                                ActionButton(
                                    label = "Cancel",
                                    onClick = { onCancel(tx.transferId) },
                                    errorTonal = true,
                                    minHeight = 44.dp,
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
                            .height(6.dp)
                            .clip(CircleShape),
                        color = palette.loading,
                        trackColor = palette.switchTrackOff,
                    )
                }
            }
        }
    }
}
