package org.keynear.designsystem

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign

@Composable
fun PairingPromptCard(
    heading: String,
    verificationCode: String,
    countdownText: String,
    declineLabel: String,
    confirmLabel: String,
    onDecline: () -> Unit,
    onConfirm: () -> Unit,
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
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearLayout.CardPadding),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            SectionHeading(
                title = heading,
                textAlign = TextAlign.Center,
            )
            TactileSurface(
                modifier = Modifier.fillMaxWidth(),
                shape = KeynearShapes.Xl,
                containerColor = palette.trackWell,
                tone = TactileTone.INSET,
                cornerRadius = KeynearRadii.Xl,
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = verificationCode,
                    color = palette.textPrimary,
                    fontFamily = FontFamily.Monospace,
                    fontSize = KeynearTypeSizes.Title,
                    fontWeight = FontWeight.Bold,
                    letterSpacing = KeynearTypeSizes.Sp4,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.padding(vertical = KeynearLayout.ContentGap),
                )
            }
            Text(
                text = countdownText,
                color = palette.textMuted,
                fontSize = KeynearTypeSizes.Caption,
                textAlign = TextAlign.Center,
            )
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(KeynearLayout.ContentGap),
            ) {
                ActionButton(
                    label = declineLabel,
                    onClick = onDecline,
                    tonalSurface = true,
                    modifier = Modifier.weight(1f),
                )
                ActionButton(
                    label = confirmLabel,
                    onClick = onConfirm,
                    primary = true,
                    modifier = Modifier.weight(1f),
                )
            }
        }
    }
}
