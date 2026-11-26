use keynear_protocol::KeynearError;

use super::*;
use crate::audio::AudioSinkProfile;

fn begin(engine: &mut CallAlertEngine, seq: u64) -> CallPlan {
    engine
        .begin(
            seq,
            CallAlertDescriptor {
                call_id: "call-1",
                caller: "Pixel 9",
                sink_profile: AudioSinkProfile::HfpHf,
            },
        )
        .expect("begin")
}

#[test]
fn incoming_call_releases_headset_and_decline_reclaims_it() {
    let mut engine = CallAlertEngine::new();
    let start = begin(&mut engine, 1);
    assert!(start.release_sink && !start.reclaim_sink);
    assert_eq!(engine.snapshot().expect("state").phase, CallPhase::Incoming);
    let declined = engine
        .apply(2, "call-1", CallActionKind::Decline)
        .expect("decline");
    assert!(declined.reclaim_sink);
    assert_eq!(engine.snapshot().expect("state").phase, CallPhase::Ended);
}

#[test]
fn answer_then_mute_then_end() {
    let mut engine = CallAlertEngine::new();
    begin(&mut engine, 1);
    engine
        .apply(2, "call-1", CallActionKind::Answer)
        .expect("answer");
    assert_eq!(engine.snapshot().expect("state").phase, CallPhase::Active);
    let muted = engine
        .apply(3, "call-1", CallActionKind::Mute)
        .expect("mute");
    assert!(muted.mute);
    assert!(engine.snapshot().expect("state").muted);
    let ended = engine.end(4, "call-1").expect("end");
    assert!(ended.reclaim_sink);
}

#[test]
fn stale_call_action_is_rejected() {
    let mut engine = CallAlertEngine::new();
    begin(&mut engine, 5);
    let stale = engine
        .apply(4, "call-1", CallActionKind::Decline)
        .expect("stale");
    assert_eq!(stale, CallPlan::default());
    let wrong_call = engine.apply(6, "call-2", CallActionKind::Decline);
    assert!(matches!(wrong_call, Err(KeynearError::Protocol(_))));
}

#[test]
fn mute_before_answer_is_rejected() {
    let mut engine = CallAlertEngine::new();
    begin(&mut engine, 1);
    let err = engine
        .apply(2, "call-1", CallActionKind::Mute)
        .expect_err("mute requires active call");
    assert!(matches!(err, KeynearError::Protocol(_)));
}

#[test]
fn invalid_caller_is_rejected_before_state_change() {
    let mut engine = CallAlertEngine::new();
    let err = engine
        .begin(
            1,
            CallAlertDescriptor {
                call_id: "call-1",
                caller: "",
                sink_profile: AudioSinkProfile::HfpHf,
            },
        )
        .expect_err("empty caller rejected");
    assert!(matches!(err, KeynearError::Validation { .. }));
    assert!(engine.snapshot().is_none());
    let long_caller = "x".repeat(MAX_CALLER_LEN + 1);
    let err = engine
        .begin(
            2,
            CallAlertDescriptor {
                call_id: "call-1",
                caller: &long_caller,
                sink_profile: AudioSinkProfile::BapSink,
            },
        )
        .expect_err("long caller rejected");
    assert!(matches!(err, KeynearError::Validation { .. }));
}

#[test]
fn clear_drops_the_active_alert() {
    let mut engine = CallAlertEngine::new();
    begin(&mut engine, 1);
    assert!(engine.snapshot().is_some());
    engine.clear();
    assert!(engine.snapshot().is_none());
    let plan = begin(&mut engine, 2);
    assert!(plan.release_sink);
}

#[test]
fn invalid_call_id_is_rejected_before_state_change() {
    let mut engine = CallAlertEngine::new();
    let err = engine
        .begin(
            1,
            CallAlertDescriptor {
                call_id: "",
                caller: "Pixel 9",
                sink_profile: AudioSinkProfile::HfpHf,
            },
        )
        .expect_err("empty call id rejected");
    assert!(matches!(err, KeynearError::Validation { .. }));
    assert!(engine.snapshot().is_none());
    let long_id = "x".repeat(MAX_CALL_ID_LEN + 1);
    let err = engine
        .begin(
            2,
            CallAlertDescriptor {
                call_id: &long_id,
                caller: "Pixel 9",
                sink_profile: AudioSinkProfile::HfpHf,
            },
        )
        .expect_err("long call id rejected");
    assert!(matches!(err, KeynearError::Validation { .. }));
    let err = engine
        .begin(
            3,
            CallAlertDescriptor {
                call_id: "call\nid",
                caller: "Pixel 9",
                sink_profile: AudioSinkProfile::HfpHf,
            },
        )
        .expect_err("control character rejected");
    assert!(matches!(err, KeynearError::Validation { .. }));
}

#[test]
fn apply_and_end_without_an_alert_are_rejected() {
    let mut engine = CallAlertEngine::new();
    assert!(matches!(
        engine.apply(1, "call-1", CallActionKind::Answer),
        Err(KeynearError::Protocol(_))
    ));
    assert!(matches!(
        engine.end(2, "call-1"),
        Err(KeynearError::Protocol(_))
    ));
}

#[test]
fn end_rejects_a_stale_sequence_and_a_foreign_call() {
    let mut engine = CallAlertEngine::new();
    begin(&mut engine, 5);
    let stale = engine.end(5, "call-1").expect("stale sequence");
    assert_eq!(stale, CallPlan::default());
    assert!(matches!(
        engine.end(6, "call-2"),
        Err(KeynearError::Protocol(_))
    ));
    let ended = engine.end(7, "call-1").expect("end");
    assert!(ended.reclaim_sink);
    let already_ended = engine.end(8, "call-1").expect("end again");
    assert!(!already_ended.reclaim_sink);
}
