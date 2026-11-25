use super::*;

fn begin(engine: &mut AudioHandoffEngine, seq: u64) -> AudioHandoffPlan {
    engine
        .begin(seq, "AA:BB:CC:DD:EE:FF", AudioSinkProfile::A2dpSink)
        .expect("begin")
}

#[test]
fn handoff_pauses_before_release_then_resumes() {
    let mut engine = AudioHandoffEngine::new();
    let start = begin(&mut engine, 1);
    assert!(start.pause_playback && start.release_sink);
    engine.mark_released(2).expect("released");
    assert_eq!(
        engine.snapshot().expect("state").phase,
        HandoffPhase::Released
    );
    engine.peer_claimed(3).expect("claimed");
    let restore = engine.release(4).expect("release");
    assert!(restore.reclaim_sink && restore.resume_playback);
    let state = engine.snapshot().expect("state");
    assert!(!state.playback_paused && !state.sink_released);
}

#[test]
fn abort_restores_paused_playback_and_released_sink() {
    let mut engine = AudioHandoffEngine::new();
    begin(&mut engine, 1);
    engine.mark_released(2).expect("released");
    let plan = engine.abort(3).expect("abort");
    assert!(plan.reclaim_sink && plan.resume_playback);
    assert_eq!(
        engine.snapshot().expect("state").phase,
        HandoffPhase::Aborted
    );
}

#[test]
fn stale_sequence_is_ignored_without_changing_state() {
    let mut engine = AudioHandoffEngine::new();
    begin(&mut engine, 5);
    let stale = begin(&mut engine, 4);
    assert_eq!(stale, AudioHandoffPlan::default());
    assert_eq!(engine.snapshot().expect("state").sequence, 5);
}

#[test]
fn invalid_peripheral_id_is_rejected_before_state_change() {
    let mut engine = AudioHandoffEngine::new();
    let err = engine
        .begin(1, "", AudioSinkProfile::BapSink)
        .expect_err("empty id rejected");
    assert!(matches!(err, KeynearError::Validation { .. }));
    assert!(engine.snapshot().is_none());
    let err = engine
        .begin(2, &"x".repeat(65), AudioSinkProfile::BapSink)
        .expect_err("long id rejected");
    assert!(matches!(err, KeynearError::Validation { .. }));
    let err = engine
        .begin(3, "bad\nname", AudioSinkProfile::BapSink)
        .expect_err("control char rejected");
    assert!(matches!(err, KeynearError::Validation { .. }));
}

#[test]
fn sink_profiles_parse_from_bluez_names() {
    assert_eq!(
        AudioSinkProfile::parse("bap_sink"),
        Some(AudioSinkProfile::BapSink)
    );
    assert_eq!(
        AudioSinkProfile::parse("a2dp_sink"),
        Some(AudioSinkProfile::A2dpSink)
    );
    assert_eq!(
        AudioSinkProfile::parse("hfp_hf"),
        Some(AudioSinkProfile::HfpHf)
    );
    assert_eq!(AudioSinkProfile::parse("unknown"), None);
    assert_eq!(AudioSinkProfile::HfpHf.as_str(), "hfp_hf");
    assert_eq!(
        AudioSinkProfile::parse("bap_source"),
        Some(AudioSinkProfile::BapSource)
    );
    assert_eq!(AudioSinkProfile::BapSource.as_str(), "bap_source");
}

#[test]
fn vcp_volume_maps_to_percent_and_back() {
    assert_eq!(vcp_volume_to_percent(0), 0);
    assert_eq!(vcp_volume_to_percent(255), 100);
    assert_eq!(vcp_volume_to_percent(128), 50);
    assert_eq!(percent_to_vcp_volume(0), 0);
    assert_eq!(percent_to_vcp_volume(100), 255);
    assert_eq!(percent_to_vcp_volume(50), 128);
    assert_eq!(vcp_volume_to_percent(1024), 100);
    assert_eq!(percent_to_vcp_volume(255), 255);
}

#[test]
fn out_of_order_transition_returns_typed_error() {
    let mut engine = AudioHandoffEngine::new();
    let err = engine.mark_released(1).expect_err("no active handoff");
    assert!(matches!(err, KeynearError::Protocol(_)));
}
