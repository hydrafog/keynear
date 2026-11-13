package org.keynear.app

import android.os.Bundle
import android.provider.Settings
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.keynear.app.core.CoreBridge
import org.keynear.app.core.CoreClientProvider
import org.keynear.app.core.CoreSnapshot
import org.keynear.app.core.CoreState
import org.keynear.app.ui.theme.KeynearMotion
import org.keynear.app.ui.theme.KeynearRadii
import org.keynear.app.ui.theme.KeynearSpacing
import org.keynear.app.ui.theme.KeynearTheme
import org.keynear.app.ui.theme.LocalKeynearPalette

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val bridge = CoreClientProvider.get()
        setContent {
            KeynearTheme {
                KeynearFoundationScreen(bridge = bridge)
            }
        }
    }
}

@Composable
private fun KeynearFoundationScreen(bridge: CoreBridge) {
    val palette = LocalKeynearPalette.current
    val context = LocalContext.current
    val reducedMotion = remember(context) {
        runCatching {
            Settings.Global.getFloat(
                context.contentResolver,
                Settings.Global.ANIMATOR_DURATION_SCALE,
                1f,
            ) == 0f
        }.getOrDefault(false)
    }

    var uiState by remember { mutableStateOf<CoreState>(CoreState.Loading) }

    LaunchedEffect(bridge) {
        uiState = withContext(Dispatchers.IO) { bridge.loadState() }
    }

    Surface(
        modifier = Modifier.fillMaxSize(),
        color = palette.canvas,
    ) {
        Crossfade(
            targetState = uiState,
            animationSpec = tween(durationMillis = if (reducedMotion) 0 else KeynearMotion.SHORT_DURATION_MS),
            label = "core_state_crossfade",
        ) { state ->
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(KeynearSpacing.Lg),
                verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
            ) {
                Text(
                    text = stringResource(R.string.app_name),
                    style = MaterialTheme.typography.headlineSmall,
                    color = palette.textPrimary,
                )
                Text(
                    text = "Local continuity for nearby devices",
                    style = MaterialTheme.typography.bodyMedium,
                    color = palette.textSecondary,
                )

                when (state) {
                    is CoreState.Loading -> LoadingCard()
                    is CoreState.Empty -> SnapshotContent(
                        snapshot = state.snapshot,
                        onCycleVisibility = { uiState = bridge.cycleVisibility() },
                    )
                    is CoreState.Ready -> SnapshotContent(
                        snapshot = state.snapshot,
                        onCycleVisibility = { uiState = bridge.cycleVisibility() },
                    )
                    is CoreState.Error -> {
                        ErrorBanner(
                            message = state.message,
                            onRetry = { uiState = bridge.loadState() },
                        )
                        state.previous?.let { previous ->
                            SnapshotContent(
                                snapshot = previous,
                                onCycleVisibility = { uiState = bridge.cycleVisibility() },
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun LoadingCard() {
    val palette = LocalKeynearPalette.current
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.TwoXl),
        colors = CardDefaults.cardColors(containerColor = palette.surface),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Lg),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            CircularProgressIndicator(color = palette.accent)
            Spacer(modifier = Modifier.width(KeynearSpacing.Md))
            Text(
                text = stringResource(R.string.status_loading),
                style = MaterialTheme.typography.bodyLarge,
                color = palette.textPrimary,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f),
            )
        }
    }
}

@Composable
private fun ErrorBanner(
    message: String,
    onRetry: () -> Unit,
) {
    val palette = LocalKeynearPalette.current
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.Xl),
        colors = CardDefaults.cardColors(containerColor = palette.errorTonal),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Lg),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Md),
        ) {
            Text(
                text = message,
                style = MaterialTheme.typography.bodyMedium,
                color = palette.onDangerTonal,
                maxLines = 3,
                overflow = TextOverflow.Ellipsis,
            )
            Button(
                onClick = onRetry,
                modifier = Modifier.defaultMinSize(
                    minWidth = KeynearSpacing.MinTouchTarget,
                    minHeight = KeynearSpacing.MinTouchTarget,
                ),
                shape = RoundedCornerShape(KeynearRadii.Xl),
                colors = ButtonDefaults.buttonColors(
                    containerColor = palette.accent,
                    contentColor = palette.onAccent,
                ),
            ) {
                Text(text = stringResource(R.string.action_retry))
            }
        }
    }
}

@Composable
private fun SnapshotContent(
    snapshot: CoreSnapshot,
    onCycleVisibility: () -> Unit,
) {
    val palette = LocalKeynearPalette.current

    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.TwoXl),
        colors = CardDefaults.cardColors(containerColor = palette.surface),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Lg),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Sm),
        ) {
            Text(
                text = "Discovery visibility",
                style = MaterialTheme.typography.titleMedium,
                color = palette.textPrimary,
            )
            Text(
                text = snapshot.visibility.label(),
                style = MaterialTheme.typography.bodyLarge,
                color = palette.textSecondary,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            Spacer(modifier = Modifier.height(KeynearSpacing.Xs))
            Button(
                onClick = onCycleVisibility,
                modifier = Modifier.defaultMinSize(
                    minWidth = KeynearSpacing.MinTouchTarget,
                    minHeight = KeynearSpacing.MinTouchTarget,
                ),
                shape = RoundedCornerShape(KeynearRadii.Xl),
                colors = ButtonDefaults.buttonColors(
                    containerColor = palette.accent,
                    contentColor = palette.onAccent,
                ),
            ) {
                Text(text = stringResource(R.string.action_cycle_visibility))
            }
        }
    }

    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.TwoXl),
        colors = CardDefaults.cardColors(containerColor = palette.surface),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Lg),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Xs),
        ) {
            Text(
                text = "Core contract",
                style = MaterialTheme.typography.titleMedium,
                color = palette.textPrimary,
            )
            Text(
                text = "Version ${snapshot.version} (${snapshot.maxControlFrameBytes} bytes max frame)",
                style = MaterialTheme.typography.bodyMedium,
                color = palette.textSecondary,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                text = snapshot.configSummary,
                style = MaterialTheme.typography.labelMedium,
                color = palette.textMuted,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }

    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(KeynearRadii.TwoXl),
        colors = CardDefaults.cardColors(containerColor = palette.surfaceTonal),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(KeynearSpacing.Lg),
            verticalArrangement = Arrangement.spacedBy(KeynearSpacing.Xs),
        ) {
            Text(
                text = stringResource(R.string.empty_devices_title),
                style = MaterialTheme.typography.titleMedium,
                color = palette.textPrimary,
            )
            Text(
                text = stringResource(R.string.empty_devices_body),
                style = MaterialTheme.typography.bodyMedium,
                color = palette.textSecondary,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}
