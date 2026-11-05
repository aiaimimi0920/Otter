//! 不sleep的跨组同view在途围栏；新字段仍走原120秒上限和一次1500ms放行。
use super::*;
use crate::tile_surface_actions::{tests::Fixture, Kind};
use std::time::{Duration, Instant};
#[test]
fn rejected_before_http_or_early_return_always_releases_the_weak_inflight_identity() {
    let f = Fixture::new("art");
    let now = Instant::now();
    let edit = crate::tile_surface_edit::Edit::new("field".into(), [0; 32]);
    let mut commands = f.commands();
    commands[0].edit = Some(edit.clone());
    let mut s = f.shared.lock();
    assert!(s.actions.offer(commands, now));
    let c = s.actions.take(&f.state, now).unwrap();
    assert!(edit.in_flight_at(now + Duration::from_secs(2)));
    {
        let _guard = c.edit.as_ref().unwrap().dispatch_guard();
        s.actions.cancel_edit(&edit);
    }
    assert!(!edit.in_flight_at(now + Duration::from_secs(2)));
    assert!(s.actions.inflight.as_ref().unwrap().2.upgrade().is_some()); // 弱身份可存，但不得继续授等待。
}
#[test]
fn a_new_explicit_edit_group_cannot_expire_behind_the_same_views_inflight_prefix() {
    let f = Fixture::new("art");
    let first = crate::tile_surface_edit::Edit::new("field".into(), [0; 32]);
    let next = crate::tile_surface_edit::Edit::new("next".into(), [1; 32]);
    let mut commands = f.commands();
    commands[0].edit = Some(first.clone());
    let now = Instant::now();
    let mut s = f.shared.lock();
    assert!(s.actions.offer(commands, now));
    let c = s.actions.take(&f.state, now).unwrap();
    let mut later = f.commands();
    later[0].edit = Some(next.clone());
    assert!(s.actions.offer(later, now));
    let mut v = crate::tile_surface_actions::tests::value(&f.job.key, "art");
    let Kind::Event(e) = &c.kind else { panic!() };
    v["pending"] = json!([{"ack":crate::tile_surface_actions::tests::ack_value(e["eventId"].as_str().unwrap(),"running"),"actionId":"run","cancelable":true}]);
    let busy = Arc::new(
        crate::tile_surface_wire::State::parse(
            &serde_json::to_vec(&v).unwrap(),
            &f.state.view.binding,
            "art",
            None,
        )
        .unwrap(),
    );
    assert!(s
        .actions
        .take(&busy, now + Duration::from_secs(2))
        .is_none());
    assert!(next.active());
    s.actions
        .attempted(&c, &f.ack(&c, "running"), now + Duration::from_secs(3));
    s.actions
        .observe(busy, "origin", "device", now + Duration::from_secs(4));
    assert!(next.active());
    s.actions.observe(
        f.state.clone(),
        "origin",
        "device",
        now + Duration::from_secs(5),
    );
    assert!(s
        .actions
        .take(&f.state, now + Duration::from_secs(6))
        .is_some());
}
