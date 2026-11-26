package org.keynear.app.ui

import androidx.compose.ui.graphics.Color
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.keynear.designsystem.DarkKeynearPalette
import org.keynear.designsystem.LightKeynearPalette
import org.keynear.designsystem.withAccentHex

class AccentDerivationTest {
    @Test
    fun lightAccentDerivesInkTonalAndRimThroughAccentHex() {
        val palette = LightKeynearPalette.withAccentHex("#4ABBD5")
        assertEquals(Color(0xFF4ABBD5), palette.accent)
        assertEquals(palette.loadingInk, palette.focusRing)
        assertTrue(palette.loadingInk != palette.accent)
        assertTrue(palette.onLoadingTonal != palette.accent)
        assertEquals(0.24f, palette.accentRim.alpha, 0.01f)
    }

    @Test
    fun darkAccentKeepsInkAsFillAndUsesDarkRimAlpha() {
        val palette = DarkKeynearPalette.withAccentHex("#E79750")
        assertEquals(palette.accent, palette.loadingInk)
        assertEquals(palette.accent, palette.focusRing)
        assertEquals(0.22f, palette.accentRim.alpha, 0.01f)
        assertTrue(palette.onLoadingTonal != palette.accent)
    }

    @Test
    fun waveColorAlphasFollowLightAndDarkBranches() {
        val light = LightKeynearPalette.withAccentHex("#4ABBD5")
        assertEquals(0.12f, light.waveSpoke.alpha, 0.01f)
        assertEquals(0.22f, light.waveRing.alpha, 0.01f)
        assertEquals(0.28f, light.waveEcho.alpha, 0.01f)
        assertEquals(0.36f, light.waveMote.alpha, 0.01f)
        assertEquals(0.24f, light.brandAuraInner.alpha, 0.01f)
        assertEquals(0.07f, light.brandAuraMid.alpha, 0.01f)

        val dark = DarkKeynearPalette.withAccentHex("#4ABBD5")
        assertEquals(0.12f, dark.waveSpoke.alpha, 0.01f)
        assertEquals(0.38f, dark.waveRing.alpha, 0.01f)
        assertEquals(0.22f, dark.waveEcho.alpha, 0.01f)
        assertEquals(0.44f, dark.waveMote.alpha, 0.01f)
        assertEquals(0.20f, dark.brandAuraInner.alpha, 0.01f)
        assertEquals(0.05f, dark.brandAuraMid.alpha, 0.01f)
    }

    @Test
    fun chromaGradientDependsOnFillInBothBranches() {
        val light = LightKeynearPalette.withAccentHex("#6AC380")
        assertTrue(light.accentSheen != light.accentShift)
        assertTrue(light.accentSheen != light.accent)
        assertTrue(light.accentShift != light.accent)

        val dark = DarkKeynearPalette.withAccentHex("#E79750")
        assertTrue(dark.accentSheen != dark.accentShift)
        assertTrue(dark.accentSheen != light.accentSheen)
    }

    @Test
    fun invalidAccentHexLeavesPaletteUnchanged() {
        assertEquals(LightKeynearPalette, LightKeynearPalette.withAccentHex("not-hex"))
        assertEquals(DarkKeynearPalette, DarkKeynearPalette.withAccentHex("#12345G"))
    }
}
