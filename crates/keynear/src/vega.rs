use keynear_core::{DaemonConfig, KeynearError, VegaCapabilities, CAP_AUDIO_HANDOFF};

use crate::audio::{observe_sink_profile, SinkProfileObservation};
use crate::fallback::probe_no_lan;

pub async fn initialize(config: &DaemonConfig) {
    let flags = advertise_capabilities(config);
    if flags & CAP_AUDIO_HANDOFF != 0 {
        log_audio_environment().await;
    } else {
        log_audio_disabled();
    }
    log_fallback().await;
}

fn advertise_capabilities(config: &DaemonConfig) -> u32 {
    let capabilities = VegaCapabilities {
        audio_handoff: config.audio_handoff_enabled,
        ..VegaCapabilities::default()
    };
    let flags = capabilities.flags();
    log_capabilities(flags);
    flags
}

fn log_capabilities(flags: u32) {
    tracing::info!(event = "vega_capabilities_advertised", flags);
}

fn log_audio_disabled() {
    tracing::info!(event = "vega_audio_handoff_disabled");
}

async fn log_audio_environment() {
    log_observation(observe_sink_profile().await);
}

fn log_observation(result: Result<SinkProfileObservation, KeynearError>) {
    match result {
        Ok(observation) => log_sink(observation),
        Err(err) => log_audio_unavailable(err),
    }
}

fn log_sink(observation: SinkProfileObservation) {
    if observation.object_path.is_empty() {
        log_sink_none();
        return;
    }
    log_sink_profile(&observation);
}

fn log_sink_none() {
    tracing::info!(event = "vega_sink_observed", profile = "none");
}

fn log_sink_profile(observation: &SinkProfileObservation) {
    tracing::info!(
        event = "vega_sink_observed",
        profile = observation.profile.as_str(),
        path = %observation.object_path
    );
}

fn log_audio_unavailable(err: KeynearError) {
    tracing::warn!(event = "vega_audio_unavailable", error = %err);
}

async fn log_fallback() {
    match probe_no_lan().await {
        Ok(choice) => log_fallback_choice(choice),
        Err(err) => log_fallback_unavailable(err),
    }
}

fn log_fallback_choice(choice: keynear_core::FallbackChoice) {
    tracing::info!(event = "vega_fallback_selected", media = ?choice.media, status = ?choice.status);
}

fn log_fallback_unavailable(err: KeynearError) {
    tracing::warn!(event = "vega_fallback_unavailable", error = %err);
}
