//! Policy/sticky-fallback protection; encoder reconstruction cannot bypass this connection's terminal failure.
use super::*;
fn policy(allowed: bool, sequence: u64) -> VideoPolicy {
    VideoPolicy {
        epoch: 1,
        h264_allowed: allowed,
        keyframe_sequence: sequence,
    }
}
#[test]
fn policy_epoch_sequence_and_legacy_fallback_are_fenced() {
    let budget = Budget::new(0);
    let mut video = Video::new([64, 64], &budget);
    assert!(!video.available());
    video.policy(policy(true, 2)).unwrap();
    assert!(video.refresh());
    assert!(video.policy(policy(true, 1)).is_err());
    assert!(video
        .policy(VideoPolicy {
            epoch: 2,
            ..policy(true, 3)
        })
        .is_err());
    assert!(video.encode(&[], 1, &AtomicBool::new(false)).is_none());
    assert!(!video.refresh());
    video.policy(policy(false, 2)).unwrap();
    assert!(video.refresh());
    video.legacy();
    video.reconnect();
    assert!(video.refresh());
    assert!(!video.policy.h264_allowed);
    assert!(!video.available());
    assert_eq!(budget.used(), 0);
    video.resize([96, 80], &budget);
    assert!(!video.available());
    assert_eq!(budget.used(), 0);
}
#[test]
#[ignore = "requires Windows system encoder; no screen or network"]
fn system_policy_requests_periodic_idr_transition_and_sticky_alpha_fallback() {
    let budget = Budget::new(8 * 1024 * 1024);
    let mut video = Video::new([64, 64], &budget);
    assert!(video.available());
    let mut bgra = [255, 0, 0, 255].repeat(64 * 64);
    let stop = AtomicBool::new(false);
    assert!(video.encode(&bgra, 1, &stop).is_none());
    video.policy(policy(true, 1)).unwrap();
    for id in 2..=33 {
        video.resize([64, 64], &budget);
        let encoded = video.encode(&bgra, id, &stop).unwrap();
        assert_eq!(encoded.keyframe, id == 2 || id == 32);
    }
    video.policy(policy(true, 2)).unwrap();
    assert!(video.refresh());
    assert!(video.encode(&bgra, 34, &stop).unwrap().keyframe);
    video.policy(policy(false, 2)).unwrap();
    assert!(video.encode(&bgra, 35, &stop).is_none());
    video.policy(policy(true, 2)).unwrap();
    assert!(video.encode(&bgra, 36, &stop).unwrap().keyframe);
    video.reconnect();
    assert!(video.refresh());
    assert!(!video.policy.h264_allowed);
    video.policy(policy(true, 2)).unwrap();
    assert!(video.encode(&bgra, 37, &stop).unwrap().keyframe);
    // Same-session dimension changes require immediate new SPS/IDR; dimension round trips must not leave old encoder leases.
    let original_budget = budget.used();
    video.resize([96, 80], &budget);
    let resized = [17, 83, 211, 255].repeat(96 * 80);
    let encoded = video.encode(&resized, 38, &stop).unwrap();
    assert!(encoded.keyframe);
    crate::live_h264::validate(&encoded.bytes, 96, 80, true).unwrap();
    drop(encoded);
    video.resize([64, 64], &budget);
    assert_eq!(budget.used(), original_budget);
    assert!(video.encode(&bgra, 39, &stop).unwrap().keyframe);
    bgra[3] = 0;
    assert!(video.encode(&bgra, 40, &stop).is_none());
    assert!(!video.available());
    assert_eq!(budget.used(), 0);
    bgra[3] = 255;
    video.reconnect();
    video.policy(policy(true, 3)).unwrap();
    video.resize([96, 80], &budget);
    assert!(video.encode(&resized, 41, &stop).is_none());
    assert_eq!(budget.used(), 0);
}
