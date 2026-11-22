package org.keynear.designsystem

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.graphics.vector.PathParser

private const val DEFAULT_STROKE_WIDTH = 2f


private data class LucideRect(
    val x: Float,
    val y: Float,
    val w: Float,
    val h: Float,
    val r: Float,
)

private data class LucideCircle(
    val cx: Float,
    val cy: Float,
    val r: Float,
)

@Composable
private fun LucideSvgIcon(
    modifier: Modifier,
    tint: Color,
    strokeWidth: Float = DEFAULT_STROKE_WIDTH,
    pathStrings: List<String> = emptyList(),
    rects: List<LucideRect> = emptyList(),
    circles: List<LucideCircle> = emptyList(),
) {
    val parsedPaths = remember(pathStrings) {
        pathStrings.map { PathParser().parsePathString(it).toPath() }
    }
    Canvas(modifier = modifier) {
        val sx = size.width / 24f
        val sy = size.height / 24f
        val stroke = Stroke(width = strokeWidth, cap = StrokeCap.Round, join = StrokeJoin.Round)
        scale(scaleX = sx, scaleY = sy, pivot = Offset.Zero) {
            parsedPaths.forEach { path ->
                drawPath(path = path, color = tint, style = stroke)
            }
            rects.forEach { rect ->
                drawRoundRect(
                    color = tint,
                    topLeft = Offset(rect.x, rect.y),
                    size = Size(rect.w, rect.h),
                    cornerRadius = CornerRadius(rect.r, rect.r),
                    style = stroke,
                )
            }
            circles.forEach { circle ->
                drawCircle(
                    color = tint,
                    radius = circle.r,
                    center = Offset(circle.cx, circle.cy),
                    style = stroke,
                )
            }
        }
    }
}

@Composable
fun IconInbox(
    modifier: Modifier = Modifier.size(KeynearSpacing.Xl),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf(
            "M 5.45 5.11 L 2 12 L 2 18 A 2 2 0 0 0 4 20 L 20 20 " +
                "A 2 2 0 0 0 22 18 L 22 12 L 18.55 5.11 " +
                "A 2 2 0 0 0 16.76 4 L 7.24 4 A 2 2 0 0 0 5.45 5.11 Z",
            "M 22 12 L 16 12 L 14 15 L 10 15 L 8 12 L 2 12",
        ),
    )
}

@Composable
fun IconSend(
    modifier: Modifier = Modifier.size(KeynearSpacing.Xl),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf(
            "M 14.536 21.686 A 0.5 0.5 0 0 0 15.473 21.662 L 21.973 2.662 " +
                "A 0.496 0.496 0 0 0 21.338 2.027 L 2.338 8.527 " +
                "A 0.5 0.5 0 0 0 2.314 9.464 L 10.244 12.644 " +
                "A 2 2 0 0 1 11.356 13.754 Z",
            "M 21.854 2.147 L 10.914 13.086",
        ),
    )
}

@Composable
fun IconSliders(
    modifier: Modifier = Modifier.size(KeynearSpacing.Xl),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf(
            "M 10 5 H 3",
            "M 21 5 H 14",
            "M 14 3 V 7",
            "M 8 12 H 3",
            "M 21 12 H 12",
            "M 8 10 V 14",
            "M 12 19 H 3",
            "M 21 19 H 16",
            "M 16 17 V 21",
        ),
    )
}

@Composable
fun IconArrowLeft(
    modifier: Modifier = Modifier.size(KeynearSpacing.Xl),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf(
            "M 12 19 L 5 12 L 12 5",
            "M 19 12 L 5 12",
        ),
    )
}

@Composable
fun IconChevronRight(
    modifier: Modifier = Modifier.size(KeynearLayout.ChevronIconSize),
    tint: Color = LocalKeynearPalette.current.textMuted,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf("M 9 18 L 15 12 L 9 6"),
    )
}

@Composable
fun IconChevronDown(
    modifier: Modifier = Modifier.size(KeynearLayout.ChevronIconSize),
    tint: Color = LocalKeynearPalette.current.textMuted,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf("M 6 9 L 12 15 L 18 9"),
    )
}

@Composable
fun IconChevronUp(
    modifier: Modifier = Modifier.size(KeynearLayout.ChevronIconSize),
    tint: Color = LocalKeynearPalette.current.textMuted,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf("M 18 15 L 12 9 L 6 15"),
    )
}

@Composable
fun IconLaptop(
    modifier: Modifier = Modifier.size(KeynearSpacing.Xl),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf(
            "M 18 5 A 2 2 0 0 1 20 7 L 20 15.526 A 2 2 0 0 0 20.212 16.423 " +
                "L 21.28 18.55 A 1 1 0 0 1 20.38 20 L 3.62 20 " +
                "A 1 1 0 0 1 2.72 18.55 L 3.788 16.423 " +
                "A 2 2 0 0 0 4 15.526 L 4 7 A 2 2 0 0 1 6 5 Z",
            "M 3.946 15.987 L 20.054 15.987",
        ),
    )
}

@Composable
fun IconSmartphone(
    modifier: Modifier = Modifier.size(KeynearSpacing.Xl),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf("M 12 18 L 12.01 18"),
        rects = listOf(LucideRect(x = 5f, y = 2f, w = 14f, h = 20f, r = 2f)),
    )
}

@Composable
fun IconFile(
    modifier: Modifier = Modifier.size(KeynearSpacing.Xl),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf(
            "M 15 2 L 6 2 A 2 2 0 0 0 4 4 L 4 20 A 2 2 0 0 0 6 22 L 18 22 A 2 2 0 0 0 20 20 L 20 7 Z",
            "M 14 2 L 14 6 A 2 2 0 0 0 16 8 L 20 8",
            "M 10 9 L 8 9",
            "M 16 13 L 8 13",
            "M 16 17 L 8 17",
        ),
    )
}

@Composable
fun IconFolder(
    modifier: Modifier = Modifier.size(KeynearSpacing.Xl),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf(
            "M 20 20 A 2 2 0 0 0 22 18 L 22 8 A 2 2 0 0 0 20 6 L 12.1 6 " +
                "A 2 2 0 0 1 10.41 5.1 L 9.6 3.9 A 2 2 0 0 0 7.93 3 L 4 3 " +
                "A 2 2 0 0 0 2 5 L 2 18 A 2 2 0 0 0 4 20 Z",
        ),
    )
}

@Composable
fun IconClipboard(
    modifier: Modifier = Modifier.size(KeynearSpacing.Xl),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf(
            "M 16 4 L 18 4 A 2 2 0 0 1 20 6 L 20 20 A 2 2 0 0 1 18 22 L 6 22 A 2 2 0 0 1 4 20 L 4 6 A 2 2 0 0 1 6 4 L 8 4",
        ),
        rects = listOf(LucideRect(x = 8f, y = 2f, w = 8f, h = 4f, r = 1f)),
    )
}

@Composable
fun IconPlus(
    modifier: Modifier = Modifier.size(KeynearSpacing.Xl),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf("M 5 12 L 19 12", "M 12 5 L 12 19"),
    )
}

@Composable
fun IconEyeOff(
    modifier: Modifier = Modifier.size(KeynearSpacing.Lg),
    tint: Color = LocalKeynearPalette.current.textSecondary,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf(
            "M 10.733 5.076 A 10.744 10.744 0 0 1 21.938 11.651 A 1 1 0 0 1 21.938 12.347 A 10.747 10.747 0 0 1 20.494 14.837",
            "M 14.084 14.158 A 3 3 0 0 1 9.842 9.916",
            "M 17.479 17.499 A 10.75 10.75 0 0 1 2.062 12.348 A 1 1 0 0 1 2.062 11.652 A 10.75 10.75 0 0 1 6.508 6.509",
            "M 2 2 L 22 22",
        ),
    )
}

@Composable
fun IconShieldCheck(
    modifier: Modifier = Modifier.size(KeynearSpacing.Lg),
    tint: Color = LocalKeynearPalette.current.textSecondary,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf(
            "M 20 13 C 20 18 16.5 20.5 12.34 21.95 A 1 1 0 0 1 11.67 21.94 " +
                "C 7.5 20.5 4 18 4 13 V 6 A 1 1 0 0 1 5 5 C 7 5 9.5 3.8 11.24 2.28 " +
                "A 1.17 1.17 0 0 1 12.76 2.28 C 14.51 3.81 17 5 19 5 " +
                "A 1 1 0 0 1 20 6 Z",
            "M 9 12 L 11 14 L 15 10",
        ),
    )
}

@Composable
fun IconRadio(
    modifier: Modifier = Modifier.size(KeynearSpacing.Lg),
    tint: Color = LocalKeynearPalette.current.textSecondary,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf(
            "M 16.247 7.761 A 6 6 0 0 1 16.247 16.239",
            "M 19.075 4.933 A 10 10 0 0 1 19.075 19.067",
            "M 4.925 19.067 A 10 10 0 0 1 4.925 4.933",
            "M 7.753 16.239 A 6 6 0 0 1 7.753 7.761",
        ),
        circles = listOf(LucideCircle(cx = 12f, cy = 12f, r = 2f)),
    )
}

@Composable
fun IconAlertTriangle(
    modifier: Modifier = Modifier.size(KeynearLayout.ChevronIconSize),
    tint: Color = LocalKeynearPalette.current.onDangerTonal,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf(
            "M 21.73 18 L 13.73 4 A 2 2 0 0 0 10.25 4 L 2.25 18 A 2 2 0 0 0 4 21 L 20 21 A 2 2 0 0 0 21.73 18 Z",
            "M 12 9 L 12 13",
            "M 12 17 L 12.01 17",
        ),
    )
}

@Composable
fun IconClose(
    modifier: Modifier = Modifier.size(KeynearLayout.ChevronIconSize),
    tint: Color = LocalKeynearPalette.current.error,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf("M 18 6 L 6 18", "M 6 6 L 18 18"),
    )
}

@Composable
fun IconCheck(
    modifier: Modifier = Modifier.size(KeynearSpacing.Lg),
    tint: Color = LocalKeynearPalette.current.onLoadingTonal,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf("M 20 6 L 9 17 L 4 12"),
    )
}

@Composable
fun IconRefreshCw(
    modifier: Modifier = Modifier.size(KeynearLayout.ChevronIconSize),
    tint: Color = LocalKeynearPalette.current.textPrimary,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf(
            "M 21 12 A 9 9 0 0 0 12 3 A 9.75 9.75 0 0 0 5.26 5.74 L 3 8",
            "M 3 3 L 3 8 L 8 8",
            "M 3 12 A 9 9 0 0 0 12 21 A 9.75 9.75 0 0 0 18.74 18.26 L 21 16",
            "M 16 16 L 21 16 L 21 21",
        ),
    )
}

@Composable
fun IconTrash2(
    modifier: Modifier = Modifier.size(KeynearLayout.ChevronIconSize),
    tint: Color = LocalKeynearPalette.current.error,
    strokeWidthDp: Float = DEFAULT_STROKE_WIDTH,
) {
    LucideSvgIcon(
        modifier = modifier,
        tint = tint,
        strokeWidth = strokeWidthDp,
        pathStrings = listOf(
            "M 10 11 L 10 17",
            "M 14 11 L 14 17",
            "M 19 6 L 19 20 A 2 2 0 0 1 17 22 L 7 22 A 2 2 0 0 1 5 20 L 5 6",
            "M 3 6 L 21 6",
            "M 8 6 L 8 4 A 2 2 0 0 1 10 2 L 14 2 A 2 2 0 0 1 16 4 L 16 6",
        ),
    )
}

@Composable
fun IconKeynearSymbolic(
    modifier: Modifier = Modifier.size(KeynearSpacing.Xl),
    tint: Color = LocalKeynearPalette.current.textPrimary,
) {
    val layers = remember {
        listOf(
            SYMBOLIC_STEM_ALPHA to "M 160,276 C 177.728,276 192,290.272 192,308 L 192,368 " +
                "C 192,385.728 177.728,400 160,400 C 142.272,400 128,385.728 128,368 " +
                "L 128,308 C 128,290.272 142.272,276 160,276 Z",
            SYMBOLIC_ARM_ALPHA to "M 324.303,104.082 A 32,32 0 0 0 301.061,111.803 L 197.061,201.803 " +
                "A 32,32 0 0 0 193.803,246.939 A 32,32 0 0 0 238.939,250.197 " +
                "L 342.939,160.197 A 32,32 0 0 0 346.197,115.061 A 32,32 0 0 0 324.303,104.082 Z",
            1.0f to "M 160,112 A 32,32 0 0 0 128,144 L 128,248 " +
                "C 128,271.394 139.942,294.939 158.354,309.260 A 32.003,32.003 0 0 0 161.039,311.137 " +
                "L 305.039,401.137 A 32,32 0 0 0 349.137,390.961 A 32,32 0 0 0 338.961,346.863 " +
                "L 196.658,257.924 C 192.246,254.407 192,253.486 192,248 L 192,144 A 32,32 0 0 0 160,112 Z",
            1.0f to "M 335.311,191.021 A 11,11 0 0 0 327.75,194.725 A 11,11 0 0 0 328.725,210.25 " +
                "C 356.403,234.66 356.403,277.34 328.725,301.75 A 11,11 0 0 0 327.75,317.275 " +
                "A 11,11 0 0 0 343.275,318.25 C 380.601,285.332 380.601,226.668 343.275,193.75 " +
                "A 11,11 0 0 0 335.311,191.021 Z",
            SYMBOLIC_OUTER_ARC_ALPHA to "M 375.498,164.016 A 8,8 0 0 0 370,166.709 A 8,8 0 0 0 370.709,178 " +
                "C 417.76,219.495 417.76,292.505 370.709,334 A 8,8 0 0 0 370,345.291 " +
                "A 8,8 0 0 0 381.291,346 C 435.357,298.318 435.357,213.682 381.291,166 " +
                "A 8,8 0 0 0 375.498,164.016 Z",
        ).map { (alpha, pathData) -> alpha to PathParser().parsePathString(pathData).toPath() }
    }
    Canvas(modifier = modifier) {
        val sx = size.width / 316f
        val sy = size.height / 328f
        scale(scaleX = sx, scaleY = sy, pivot = Offset.Zero) {
            translate(left = -112f, top = -92f) {
                layers.forEach { (alpha, path) ->
                    drawPath(path = path, color = tint.copy(alpha = tint.alpha * alpha))
                }
            }
        }
    }
}
