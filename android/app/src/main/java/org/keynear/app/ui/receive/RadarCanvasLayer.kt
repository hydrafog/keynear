package org.keynear.app.ui.receive

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.keynear.designsystem.LocalKeynearPalette
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin

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

private object WaveMap {
    val InnerRadius = 78.dp
    val MiddleRadius = 136.dp
    val OuterRadius = 196.dp
    val GuideRadii = listOf(InnerRadius, MiddleRadius, OuterRadius)
    const val Spokes = 12
    const val RingPoints = 96
    const val BaseLobes = 6
    const val LobeStep = 2
    const val HiddenAmplitude = 0.5f
    const val BaseAmplitude = 2.2f
    const val AmplitudeStep = 0.9f
    const val SpokeRate = 0.00015f
    const val WaveRate = 0.0028f
    const val SecondaryWaveFactor = 0.7f
    const val EchoAmplitudeFactor = 1.15f
    const val EchoPhaseFactor = 1.1f
    const val SecondaryHarmonic = 3f
    const val SecondaryAmplitudeFactor = 0.5f
    const val MoteRate = 0.0004f
    const val MoteWaveRate = 0.002f
    const val MoteCount = 8
    val MoteDrift = 6.dp
    val MoteRadius = 1.5.dp
}

@Composable
internal fun RadarCanvasLayer(radarActive: Boolean) {
    val palette = LocalKeynearPalette.current
    val cycleMs = WAVE_CYCLE_MS
    val transition = rememberInfiniteTransition(label = "waveMesh")
    val timestamp by transition.animateFloat(
        initialValue = 0f,
        targetValue = cycleMs.toFloat(),
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = cycleMs, easing = LinearEasing),
            repeatMode = RepeatMode.Restart,
        ),
        label = "waveTime",
    )

    Canvas(modifier = Modifier.fillMaxSize()) {
        val cx = size.width / 2f
        val cy = size.height / 2f
        val innerHugRadius = WaveMap.InnerRadius.toPx()
        val maxRadius = WaveMap.OuterRadius.toPx()
        val guideRadii = WaveMap.GuideRadii.map { it.toPx() }
        val speedMult = 1f

        for (s in 0 until WaveMap.Spokes) {
            val a = (s / WaveMap.Spokes.toFloat()) * 2f * PI.toFloat() +
                timestamp * WaveMap.SpokeRate
            drawLine(
                color = palette.waveSpoke,
                start = Offset(cx + cos(a) * innerHugRadius, cy + sin(a) * innerHugRadius),
                end = Offset(cx + cos(a) * maxRadius, cy + sin(a) * maxRadius),
                strokeWidth = SpokeStrokeWidth.toPx(),
            )
        }

        guideRadii.forEachIndexed { idx, baseR ->
            val points = WaveMap.RingPoints
            val lobeCount = WaveMap.BaseLobes + idx * WaveMap.LobeStep
            val amp = if (radarActive) {
                WaveMap.BaseAmplitude + idx * WaveMap.AmplitudeStep
            } else {
                WaveMap.HiddenAmplitude
            }
            val ampPx = amp.dp.toPx()
            val phase = timestamp * WaveMap.WaveRate * speedMult *
                (if (idx % 2 == 0) 1f else -1f) + idx

            val ringPath = Path()
            for (p in 0..points) {
                val theta = (p / points.toFloat()) * 2f * PI.toFloat()
                val wave = sin(theta * lobeCount.toFloat() + phase) * ampPx +
                    cos(theta * WaveMap.SecondaryHarmonic - phase * WaveMap.SecondaryWaveFactor) *
                    (ampPx * WaveMap.SecondaryAmplitudeFactor)
                val r = baseR + wave
                val x = cx + cos(theta) * r
                val y = cy + sin(theta) * r
                if (p == 0) ringPath.moveTo(x, y) else ringPath.lineTo(x, y)
            }
            ringPath.close()
            drawPath(ringPath, palette.waveRing, style = Stroke(width = BadgeHorizontalPadding.toPx()))

            if (radarActive) {
                val echoPath = Path()
                for (p in 0..points) {
                    val theta = (p / points.toFloat()) * 2f * PI.toFloat()
                    val wave = cos(
                        theta * lobeCount.toFloat() - phase * WaveMap.EchoPhaseFactor,
                    ) * (ampPx * WaveMap.EchoAmplitudeFactor)
                    val r = baseR + wave
                    val x = cx + cos(theta) * r
                    val y = cy + sin(theta) * r
                    if (p == 0) echoPath.moveTo(x, y) else echoPath.lineTo(x, y)
                }
                echoPath.close()
                drawPath(echoPath, palette.waveEcho, style = Stroke(width = EchoStrokeWidth.toPx()))
            }
        }

        guideRadii.forEachIndexed { idx, baseR ->
            for (s in 0 until WaveMap.MoteCount) {
                val theta = (s / WaveMap.MoteCount.toFloat()) * 2f * PI.toFloat() +
                    timestamp * WaveMap.MoteRate + idx
                val r = baseR + sin(timestamp * WaveMap.MoteWaveRate + idx + s) * WaveMap.MoteDrift.toPx()
                drawCircle(
                    color = palette.waveEcho.copy(alpha = WAVE_MOTE_ALPHA),
                    radius = WaveMap.MoteRadius.toPx(),
                    center = Offset(cx + cos(theta) * r, cy + sin(theta) * r),
                )
            }
        }
    }
}
