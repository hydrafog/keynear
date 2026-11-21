use keynear_core::{DaemonConfig, VegaCapabilities, CAP_AUDIO_HANDOFF};

use crate::audio::{observe_sink_profile, DbusHandoffControl, VegaAudioController};
use crate::fallback::probe_no_lan;

pub async fn initialize(config: &DaemonConfig) {
    let capabilities = VegaCapabilities {
        audio_handoff: config.audio_handoff_enabled,
        ..VegaCapabilities::default()
    };
    let flags = capabilities.flags();
    tracing::info!(event = "vega_capabilities_advertised", flags);

    if flags & CAP_AUDIO_HANDOFF == 0 {
        tracing::info!(event = "vega_audio_handoff_disabled");
    } else {
        log_audio_environment().await;
    }

    match probe_no_lan().await {
        Ok(choice) => {
            tracing::info!(event = "vega_fallback_selected", media = ?choice.media, status = ?choice.status);
        }
        Err(err) => {
            tracing::warn!(event = "vega_fallback_unavailable", error = %err);
        }
    }
}

async fn log_audio_environment() {
    let controller = VegaAudioController::new(DbusHandoffControl);
    if let Some(state) = controller.snapshot() {
        tracing::debug!(event = "vega_handoff_state", phase = ?state.phase);
    }
    match observe_sink_profile().await {
        Ok(observation) if !observation.object_path.is_empty() => {
            tracing::info!(
                event = "vega_sink_observed",
                profile = observation.profile.as_str(),
                path = %observation.object_path
            );
        }
        Ok(_) => tracing::info!(event = "vega_sink_observed", profile = "none"),
        Err(err) => tracing::warn!(event = "vega_audio_unavailable", error = %err),
    }
}
