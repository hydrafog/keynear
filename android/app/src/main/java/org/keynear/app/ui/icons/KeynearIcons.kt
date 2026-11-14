package org.keynear.app.ui.icons

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.dp
import org.keynear.app.ui.theme.LocalKeynearPalette

@Composable
fun IconRadar(
    modifier: Modifier = Modifier.size(24.dp),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = 2f,
) {
    Canvas(modifier = modifier) {
        val sw = strokeWidthDp.dp.toPx()
        val s = size.minDimension
        val c = Offset(s * 0.5f, s * 0.5f)
        drawCircle(tint, radius = s * 0.42f, center = c, style = Stroke(width = sw))
        drawCircle(tint, radius = s * 0.24f, center = c, style = Stroke(width = sw))
        drawCircle(tint, radius = s * 0.08f, center = c)
    }
}

@Composable
fun IconSend(
    modifier: Modifier = Modifier.size(24.dp),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = 2f,
) {
    Canvas(modifier = modifier) {
        val sw = strokeWidthDp.dp.toPx()
        val s = size.minDimension
        val p = Path().apply {
            moveTo(s * 0.90f, s * 0.12f)
            lineTo(s * 0.10f, s * 0.44f)
            lineTo(s * 0.45f, s * 0.56f)
            lineTo(s * 0.56f, s * 0.90f)
            close()
        }
        drawPath(p, color = tint, style = Stroke(width = sw, cap = StrokeCap.Round, join = StrokeJoin.Round))
        drawLine(tint, Offset(s * 0.90f, s * 0.12f), Offset(s * 0.45f, s * 0.56f), strokeWidth = sw, cap = StrokeCap.Round)
    }
}

@Composable
fun IconSliders(
    modifier: Modifier = Modifier.size(24.dp),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = 2f,
) {
    Canvas(modifier = modifier) {
        val sw = strokeWidthDp.dp.toPx()
        val s = size.minDimension
        val ys = listOf(0.25f, 0.5f, 0.75f)
        val kx = listOf(0.35f, 0.68f, 0.42f)
        ys.forEachIndexed { idx, y ->
            drawLine(tint, Offset(s * 0.14f, s * y), Offset(s * 0.86f, s * y), strokeWidth = sw, cap = StrokeCap.Round)
            drawCircle(tint, radius = s * 0.08f, center = Offset(s * kx[idx], s * y))
        }
    }
}

@Composable
fun IconArrowLeft(
    modifier: Modifier = Modifier.size(20.dp),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = 2.1f,
) {
    Canvas(modifier = modifier) {
        val sw = strokeWidthDp.dp.toPx()
        val s = size.minDimension
        val p = Path().apply {
            moveTo(s * 0.54f, s * 0.22f)
            lineTo(s * 0.24f, s * 0.50f)
            lineTo(s * 0.54f, s * 0.78f)
        }
        drawPath(p, color = tint, style = Stroke(width = sw, cap = StrokeCap.Round, join = StrokeJoin.Round))
        drawLine(tint, Offset(s * 0.26f, s * 0.50f), Offset(s * 0.78f, s * 0.50f), strokeWidth = sw, cap = StrokeCap.Round)
    }
}

@Composable
fun IconChevronRight(
    modifier: Modifier = Modifier.size(18.dp),
    tint: Color = LocalKeynearPalette.current.textMuted,
    strokeWidthDp: Float = 2f,
) {
    Canvas(modifier = modifier) {
        val sw = strokeWidthDp.dp.toPx()
        val s = size.minDimension
        val p = Path().apply {
            moveTo(s * 0.36f, s * 0.22f)
            lineTo(s * 0.66f, s * 0.50f)
            lineTo(s * 0.36f, s * 0.78f)
        }
        drawPath(p, color = tint, style = Stroke(width = sw, cap = StrokeCap.Round, join = StrokeJoin.Round))
    }
}

@Composable
fun IconLaptop(
    modifier: Modifier = Modifier.size(24.dp),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = 2f,
) {
    Canvas(modifier = modifier) {
        val sw = strokeWidthDp.dp.toPx()
        val s = size.minDimension
        drawRoundRect(
            color = tint,
            topLeft = Offset(s * 0.18f, s * 0.22f),
            size = Size(s * 0.64f, s * 0.44f),
            cornerRadius = CornerRadius(s * 0.06f, s * 0.06f),
            style = Stroke(width = sw),
        )
        drawLine(tint, Offset(s * 0.08f, s * 0.76f), Offset(s * 0.92f, s * 0.76f), strokeWidth = sw, cap = StrokeCap.Round)
    }
}

@Composable
fun IconSmartphone(
    modifier: Modifier = Modifier.size(24.dp),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = 2f,
) {
    Canvas(modifier = modifier) {
        val sw = strokeWidthDp.dp.toPx()
        val s = size.minDimension
        drawRoundRect(
            color = tint,
            topLeft = Offset(s * 0.26f, s * 0.12f),
            size = Size(s * 0.48f, s * 0.76f),
            cornerRadius = CornerRadius(s * 0.09f, s * 0.09f),
            style = Stroke(width = sw),
        )
        drawCircle(tint, radius = s * 0.04f, center = Offset(s * 0.50f, s * 0.76f))
    }
}

@Composable
fun IconFile(
    modifier: Modifier = Modifier.size(24.dp),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = 2f,
) {
    Canvas(modifier = modifier) {
        val sw = strokeWidthDp.dp.toPx()
        val s = size.minDimension
        val p = Path().apply {
            moveTo(s * 0.24f, s * 0.14f)
            lineTo(s * 0.60f, s * 0.14f)
            lineTo(s * 0.76f, s * 0.30f)
            lineTo(s * 0.76f, s * 0.86f)
            lineTo(s * 0.24f, s * 0.86f)
            close()
        }
        drawPath(p, color = tint, style = Stroke(width = sw, cap = StrokeCap.Round, join = StrokeJoin.Round))
    }
}

@Composable
fun IconFolder(
    modifier: Modifier = Modifier.size(24.dp),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = 2f,
) {
    Canvas(modifier = modifier) {
        val sw = strokeWidthDp.dp.toPx()
        val s = size.minDimension
        val p = Path().apply {
            moveTo(s * 0.14f, s * 0.24f)
            lineTo(s * 0.40f, s * 0.24f)
            lineTo(s * 0.48f, s * 0.34f)
            lineTo(s * 0.86f, s * 0.34f)
            lineTo(s * 0.86f, s * 0.78f)
            lineTo(s * 0.14f, s * 0.78f)
            close()
        }
        drawPath(p, color = tint, style = Stroke(width = sw, cap = StrokeCap.Round, join = StrokeJoin.Round))
    }
}

@Composable
fun IconClipboard(
    modifier: Modifier = Modifier.size(24.dp),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = 2f,
) {
    Canvas(modifier = modifier) {
        val sw = strokeWidthDp.dp.toPx()
        val s = size.minDimension
        drawRoundRect(
            color = tint,
            topLeft = Offset(s * 0.22f, s * 0.22f),
            size = Size(s * 0.56f, s * 0.66f),
            cornerRadius = CornerRadius(s * 0.08f, s * 0.08f),
            style = Stroke(width = sw),
        )
        drawRoundRect(
            color = tint,
            topLeft = Offset(s * 0.36f, s * 0.14f),
            size = Size(s * 0.28f, s * 0.14f),
            cornerRadius = CornerRadius(s * 0.04f, s * 0.04f),
            style = Stroke(width = sw),
        )
    }
}

@Composable
fun IconType(
    modifier: Modifier = Modifier.size(24.dp),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = 2f,
) {
    Canvas(modifier = modifier) {
        val sw = strokeWidthDp.dp.toPx()
        val s = size.minDimension
        drawLine(tint, Offset(s * 0.20f, s * 0.22f), Offset(s * 0.80f, s * 0.22f), strokeWidth = sw, cap = StrokeCap.Round)
        drawLine(tint, Offset(s * 0.50f, s * 0.22f), Offset(s * 0.50f, s * 0.82f), strokeWidth = sw, cap = StrokeCap.Round)
        drawLine(tint, Offset(s * 0.36f, s * 0.82f), Offset(s * 0.64f, s * 0.82f), strokeWidth = sw, cap = StrokeCap.Round)
    }
}

@Composable
fun IconEyeOff(
    modifier: Modifier = Modifier.size(16.dp),
    tint: Color = LocalKeynearPalette.current.textSecondary,
    strokeWidthDp: Float = 1.9f,
) {
    Canvas(modifier = modifier) {
        val sw = strokeWidthDp.dp.toPx()
        val s = size.minDimension
        drawCircle(tint, radius = s * 0.16f, center = Offset(s * 0.5f, s * 0.5f), style = Stroke(width = sw))
        drawLine(tint, Offset(s * 0.16f, s * 0.16f), Offset(s * 0.84f, s * 0.84f), strokeWidth = sw, cap = StrokeCap.Round)
    }
}

@Composable
fun IconUsers(
    modifier: Modifier = Modifier.size(16.dp),
    tint: Color = LocalKeynearPalette.current.textSecondary,
    strokeWidthDp: Float = 1.9f,
) {
    Canvas(modifier = modifier) {
        val sw = strokeWidthDp.dp.toPx()
        val s = size.minDimension
        drawCircle(tint, radius = s * 0.16f, center = Offset(s * 0.38f, s * 0.34f), style = Stroke(width = sw))
        drawArc(
            color = tint,
            startAngle = 180f,
            sweepAngle = 180f,
            useCenter = false,
            topLeft = Offset(s * 0.14f, s * 0.58f),
            size = Size(s * 0.48f, s * 0.36f),
            style = Stroke(width = sw, cap = StrokeCap.Round),
        )
        drawCircle(tint, radius = s * 0.13f, center = Offset(s * 0.72f, s * 0.38f), style = Stroke(width = sw))
    }
}

@Composable
fun IconGlobe(
    modifier: Modifier = Modifier.size(16.dp),
    tint: Color = LocalKeynearPalette.current.textSecondary,
    strokeWidthDp: Float = 1.9f,
) {
    Canvas(modifier = modifier) {
        val sw = strokeWidthDp.dp.toPx()
        val s = size.minDimension
        drawCircle(tint, radius = s * 0.38f, center = Offset(s * 0.5f, s * 0.5f), style = Stroke(width = sw))
        drawLine(tint, Offset(s * 0.12f, s * 0.5f), Offset(s * 0.88f, s * 0.5f), strokeWidth = sw)
        drawLine(tint, Offset(s * 0.5f, s * 0.12f), Offset(s * 0.5f, s * 0.88f), strokeWidth = sw)
    }
}

@Composable
fun IconShield(
    modifier: Modifier = Modifier.size(18.dp),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = 2f,
) {
    Canvas(modifier = modifier) {
        val sw = strokeWidthDp.dp.toPx()
        val s = size.minDimension
        val p = Path().apply {
            moveTo(s * 0.5f, s * 0.12f)
            lineTo(s * 0.84f, s * 0.26f)
            lineTo(s * 0.80f, s * 0.62f)
            lineTo(s * 0.5f, s * 0.88f)
            lineTo(s * 0.20f, s * 0.62f)
            lineTo(s * 0.16f, s * 0.26f)
            close()
        }
        drawPath(p, color = tint, style = Stroke(width = sw, cap = StrokeCap.Round, join = StrokeJoin.Round))
    }
}

@Composable
fun IconCheck(
    modifier: Modifier = Modifier.size(16.dp),
    tint: Color = LocalKeynearPalette.current.onLoadingTonal,
    strokeWidthDp: Float = 2.2f,
) {
    Canvas(modifier = modifier) {
        val sw = strokeWidthDp.dp.toPx()
        val s = size.minDimension
        val p = Path().apply {
            moveTo(s * 0.20f, s * 0.52f)
            lineTo(s * 0.42f, s * 0.74f)
            lineTo(s * 0.82f, s * 0.28f)
        }
        drawPath(p, color = tint, style = Stroke(width = sw, cap = StrokeCap.Round, join = StrokeJoin.Round))
    }
}

@Composable
fun IconClose(
    modifier: Modifier = Modifier.size(16.dp),
    tint: Color = LocalKeynearPalette.current.error,
    strokeWidthDp: Float = 2.2f,
) {
    Canvas(modifier = modifier) {
        val sw = strokeWidthDp.dp.toPx()
        val s = size.minDimension
        drawLine(tint, Offset(s * 0.22f, s * 0.22f), Offset(s * 0.78f, s * 0.78f), strokeWidth = sw, cap = StrokeCap.Round)
        drawLine(tint, Offset(s * 0.78f, s * 0.22f), Offset(s * 0.22f, s * 0.78f), strokeWidth = sw, cap = StrokeCap.Round)
    }
}

