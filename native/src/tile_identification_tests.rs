//! 独立单调时间oracle；识别不变布局、不以HTTP接纳冒充绘制，也不恢复已退休请求。
use super::*;
fn fixture() -> (Identification, Endpoint, Command, Instant) {
    (
        Identification::default(),
        Endpoint::image("device-1", &crate::tile_tests::output()),
        Command {
            request_id: "11111111-1111-4111-8111-111111111111".into(),
            remaining_ms: 1000,
            applied: false,
        },
        Instant::now(),
    )
}
#[test]
fn delayed_reads_duplicates_and_expiry_never_extend_or_reopen() {
    let (mut id, e, mut c, t) = fixture();
    id.observe(Some(&c), &e, "lease", t, t + Duration::from_millis(600));
    let m = id.marker(t).unwrap();
    assert_eq!(m.until, t + Duration::from_secs(1));
    id.observe(
        Some(&c),
        &e,
        "lease",
        t + Duration::from_millis(500),
        t + Duration::from_millis(700),
    );
    assert!(Arc::ptr_eq(&m, &id.marker(t).unwrap()));
    id.expire(t + Duration::from_secs(1));
    assert!(id.marker(t).is_none() && id.blocked());
    id.observe(
        Some(&c),
        &e,
        "lease",
        t + Duration::from_secs(1),
        t + Duration::from_secs(1),
    );
    assert!(id.marker(t).is_none() && id.blocked());
    id.finish_cycle();
    assert!(!id.blocked());
    c.request_id = "22222222-2222-4222-8222-222222222222".into();
    id.observe(Some(&c), &e, "lease", t, t + Duration::from_secs(2));
    assert!(id.marker(t).is_none());
}
#[test]
fn only_current_actual_paint_can_report_once_and_dismiss_is_serial() {
    let (mut id, e, c, t) = fixture();
    id.observe(Some(&c), &e, "lease", t, t);
    assert!(id.take_report(&e.endpoint_id, "lease", t).is_none());
    let old = id.marker(t).unwrap();
    id.observe(
        Some(&Command {
            remaining_ms: 500,
            ..c
        }),
        &e,
        "lease",
        t,
        t,
    );
    id.confirm(&old, t);
    assert!(id.take_report(&e.endpoint_id, "lease", t).is_none());
    let current = id.marker(t).unwrap();
    id.confirm(&current, t);
    let report = id.take_report(&e.endpoint_id, "lease", t).unwrap();
    assert_eq!(report.outcome, Outcome::Applied);
    id.confirm(&current, t);
    assert!(id.take_report(&e.endpoint_id, "lease", t).is_none());
    assert!(!id.dismiss(&old, t));
    assert!(id.dismiss(&current, t));
    assert!(!id.dismiss(&current, t));
    assert_eq!(
        id.take_report(&e.endpoint_id, "lease", t).unwrap().outcome,
        Outcome::Dismissed
    );
    assert!(id.marker(t).is_none() && id.blocked());
}
#[test]
fn dismiss_before_control_consumption_overrides_applied_and_requires_fresh_heartbeat() {
    let (mut id, e, c, t) = fixture();
    id.observe(Some(&c), &e, "lease", t, t);
    let m = id.marker(t).unwrap();
    id.confirm(&m, t);
    assert!(id.dismiss(&m, t));
    assert_eq!(
        id.take_report(&e.endpoint_id, "lease", t).unwrap().outcome,
        Outcome::Dismissed
    );
    id.finish_cycle();
    assert!(id.blocked());
    id.observe(Some(&c), &e, "lease", t, t);
    assert!(id.blocked() && id.marker(t).is_none());
    id.finish_cycle();
    assert!(!id.blocked());
    id.observe(Some(&c), &e, "lease", t, t);
    assert!(id.marker(t).is_none());
}
#[test]
fn lease_replacement_and_clear_reject_late_paint_and_reports() {
    let (mut id, e, c, t) = fixture();
    id.observe(Some(&c), &e, "old", t, t);
    let m = id.marker(t).unwrap();
    id.confirm(&m, t);
    assert!(id.take_report(&e.endpoint_id, "new", t).is_none());
    id.clear();
    id.confirm(&m, t);
    assert!(id.take_report(&e.endpoint_id, "old", t).is_none());
    id.observe(Some(&c), &e, "old", t, t);
    assert!(id.marker(t).is_none());
    id.observe(Some(&c), &e, "new", t, t);
    let next = id.marker(t).unwrap();
    id.confirm(&m, t);
    assert!(id.take_report(&e.endpoint_id, "new", t).is_none());
    id.confirm(&next, t);
    assert!(id.take_report(&e.endpoint_id, "new", t).is_some());
}
#[test]
fn authorization_expiry_and_stop_clear_marker_even_without_a_layout() {
    let (mut id, e, c, t) = fixture();
    id.observe(Some(&c), &e, "lease", t, t);
    let mut s = crate::tile_shared::State {
        identification: id,
        until: Some(t + Duration::from_millis(50)),
        ..Default::default()
    };
    s.expire(t + Duration::from_millis(50));
    assert!(s.identification.marker(t).is_none() && s.expired);
    let shared = crate::tile_shared::Shared::default();
    shared
        .lock()
        .identification
        .observe(Some(&c), &e, "lease", t, t);
    shared.stop();
    assert!(shared.lock().identification.marker(t).is_none());
}
