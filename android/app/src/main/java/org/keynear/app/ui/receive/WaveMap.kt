package org.keynear.app.ui.receive

import androidx.compose.ui.unit.dp

internal object WaveMap {
    val InnerRadius = 78.dp
    val MiddleRadius = 136.dp
    val OuterRadius = 196.dp
    val Ratios = listOf(78f / 196f, 136f / 196f, 1f)
    const val FadeMs = 800
    const val IdleAlpha = 0.4f
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
