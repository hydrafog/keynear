use super::*;

fn sample(
    seq: u64,
    technology: RangingTechnology,
    distance: Option<f32>,
    rssi: Option<i32>,
) -> RangingSample {
    RangingSample {
        device_id: "phone-1".to_string(),
        technology,
        distance_meters: distance,
        rssi_dbm: rssi,
        sequence: seq,
    }
}

#[test]
fn uwb_distance_crosses_the_exit_bound_only_after_hysteresis() {
    let mut evaluator = ProximityEvaluator::new(ProximityConfig::default()).expect("evaluator");
    assert_eq!(
        evaluator
            .update(sample(1, RangingTechnology::Uwb, Some(1.0), None))
            .unwrap()
            .status,
        ProximityStatus::InRange
    );
    let near_boundary = evaluator
        .update(sample(2, RangingTechnology::Uwb, Some(3.0), None))
        .unwrap();
    assert_eq!(near_boundary.status, ProximityStatus::InRange);
    let outside = evaluator
        .update(sample(3, RangingTechnology::Uwb, Some(9.0), None))
        .unwrap();
    assert_eq!(outside.status, ProximityStatus::InRange);
    let confirmed = evaluator
        .update(sample(4, RangingTechnology::Uwb, Some(9.0), None))
        .unwrap();
    assert_eq!(confirmed.status, ProximityStatus::OutOfRange);
}

#[test]
fn entering_range_requires_the_stricter_enter_bound() {
    let mut evaluator = ProximityEvaluator::new(ProximityConfig::default()).expect("evaluator");
    for seq in 1..=2 {
        evaluator
            .update(sample(seq, RangingTechnology::Uwb, Some(8.0), None))
            .unwrap();
    }
    assert_eq!(evaluator.snapshot().status, ProximityStatus::OutOfRange);
    evaluator
        .update(sample(3, RangingTechnology::Uwb, Some(3.0), None))
        .unwrap();
    assert_eq!(evaluator.snapshot().status, ProximityStatus::OutOfRange);
    for seq in 4..=5 {
        evaluator
            .update(sample(seq, RangingTechnology::Uwb, Some(1.0), None))
            .unwrap();
    }
    assert_eq!(evaluator.snapshot().status, ProximityStatus::InRange);
}

#[test]
fn ble_rssi_is_smoothed_before_comparison() {
    let mut evaluator = ProximityEvaluator::new(ProximityConfig::default()).expect("evaluator");
    evaluator
        .update(sample(1, RangingTechnology::BleRssi, None, Some(-60)))
        .unwrap();
    let weak = evaluator
        .update(sample(2, RangingTechnology::BleRssi, None, Some(-90)))
        .unwrap();
    assert_eq!(weak.status, ProximityStatus::InRange);
    for seq in 3..=4 {
        evaluator
            .update(sample(seq, RangingTechnology::BleRssi, None, Some(-90)))
            .unwrap();
    }
    assert_eq!(evaluator.snapshot().status, ProximityStatus::OutOfRange);
}

#[test]
fn stale_and_invalid_samples_are_rejected_without_state_change() {
    let mut evaluator = ProximityEvaluator::new(ProximityConfig::default()).expect("evaluator");
    evaluator
        .update(sample(5, RangingTechnology::Uwb, Some(1.0), None))
        .unwrap();
    evaluator
        .update(sample(4, RangingTechnology::Uwb, Some(50.0), None))
        .unwrap();
    assert_eq!(evaluator.snapshot().distance_meters, Some(1.0));
    let err = evaluator
        .update(sample(6, RangingTechnology::Uwb, Some(f32::NAN), None))
        .expect_err("NaN rejected");
    assert!(matches!(err, KeynearError::Validation { .. }));
    let err = evaluator
        .update(sample(7, RangingTechnology::Uwb, None, None))
        .expect_err("missing distance rejected");
    assert!(matches!(err, KeynearError::Validation { .. }));
}

#[test]
fn session_lock_is_recorded_without_changing_range() {
    let mut evaluator = ProximityEvaluator::new(ProximityConfig::default()).expect("evaluator");
    evaluator
        .update(sample(1, RangingTechnology::Uwb, Some(1.0), None))
        .unwrap();
    evaluator.set_session_locked(true);
    assert!(evaluator.snapshot().session_locked);
    assert_eq!(evaluator.snapshot().status, ProximityStatus::InRange);
}

#[test]
fn invalid_configuration_is_rejected() {
    let config = ProximityConfig {
        exit_distance_meters: 1.0,
        enter_distance_meters: 2.0,
        ..ProximityConfig::default()
    };
    assert!(matches!(
        ProximityEvaluator::new(config),
        Err(KeynearError::Validation { .. })
    ));
}

#[test]
fn desk_distance_calibrates_rssi_and_update_config_reloads_live() {
    let near_cfg = ProximityConfig::from_desk_distance(1.0);
    let far_cfg = ProximityConfig::from_desk_distance(6.0);
    assert!(near_cfg.exit_rssi_dbm > far_cfg.exit_rssi_dbm);
    assert!(near_cfg.enter_rssi_dbm >= near_cfg.exit_rssi_dbm);
    assert!(far_cfg.enter_rssi_dbm >= far_cfg.exit_rssi_dbm);
    assert!(
        (ProximityConfig::from_desk_distance(f32::NAN).exit_distance_meters
            - ProximityConfig::default().exit_distance_meters)
            .abs()
            < 0.01
    );

    let mut evaluator = ProximityEvaluator::new(far_cfg).expect("evaluator");
    evaluator.set_session_locked(true);
    evaluator
        .update(sample(1, RangingTechnology::BleRssi, None, Some(-68)))
        .unwrap();
    assert_eq!(evaluator.snapshot().status, ProximityStatus::InRange);

    evaluator.update_config(near_cfg).expect("update config");
    assert!(evaluator.snapshot().session_locked);
    for seq in 2..=3 {
        evaluator
            .update(sample(seq, RangingTechnology::BleRssi, None, Some(-68)))
            .unwrap();
    }
    assert_eq!(evaluator.snapshot().status, ProximityStatus::OutOfRange);

    let bad = ProximityConfig {
        exit_rssi_dbm: -50,
        enter_rssi_dbm: -70,
        ..near_cfg
    };
    assert!(evaluator.update_config(bad).is_err());
}
