package org.keynear.app.ui.receive

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.dp
import org.keynear.designsystem.KeynearDimensions
import org.keynear.designsystem.KeynearMotion
import org.keynear.designsystem.KeynearPalette
import org.keynear.designsystem.LocalKeynearPalette
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin

private val EchoStrokeWidth = 4.5.dp
private val MinOuterRadius = 110.dp
private const val ECHO_VISIBILITY_THRESHOLD = 0.01f
private const val MAX_FRAME_DELTA_MS = 64f
private const val NANOS_PER_MS = 1_000_000f
private const val VIEWPORT_RADIUS_FACTOR = 0.46f

private data class WaveGeometry(
    val center: Offset,
    val guideRadii: List<Float>,
    val timestampMs: Float,
    val eased: Float,
)

private data class RingGeometry(
    val center: Offset,
    val baseR: Float,
    val lobes: Int,
    val ampPx: Float,
)

private fun lerp(start: Float, stop: Float, fraction: Float): Float = start + (stop - start) * fraction

private fun smoothstep(t: Float): Float = t * t * (3f - 2f * t)

private fun nextWaveIntensity(current: Float, active: Boolean, deltaMs: Float): Float {
    val target = if (active) 1f else 0f
    val step = deltaMs / WaveMap.FadeMs.toFloat()
    return if (target > current) minOf(target, current + step) else maxOf(target, current - step)
}

@Composable
internal fun RadarCanvasLayer(radarActive: Boolean) {
    val palette = LocalKeynearPalette.current
    var timestampMs by remember { mutableFloatStateOf(0f) }
    var intensity by remember { mutableFloatStateOf(if (radarActive) 1f else 0f) }
    val currentActive by rememberUpdatedState(radarActive)

    LaunchedEffect(radarActive) {
        var lastNanos = 0L
        while (currentActive || intensity > 0f) {
            withFrameNanos { frameNanos ->
                if (lastNanos == 0L) lastNanos = frameNanos
                val deltaMs = ((frameNanos - lastNanos) / NANOS_PER_MS).coerceIn(0f, MAX_FRAME_DELTA_MS)
                lastNanos = frameNanos
                timestampMs = (timestampMs + deltaMs) % KeynearMotion.WAVE_CYCLE_MS
                intensity = nextWaveIntensity(intensity, currentActive, deltaMs)
            }
        }
    }

    Canvas(modifier = Modifier.fillMaxSize()) {
        val eased = smoothstep(intensity.coerceIn(0f, 1f))
        val center = Offset(size.width / 2f, size.height / 2f)
        val maxR = maxOf(MinOuterRadius.toPx(), minOf(size.width, size.height) * VIEWPORT_RADIUS_FACTOR)
        val wave = WaveGeometry(center, WaveMap.Ratios.map { maxR * it }, timestampMs, eased)
        val ambientAlpha = lerp(WaveMap.IdleAlpha, 1f, eased)

        drawWaveSpokes(wave, palette.waveSpoke.copy(alpha = palette.waveSpoke.alpha * ambientAlpha))
        drawWaveRings(wave, palette)
        drawWaveMotes(wave, palette.waveMote.copy(alpha = palette.waveMote.alpha * ambientAlpha))
    }
}

private fun DrawScope.drawWaveSpokes(
    wave: WaveGeometry,
    color: androidx.compose.ui.graphics.Color,
) {
    val innerR = wave.guideRadii.first()
    val outerR = wave.guideRadii.last()
    val twoPi = 2f * PI.toFloat()
    val strokePx = KeynearDimensions.Dp1.toPx()
    for (s in 0 until WaveMap.Spokes) {
        val angle = (s / WaveMap.Spokes.toFloat()) * twoPi + wave.timestampMs * WaveMap.SpokeRate
        drawLine(
            color = color,
            start = Offset(wave.center.x + cos(angle) * innerR, wave.center.y + sin(angle) * innerR),
            end = Offset(wave.center.x + cos(angle) * outerR, wave.center.y + sin(angle) * outerR),
            strokeWidth = strokePx,
        )
    }
}

private fun DrawScope.drawWaveRings(
    wave: WaveGeometry,
    palette: KeynearPalette,
) {
    val ringStroke = Stroke(width = KeynearDimensions.Dp2.toPx())
    val echoStroke = Stroke(width = EchoStrokeWidth.toPx())
    wave.guideRadii.forEachIndexed { idx, baseR ->
        val lobes = WaveMap.BaseLobes + idx * WaveMap.LobeStep
        val ampPx = lerp(WaveMap.HiddenAmplitude, WaveMap.BaseAmplitude + idx * WaveMap.AmplitudeStep, wave.eased).dp.toPx()
        val phase = wave.timestampMs * WaveMap.WaveRate * (if (idx % 2 == 0) 1f else -1f) + idx
        val ring = RingGeometry(wave.center, baseR, lobes, ampPx)
        drawPath(
            path = buildWaveRingPath(ring, phase, isEcho = false),
            color = palette.waveRing,
            style = ringStroke,
        )
        if (wave.eased > ECHO_VISIBILITY_THRESHOLD) {
            drawPath(
                path = buildWaveRingPath(ring, phase, isEcho = true),
                color = palette.waveEcho.copy(alpha = palette.waveEcho.alpha * wave.eased),
                style = echoStroke,
            )
        }
    }
}

private fun buildWaveRingPath(
    ring: RingGeometry,
    phase: Float,
    isEcho: Boolean,
): Path {
    val path = Path()
    val twoPi = 2f * PI.toFloat()
    for (p in 0..WaveMap.RingPoints) {
        val theta = (p / WaveMap.RingPoints.toFloat()) * twoPi
        val wave = if (isEcho) {
            cos(theta * ring.lobes - phase * WaveMap.EchoPhaseFactor) * (ring.ampPx * WaveMap.EchoAmplitudeFactor)
        } else {
            sin(theta * ring.lobes + phase) * ring.ampPx +
                cos(theta * WaveMap.SecondaryHarmonic - phase * WaveMap.SecondaryWaveFactor) *
                (ring.ampPx * WaveMap.SecondaryAmplitudeFactor)
        }
        val r = ring.baseR + wave
        val x = ring.center.x + cos(theta) * r
        val y = ring.center.y + sin(theta) * r
        if (p == 0) path.moveTo(x, y) else path.lineTo(x, y)
    }
    path.close()
    return path
}

private fun DrawScope.drawWaveMotes(
    wave: WaveGeometry,
    color: androidx.compose.ui.graphics.Color,
) {
    val twoPi = 2f * PI.toFloat()
    val driftPx = WaveMap.MoteDrift.toPx()
    val radiusPx = WaveMap.MoteRadius.toPx()
    wave.guideRadii.forEachIndexed { idx, baseR ->
        for (s in 0 until WaveMap.MoteCount) {
            val theta = (s / WaveMap.MoteCount.toFloat()) * twoPi + wave.timestampMs * WaveMap.MoteRate + idx
            val r = baseR + sin(wave.timestampMs * WaveMap.MoteWaveRate + idx + s) * driftPx
            drawCircle(
                color = color,
                radius = radiusPx,
                center = Offset(wave.center.x + cos(theta) * r, wave.center.y + sin(theta) * r),
            )
        }
    }
}
