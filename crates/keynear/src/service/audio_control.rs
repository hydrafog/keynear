use keynear_core::AudioSinkProfile;
use keynear_protocol::{v1, KeynearError};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::warn;

use crate::audio::{DbusHandoffControl, VegaAudioController};

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
}

pub(crate) fn spawn_audio_control(tasks: &mut Vec<JoinHandle<()>>) -> mpsc::Sender<AudioHandoffEvent> {
    let (tx, mut rx) = mpsc::channel(32);
    tasks.push(tokio::spawn(async move {
        let mut controller = VegaAudioController::new(DbusHandoffControl);
        while let Some(event) = rx.recv().await {
            let result = dispatch_event(&mut controller, event).await;
            if let Err(err) = result {
                warn!(event = "vega_handoff_dispatch_failed", error = %err);
            }
        }
    }));
    tx
}

async fn dispatch_event(
    controller: &mut VegaAudioController<DbusHandoffControl>,
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
    }
}

pub(crate) fn enqueue_audio_handoff(
    tx: &mpsc::Sender<AudioHandoffEvent>,
    envelope: &v1::Envelope,
) -> Option<()> {
    let payload = envelope.payload.as_ref()?;
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

fn sink_profile(value: i32) -> AudioSinkProfile {
    match value {
        2 => AudioSinkProfile::A2dpSink,
        3 => AudioSinkProfile::HfpHf,
        _ => AudioSinkProfile::BapSink,
    }
}
