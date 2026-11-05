//! 独立整数时间/像素oracle；预期不调用被测选帧算法。
use super::*;
use crate::image_decode::Image;
fn frame(epoch: u64, id: u64, received: u64) -> Frame {
    Frame {
        identity: (epoch, id),
        received,
        image: Image {
            width: 1,
            height: 1,
            pixels: vec![id as u8, 0, 0, 255].into(),
        },
    }
}
#[test]
fn bounded_queue_preserves_earliest_deadline_and_two_newest_then_selects_latest_due() {
    let mut q = Queue::default();
    for i in 1..=6 {
        assert!(q.push(frame(1, i, 1000 + i * 10)));
    }
    assert_eq!(
        q.pending.iter().map(|f| f.identity.1).collect::<Vec<_>>(),
        [1, 5, 6]
    );
    assert!(q.advance(Some((1090, 1))).is_none()); // target1009,第一帧还未到期。
    assert_eq!(q.advance(Some((1091, 1))).unwrap().identity, (1, 1));
    assert_eq!(q.advance(Some((1135, 1))).unwrap().identity, (1, 5));
    assert_eq!(q.advance(Some((1141, 1))).unwrap().identity, (1, 6));
    assert_eq!(q.pending.len(), 0);
}
#[test]
fn late_hold_epoch_watermark_and_invalid_clock_clear_without_revival() {
    let mut q = Queue::default();
    q.push(frame(1, 1, 1000));
    assert!(q.advance(Some((1331, 0))).is_none()); // target1251，超过250ms丢弃。
    q.push(frame(1, 2, 1300));
    assert_eq!(q.advance(Some((1380, 0))).unwrap().identity, (1, 2));
    assert!(q.advance(Some((2380, 0))).is_some());
    assert!(q.advance(Some((2381, 0))).is_none());
    assert!(!q.push(frame(1, 1, 1400)));
    assert!(!q.push(frame(2, 1, 1299)));
    q.push(frame(2, 1, 1400));
    assert!(q.advance(Some((1400, 0))).is_none());
    assert!(q.advance(None).is_none());
    assert!(!q.push(frame(2, 1, 1400)));
    q.push(frame(2, 2, 1500));
    assert_eq!(q.advance(Some((1580, 0))).unwrap().identity, (2, 2));
}
#[test]
fn queue_and_clones_hold_pixels_until_last_owner_drops_and_ui_deadline_is_bounded() {
    let mut q = Queue::default();
    let f = frame(1, 1, 1000);
    let weak = f.image.pixels.downgrade();
    q.push(f);
    let now = Instant::now();
    let f = q.advance(Some((1100, 20))).unwrap().clone(); // target1000，剩余1000ms。
    assert_eq!(
        Queue::deadline(&f, (1100, 20), now),
        now + Duration::from_millis(1000)
    );
    q.clear();
    assert!(weak.upgrade().is_some());
    drop(f);
    assert!(weak.upgrade().is_none());
}
#[test]
fn clock_uncertainty_includes_age_drift_and_rounding_guard() {
    let now = Instant::now();
    let mut clock = crate::tile_clock::Clock::default();
    clock.observe(
        &crate::tile_wire::Timing {
            clock_id: "c".into(),
            server_time_ms: 1000,
            scenes: vec![],
        },
        now,
        now + Duration::from_millis(20),
    );
    assert_eq!(
        clock.estimate(now + Duration::from_millis(20)),
        Some((1010, 11))
    );
    assert_eq!(
        clock.estimate(now + Duration::from_millis(10020)),
        Some((11010, 12))
    );
    assert!(clock.estimate(now + Duration::from_millis(15021)).is_none());
}
#[test]
fn publication_deduplicates_pixels_but_not_identity_and_freeze_keeps_actual_presented_frame() {
    let (shared, mut f) = crate::tile_tests::shared_frame();
    let now = Instant::now();
    f.images.insert("live".into(), frame(1, 1, 1000).image);
    shared.publish_until(1, f, Some(now + Duration::from_millis(10)));
    let first = shared.lock().frame.clone().unwrap();
    shared.confirm(&first, now);
    shared.publish_until(
        1,
        crate::tile_shared::Frame {
            key: first.key.clone(),
            layout: first.layout.clone(),
            size: first.size,
            images: first.images.clone(),
            arts: first.arts.clone(),
            live_until: None,
        },
        Some(now + Duration::from_millis(20)),
    );
    assert!(std::sync::Arc::ptr_eq(
        shared.lock().frame.as_ref().unwrap(),
        &first
    ));
    assert!(!shared.lock().visible(now + Duration::from_millis(21)));
    assert!(!shared.lock().visible(now + Duration::from_millis(11)));
    {
        let mut s = shared.lock();
        s.frozen = true;
        s.frame = s.presented_frame.clone();
    }
    assert!(shared.lock().visible(now + Duration::from_millis(21)));
    shared.unavailable(1, &first.key, "late");
    assert!(shared.lock().frame.is_some());
    shared.stop();
    shared.confirm(&first, now);
    assert!(shared.lock().presented_frame.is_none());
}
#[test]
fn new_candidate_does_not_extend_old_paint_deadline_or_old_scene_receipt() {
    let (shared, mut f) = crate::tile_tests::shared_frame();
    let now = Instant::now();
    f.images.insert("live".into(), frame(1, 1, 1000).image);
    shared.publish_until(1, f, Some(now + Duration::from_millis(10)));
    let old = shared.lock().frame.clone().unwrap();
    shared.confirm(&old, now);
    let mut images = std::collections::BTreeMap::new();
    images.insert("live".into(), frame(1, 2, 1010).image);
    shared.publish_until(
        1,
        crate::tile_shared::Frame {
            key: old.key.clone(),
            layout: old.layout.clone(),
            size: old.size,
            images,
            arts: old.arts.clone(),
            live_until: None,
        },
        Some(now + Duration::from_millis(100)),
    );
    {
        let mut s = shared.lock();
        s.expire(now + Duration::from_millis(11));
        assert!(s.frame.is_some());
        assert!(s.presented.is_none());
        assert_eq!(s.generation, 1);
        assert_eq!(s.desired, Some(old.key.clone()));
    }
    shared.confirm(&old, now + Duration::from_millis(11));
    assert!(shared.lock().presented_frame.is_none());
    let fresh = shared.lock().frame.clone().unwrap();
    shared.confirm(&fresh, now + Duration::from_millis(11));
    assert!(std::sync::Arc::ptr_eq(
        shared.lock().presented_frame.as_ref().unwrap(),
        &fresh
    ));
}
