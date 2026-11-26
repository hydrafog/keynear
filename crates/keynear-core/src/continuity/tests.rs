use super::*;

fn observation(
    device: &str,
    seq: u64,
    kind: ContinuityObservationKind,
) -> AudioContinuityObservation {
    AudioContinuityObservation {
        device_id: device.to_string(),
        peripheral_id: "AA:BB:CC:DD:EE:FF".to_string(),
        kind,
        sequence: seq,
        volume: 72,
        profile: AudioSinkProfile::BapSink,
    }
}

#[test]
fn first_start_claims_and_resumes_without_release() {
    let mut engine = PhoenixContinuityEngine::new();
    let plan = engine
        .submit_observation(observation(
            "phone-1",
            1,
            ContinuityObservationKind::PlaybackStarted,
        ))
        .expect("first start");
    assert_eq!(plan.commands.len(), 2);
    assert_eq!(plan.commands[0].kind, ContinuityCommandKind::Claim);
    assert_eq!(plan.commands[1].kind, ContinuityCommandKind::Resume);
    assert_eq!(engine.owner(), Some("phone-1"));
}

#[test]
fn competing_start_releases_previous_owner_then_claims() {
    let mut engine = PhoenixContinuityEngine::new();
    engine
        .submit_observation(observation(
            "phone-1",
            1,
            ContinuityObservationKind::PlaybackStarted,
        ))
        .expect("first start");
    let plan = engine
        .submit_observation(observation(
            "desktop-1",
            2,
            ContinuityObservationKind::VoiceStarted,
        ))
        .expect("second start");
    assert_eq!(plan.commands.len(), 3);
    assert_eq!(plan.commands[0].kind, ContinuityCommandKind::Release);
    assert_eq!(plan.commands[0].owner_device_id, "phone-1");
    assert_eq!(plan.commands[1].kind, ContinuityCommandKind::Claim);
    assert_eq!(plan.commands[1].owner_device_id, "desktop-1");
    assert_eq!(plan.commands[2].kind, ContinuityCommandKind::Resume);
}

#[test]
fn simultaneous_equal_sequence_uses_stable_device_priority() {
    let mut engine = PhoenixContinuityEngine::new();
    engine
        .submit_observation(observation(
            "z-phone",
            7,
            ContinuityObservationKind::PlaybackStarted,
        ))
        .expect("higher priority device first");
    let plan = engine
        .submit_observation(observation(
            "a-phone",
            7,
            ContinuityObservationKind::PlaybackStarted,
        ))
        .expect("lower device id wins tie");
    assert_eq!(engine.owner(), Some("a-phone"));
    assert_eq!(plan.commands[0].owner_device_id, "z-phone");
    let ignored = engine
        .submit_observation(observation(
            "m-phone",
            7,
            ContinuityObservationKind::PlaybackStarted,
        ))
        .expect("later id loses tie");
    assert!(ignored.commands.is_empty());
}

#[test]
fn stale_observation_cannot_reverse_a_settled_handoff() {
    let mut engine = PhoenixContinuityEngine::new();
    engine
        .submit_observation(observation(
            "phone-1",
            5,
            ContinuityObservationKind::PlaybackStarted,
        ))
        .expect("start");
    let stale = engine
        .submit_observation(observation(
            "desktop-1",
            4,
            ContinuityObservationKind::PlaybackStarted,
        ))
        .expect("stale ignored");
    assert!(stale.commands.is_empty());
    assert_eq!(engine.owner(), Some("phone-1"));
}

#[test]
fn failed_claim_recovers_previous_owner() {
    let mut engine = PhoenixContinuityEngine::new();
    engine
        .submit_observation(observation(
            "phone-1",
            1,
            ContinuityObservationKind::PlaybackStarted,
        ))
        .expect("first start");
    engine
        .submit_observation(observation(
            "desktop-1",
            2,
            ContinuityObservationKind::PlaybackStarted,
        ))
        .expect("handoff");
    let recovery = engine.plan_recovery(3, "claim_failed").expect("recovery");
    assert_eq!(recovery.commands.len(), 1);
    assert_eq!(
        recovery.commands[0].kind,
        ContinuityCommandKind::AbortAndReclaim
    );
    assert_eq!(recovery.commands[0].owner_device_id, "phone-1");
    assert_eq!(engine.owner(), Some("phone-1"));
}

#[test]
fn owner_stop_reclaims_previous_owner() {
    let mut engine = PhoenixContinuityEngine::new();
    engine
        .submit_observation(observation(
            "phone-1",
            1,
            ContinuityObservationKind::PlaybackStarted,
        ))
        .expect("first start");
    engine
        .submit_observation(observation(
            "desktop-1",
            2,
            ContinuityObservationKind::VoiceStarted,
        ))
        .expect("handoff");
    let plan = engine
        .submit_observation(observation(
            "desktop-1",
            3,
            ContinuityObservationKind::VoiceStopped,
        ))
        .expect("stop");
    assert_eq!(plan.commands.len(), 1);
    assert_eq!(
        plan.commands[0].kind,
        ContinuityCommandKind::AbortAndReclaim
    );
    assert_eq!(engine.owner(), None);
}

#[test]
fn invalid_observation_is_rejected_before_state_change() {
    let mut engine = PhoenixContinuityEngine::new();
    let mut bad = observation("", 1, ContinuityObservationKind::PlaybackStarted);
    assert!(matches!(
        engine.submit_observation(bad.clone()),
        Err(KeynearError::Validation { .. })
    ));
    bad.device_id = "phone-1".to_string();
    bad.volume = 101;
    assert!(matches!(
        engine.submit_observation(bad),
        Err(KeynearError::Validation { .. })
    ));
    assert_eq!(engine.owner(), None);
}

#[test]
fn observation_kind_marks_only_start_events() {
    assert!(ContinuityObservationKind::PlaybackStarted.is_start());
    assert!(ContinuityObservationKind::VoiceStarted.is_start());
    assert!(!ContinuityObservationKind::PlaybackStopped.is_start());
    assert!(!ContinuityObservationKind::VoiceStopped.is_start());
}
