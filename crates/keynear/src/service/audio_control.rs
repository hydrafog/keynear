use keynear_core::{
    percent_to_vcp_volume, AudioContinuityCommand, AudioContinuityObservation, AudioSinkProfile,
    ContinuityCommandKind, ContinuityPlan, KeynearError, PhoenixContinuityEngine,
};
use keynear_protocol::v1;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::audio::{
    call_players, set_vcp_volume, switch_profile, vcp_volume, DbusHandoffControl,
    VegaAudioController,
};

pub(crate) enum AudioHandoffEvent {
    Begin {
        sequence: u64,
        peripheral_id: String,
        profile: AudioSinkProfile,
    },
    Released {
        sequence: u64,
    },
    PeerClaimed {
        sequence: u64,
    },
    Finish {
        sequence: u64,
    },
    Abort {
        sequence: u64,
    },
    ContinuityObservation(AudioContinuityObservation),
}

pub(crate) fn spawn_audio_control(
    tasks: &mut Vec<JoinHandle<()>>,
) -> mpsc::Sender<AudioHandoffEvent> {
    let (tx, mut rx) = mpsc::channel(32);
    tasks.push(tokio::spawn(async move {
        let mut controller = VegaAudioController::new(DbusHandoffControl);
        let mut continuity = PhoenixContinuityEngine::new();
        while let Some(event) = rx.recv().await {
            let result = dispatch_event(&mut controller, &mut continuity, event).await;
            if let Err(err) = result {
                warn!(event = "vega_handoff_dispatch_failed", error = %err);
            }
        }
    }));
    tx
}

async fn dispatch_event(
    controller: &mut VegaAudioController<DbusHandoffControl>,
    continuity: &mut PhoenixContinuityEngine,
    event: AudioHandoffEvent,
) -> Result<(), KeynearError> {
    match event {
        AudioHandoffEvent::Begin {
            sequence,
            peripheral_id,
            profile,
        } => controller.begin(sequence, &peripheral_id, profile).await,
        AudioHandoffEvent::Released { sequence } => controller.mark_released(sequence).await,
        AudioHandoffEvent::PeerClaimed { sequence } => controller.peer_claimed(sequence).await,
        AudioHandoffEvent::Finish { sequence } => controller.finish(sequence).await,
        AudioHandoffEvent::Abort { sequence } => controller.abort(sequence).await,
        AudioHandoffEvent::ContinuityObservation(observation) => {
            let plan = continuity.submit_observation(observation)?;
            apply_continuity_plan(plan).await
        }
    }
}

async fn apply_continuity_plan(plan: ContinuityPlan) -> Result<(), KeynearError> {
    for command in plan.commands {
        apply_continuity_command(command).await?;
    }
    Ok(())
}

async fn apply_continuity_command(command: AudioContinuityCommand) -> Result<(), KeynearError> {
    match command.kind {
        ContinuityCommandKind::Release => apply_release(command.profile).await,
        ContinuityCommandKind::Claim => apply_claim(command).await,
        ContinuityCommandKind::Resume => {
            call_players("Play").await?;
            Ok(())
        }
        ContinuityCommandKind::AbortAndReclaim => apply_reclaim(command.profile).await,
    }
}

async fn apply_release(profile: AudioSinkProfile) -> Result<(), KeynearError> {
    let current = vcp_volume(profile).await.unwrap_or_default();
    info!(event = "phoenix_vcp_release_volume", volume = current);
    switch_profile(profile, "Release").await
}

async fn apply_claim(command: AudioContinuityCommand) -> Result<(), KeynearError> {
    switch_profile(command.profile, "Acquire").await?;
    set_vcp_volume(command.profile, percent_to_vcp_volume(command.volume)).await
}

async fn apply_reclaim(profile: AudioSinkProfile) -> Result<(), KeynearError> {
    if switch_profile(profile, "Release").await.is_ok() {
        switch_profile(profile, "Acquire").await?;
    }
    call_players("Play").await?;
    Ok(())
}

pub(crate) fn enqueue_audio_handoff(
    tx: &mpsc::Sender<AudioHandoffEvent>,
    envelope: &v1::Envelope,
) -> Option<()> {
    let payload = envelope.payload.as_ref()?;
    if let v1::envelope::Payload::AudioContinuityObservation(observation) = payload {
        return tx
            .try_send(AudioHandoffEvent::ContinuityObservation(
                AudioContinuityObservation {
                    device_id: observation.device_id.clone(),
                    peripheral_id: observation.peripheral_id.clone(),
                    kind: continuity_kind(observation.kind),
                    sequence: observation.sequence,
                    volume: observation.volume.clamp(0, 100) as u8,
                    profile: sink_profile(observation.sink_profile),
                },
            ))
            .ok();
    }
    let v1::envelope::Payload::AudioHandoff(handoff) = payload else {
        return None;
    };
    let sequence = handoff.sequence;
    let event = match handoff.phase {
        2 => AudioHandoffEvent::Released { sequence },
        3 => AudioHandoffEvent::PeerClaimed { sequence },
        4 => AudioHandoffEvent::Finish { sequence },
        5 => AudioHandoffEvent::Abort { sequence },
        _ => AudioHandoffEvent::Begin {
            sequence,
            peripheral_id: handoff.peripheral_id.clone(),
            profile: sink_profile(handoff.sink_profile),
        },
    };
    tx.try_send(event).ok()
}

fn continuity_kind(value: i32) -> keynear_core::ContinuityObservationKind {
    match value {
        2 => keynear_core::ContinuityObservationKind::VoiceStarted,
        3 => keynear_core::ContinuityObservationKind::PlaybackStopped,
        4 => keynear_core::ContinuityObservationKind::VoiceStopped,
        _ => keynear_core::ContinuityObservationKind::PlaybackStarted,
    }
}

fn sink_profile(value: i32) -> AudioSinkProfile {
    match value {
        2 => AudioSinkProfile::A2dpSink,
        3 => AudioSinkProfile::HfpHf,
        4 => AudioSinkProfile::BapSource,
        _ => AudioSinkProfile::BapSink,
    }
}
