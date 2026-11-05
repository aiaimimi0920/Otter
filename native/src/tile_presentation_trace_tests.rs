//! 逻辑时钟与迟到paint回归；诊断不能使prepared获得presented资格。
use super::*;
use crate::tile_shared::Shared;
use std::{sync::Arc, time::Duration};

fn prepared() -> (Arc<Shared>, Arc<crate::tile_shared::Frame>) {
    let (shared, frame) = crate::tile_tests::shared_frame();
    shared.publish(1, frame);
    let frame = shared.lock().frame.clone().unwrap();
    (shared, frame)
}
fn seen(shared: &Shared, stage: Stage) -> bool {
    shared.trace.0.load(Ordering::Relaxed) & (1 << stage as u32) != 0
}
#[test]
fn prepared_clock_estimate_is_not_activation_permission() {
    let (shared, frame) = prepared();
    let now = Instant::now();
    {
        let mut s = shared.lock();
        let (server, error) = s.clock.estimate(now).unwrap();
        s.activate_at = server.saturating_sub(error) + 1;
        assert_eq!(visibility(&s, now), Stage::ClockPending);
        assert!(!s.visible(now));
    }
    shared.confirm(&frame, now);
    assert!(seen(&shared, Stage::ClockPending));
    assert!(shared.lock().presented.is_none());
    assert!(shared.lock().presented_frame.is_none());
    shared.lock().activate_at = 0;
    shared.confirm(&frame, now);
    assert!(seen(&shared, Stage::Confirmed));
    assert!(shared.lock().presented.is_some());
}
#[test]
fn visibility_preserves_strict_authorization_and_inclusive_media_deadlines() {
    let (shared, frame) = prepared();
    let now = Instant::now();
    let mut s = shared.lock();
    s.until = Some(now);
    assert_eq!(visibility(&s, now), Stage::AuthorizationExpired);
    s.until = Some(now + Duration::from_secs(1));
    let mut media = crate::tile_tests::shared_frame().1;
    media.key = frame.key.clone();
    media.live_until = Some(now);
    s.frame = Some(Arc::new(media));
    assert_eq!(visibility(&s, now), Stage::Visible);
    assert_eq!(
        visibility(&s, now + Duration::from_nanos(1)),
        Stage::LiveExpired
    );
    s.frozen = true;
    assert_eq!(
        visibility(&s, now + Duration::from_nanos(1)),
        Stage::Visible
    );
    s.frame = None;
    assert_eq!(visibility(&s, now), Stage::CandidateMissing);
}
#[test]
fn stale_paint_logs_rejection_without_reviving_key_or_media() {
    let (shared, frame) = prepared();
    let now = Instant::now();
    let mut stale = crate::tile_tests::shared_frame().1;
    stale.key = frame.key.clone();
    stale.key.revision += 1;
    shared.confirm(&Arc::new(stale), now);
    assert!(seen(&shared, Stage::KeyChangedDuringPaint));
    let mut stale = crate::tile_tests::shared_frame().1;
    stale.key = frame.key.clone();
    stale.live_until = Some(now - Duration::from_nanos(1));
    shared.confirm(&Arc::new(stale), now);
    assert!(seen(&shared, Stage::MediaExpiredDuringPaint));
    assert!(shared.lock().presented.is_none());
    shared.stop();
    shared.confirm(&frame, now);
    assert!(seen(&shared, Stage::Stopped));
    assert!(shared.lock().presented.is_none());
}
#[test]
fn finite_trace_is_per_owner_and_never_resets_with_business_clear() {
    let shared = Shared::default();
    for _ in 0..1000 {
        shared.trace.observe(Stage::PaintUnconfirmed);
        shared.trace.observe(Stage::HeartbeatPreparedWithoutApplied);
    }
    assert_eq!(shared.trace.0.load(Ordering::Relaxed).count_ones(), 2);
    shared.lock().clear("test");
    assert_eq!(shared.trace.0.load(Ordering::Relaxed).count_ones(), 2);
    assert_eq!(Shared::default().trace.0.load(Ordering::Relaxed), 0);
    assert!((Stage::HeartbeatPreparedWithoutApplied as u32) < 32);
}
