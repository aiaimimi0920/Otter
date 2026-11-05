//! 独立固定值oracle；失败候选不commit、两样本/心跳/丢失/恢复与容量。
use super::*;
fn sample(value: &str) -> Sample {
    let mut sample = root_error("unused");
    sample.value = Ok(serde_json::json!({"value":{"text":value,"readOnly":false}}));
    sample
}
#[test]
fn source_observation_stability_commit_heartbeat_missing_and_recovery() {
    let mut tracks = Tracks::default();
    let first = tracks
        .candidates(vec![sample("seven")], 100)
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(
        (first.sequence, first.state, first.stable_since_ms),
        (1, State::Observing, None)
    );
    let retry = tracks
        .candidates(vec![sample("seven")], 101)
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(retry.sequence, 1);
    assert_eq!(retry.state, State::Observing);
    tracks.commit(first);
    let stable = tracks
        .candidates(vec![sample("seven")], 600)
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(
        (stable.sequence, stable.state, stable.stable_since_ms),
        (2, State::Stable, Some(600))
    );
    tracks.commit(stable);
    assert!(tracks
        .candidates(vec![sample("seven")], 1599)
        .unwrap()
        .is_empty());
    let heartbeat = tracks
        .candidates(vec![sample("seven")], 1600)
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(
        (heartbeat.sequence, heartbeat.stable_since_ms),
        (3, Some(600))
    );
    tracks.commit(heartbeat);
    let changed = tracks
        .candidates(vec![sample("eight")], 1700)
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(changed.state, State::Observing);
    assert_eq!(changed.stable_since_ms, None);
    tracks.commit(changed);
    let stale = tracks.candidates(vec![], 1800).unwrap().pop().unwrap();
    assert_eq!(stale.state, State::Stale);
    assert_eq!(stale.value, None);
    tracks.commit(stale);
    assert!(tracks.candidates(vec![], 1900).unwrap().is_empty());
    let recovery = tracks
        .candidates(vec![sample("eight")], 2000)
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(recovery.state, State::Observing);
    assert_eq!(recovery.sequence, 6);
}
#[test]
fn source_observation_failed_scan_never_stales_partial_values_and_bounds_store() {
    let mut tracks = Tracks::default();
    let mut a = sample("a");
    a.id = "uia:a".into();
    let value = tracks
        .candidates(vec![a.clone()], 100)
        .unwrap()
        .pop()
        .unwrap();
    tracks.commit(value);
    let errors = tracks.scan_error(200).unwrap();
    assert_eq!(errors.len(), 2);
    assert!(errors
        .iter()
        .all(|o| o.state == State::Error && o.value.is_none()));
    for error in errors {
        tracks.commit(error);
    }
    assert!(tracks.scan_error(300).unwrap().is_empty());
    assert!(tracks.candidates(vec![a.clone(), a], 400).is_err());
    let many: Vec<_> = (0..256)
        .map(|i| {
            let mut s = sample("x");
            s.id = format!("uia:{i}");
            s
        })
        .collect();
    assert!(tracks.candidates(many, 500).is_err());
    assert!(tracks
        .candidates(vec![sample(&"x".repeat(65537))], 500)
        .is_err());
    assert!(tracks.candidates(vec![], 0).is_err());
    let mut bad = tracks.0.values().next().unwrap().clone();
    bad.stable_since_ms = Some(bad.observed_at_ms + 1);
    assert!(bad.validate().is_err());
}
#[test]
fn source_observation_locator_only_change_resets_stability_without_replacing_identity() {
    let mut tracks = Tracks::default();
    for now in [100, 600] {
        let value = tracks
            .candidates(vec![sample("same")], now)
            .unwrap()
            .pop()
            .unwrap();
        tracks.commit(value);
    }
    let mut renamed = sample("same");
    renamed.locator.name = Some("renamed label".into());
    let changed = tracks
        .candidates(vec![renamed], 700)
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(changed.observation_id, "uia:source");
    assert_eq!(changed.state, State::Observing);
    assert_eq!(changed.sequence, 3);
    assert_eq!(changed.stable_since_ms, None);
}
