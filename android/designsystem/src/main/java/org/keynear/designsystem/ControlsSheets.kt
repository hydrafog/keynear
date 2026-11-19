package org.keynear.designsystem

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

private val CaptionLetterSpacing = 0.8.sp
private val CodeLetterSpacing = 4.sp

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
            modifier = Modifier.padding(KeynearLayout.CardPadding),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
        ) {
            Text(
                text = heading,
                color = palette.textMuted,
                fontSize = KeynearTypeSizes.Caption,
                fontWeight = FontWeight.Bold,
                letterSpacing = CaptionLetterSpacing,
            )
            Text(
                text = verificationCode,
                color = palette.textPrimary,
                fontSize = KeynearTypeSizes.Title,
                fontWeight = FontWeight.Bold,
                letterSpacing = CodeLetterSpacing,
            )
            Text(
                text = countdownText,
                color = palette.textMuted,
                fontSize = KeynearTypeSizes.Caption,
            )
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
            ) {
                ActionButton(
                    label = declineLabel,
                    onClick = onDecline,
                    errorTonal = true,
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

