//! 策略和sticky回退保护，不通过重建encoder绕开本连接失败终态。
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
    bgra[3] = 0;
    assert!(video.encode(&bgra, 38, &stop).is_none());
    assert!(!video.available());
    assert_eq!(budget.used(), 0);
    bgra[3] = 255;
    video.reconnect();
    video.policy(policy(true, 3)).unwrap();
    assert!(video.encode(&bgra, 39, &stop).is_none());
    assert_eq!(budget.used(), 0);
}
