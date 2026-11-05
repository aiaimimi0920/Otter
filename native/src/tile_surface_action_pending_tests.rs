//! 弱预留、全量预算、退休及原Edit等待期限的独立状态oracle；不启动网络。
use super::*;
use crate::tile_surface_actions::tests::Fixture;

#[test]
fn held_command_counts_toward_slots_even_after_clear_until_actual_owner_releases() {
    let f = Fixture::new("art");
    let now = Instant::now();
    let mut s = f.shared.lock();
    assert!(s.actions.offer(f.commands(), now));
    let (c, ticket) = s.actions.reserve(&f.state, now, true).unwrap();
    for _ in 0..31 {
        assert!(s.actions.offer(f.commands(), now));
    }
    assert!(!s.actions.offer(f.commands(), now));
    s.actions.clear();
    assert!(!s.actions.holds(&ticket));
    assert_eq!(s.actions.held_cost().0, 1);
    s.actions.observe(f.state.clone(), "o", "d", now);
    for _ in 0..31 {
        assert!(s.actions.offer(f.commands(), now));
    }
    assert!(!s.actions.offer(f.commands(), now));
    assert!(!s.actions.release(&ticket));
    drop(c);
    assert!(s.actions.offer(f.commands(), now));
}
fn large(f: &Fixture, bytes: usize) -> Command {
    let mut c = f.commands().pop().unwrap();
    c.anchor.snapshot = Arc::new(crate::tile_surface_wire::Snapshot {
        revision: 1,
        digest: [0; 32],
        scene: vec![b'x'; bytes],
        nodes: 1,
        images: Default::default(),
        dimensions: Default::default(),
    });
    c
}
#[test]
fn held_command_full_bytes_remain_charged_after_retirement_and_weak_drop_releases() {
    let f = Fixture::new("art");
    let now = Instant::now();
    let mut s = f.shared.lock();
    assert!(s.actions.offer(vec![large(&f, 5 * 1024 * 1024)], now));
    let (c, ticket) = s.actions.reserve(&f.state, now, true).unwrap();
    assert!(!s.actions.offer(vec![large(&f, 4 * 1024 * 1024)], now));
    s.actions.suspend();
    assert!(!s.actions.offer(vec![large(&f, 4 * 1024 * 1024)], now));
    assert!(!s.actions.holds(&ticket));
    drop(c);
    drop(ticket);
    assert_eq!(s.actions.held_cost(), (0, 0));
    assert!(s.actions.offer(vec![large(&f, 4 * 1024 * 1024)], now));
}
#[test]
fn defer_keeps_original_order_id_deadline_and_old_ticket_cannot_release_successor() {
    let f = Fixture::new("art");
    let now = Instant::now();
    let mut s = f.shared.lock();
    let first = f.commands().pop().unwrap();
    let until = first.until;
    let Kind::Event(e) = &first.kind else {
        panic!()
    };
    let id = e["eventId"].clone();
    assert!(s.actions.offer(vec![first], now));
    let (c, old) = s.actions.reserve(&f.state, now, true).unwrap();
    let mut later = f.commands().pop().unwrap();
    later.until = until; // 两个独立意图用相同逻辑期限，不能把后入队的原始期限误判为续期。
    assert!(s.actions.offer(vec![later], now));
    s.actions.defer(c, &old);
    let (c, new) = s.actions.reserve(&f.state, now, true).unwrap();
    assert_eq!(c.until, until);
    let Kind::Event(e) = &c.kind else { panic!() };
    assert_eq!(e["eventId"], id);
    assert!(!s.actions.release(&old));
    assert!(s.actions.holds(&new));
    assert!(s.actions.release(&new));
    assert!(
        s.actions.take(&f.state, until).is_none(),
        "original ordinary deadline not renewed"
    );
}
#[test]
fn retirement_is_scoped_and_cancel_survives_visual_suspend_but_not_view_loss() {
    for cancel in [false, true] {
        let f = Fixture::new("art");
        let now = Instant::now();
        let mut s = f.shared.lock();
        let mut c = f.commands().pop().unwrap();
        if cancel {
            c.kind = Kind::Cancel {
                request: "r".into(),
                event: "e".into(),
            };
        }
        assert!(s.actions.offer(vec![c], now));
        let (_c, ticket) = s.actions.reserve(&f.state, now, true).unwrap();
        s.actions.failed("other");
        assert!(s.actions.holds(&ticket));
        s.actions.suspend();
        assert_eq!(s.actions.holds(&ticket), cancel);
        s.actions.failed("art");
        assert!(!s.actions.holds(&ticket));
    }
}
#[test]
fn reserved_edit_retirement_stops_group_and_preserves_existing_cross_group_wait_only_while_inflight(
) {
    let f = Fixture::new("art");
    let now = Instant::now();
    let mut s = f.shared.lock();
    let first = crate::tile_surface_edit::Edit::new("a".into(), [0; 32]);
    let later = crate::tile_surface_edit::Edit::new("b".into(), [1; 32]);
    let mut c = f.commands().pop().unwrap();
    c.edit = Some(first.clone());
    assert!(s.actions.offer(vec![c], now));
    let (c, ticket) = s.actions.reserve(&f.state, now, true).unwrap();
    let guard = c.edit.as_ref().unwrap().dispatch_guard();
    let mut tail = f.commands().pop().unwrap();
    tail.edit = Some(later.clone());
    assert!(s.actions.offer(vec![tail], now));
    let at = now + Duration::from_secs(2);
    assert!(s
        .actions
        .waiting_edit(s.actions.pending.front().unwrap(), at));
    s.actions.cancel_edit(&later);
    assert!(s.actions.holds(&ticket));
    s.actions.cancel_edit(&first);
    assert!(!s.actions.holds(&ticket));
    assert!(!first.active());
    drop(guard);
    assert!(!first.in_flight_at(at));
    s.actions.release(&ticket);
}
#[test]
fn observe_failure_or_generation_retires_reserved_edit_and_abandoned_owner_does_not_pin_budget() {
    for failure in [false, true] {
        let f = Fixture::new("art");
        let now = Instant::now();
        let mut s = f.shared.lock();
        let edit = crate::tile_surface_edit::Edit::new("a".into(), [0; 32]);
        let mut c = f.commands().pop().unwrap();
        c.edit = Some(edit.clone());
        assert!(s.actions.offer(vec![c], now));
        let (c, ticket) = s.actions.reserve(&f.state, now, true).unwrap();
        let guard = edit.dispatch_guard();
        let mut wire = crate::tile_surface_actions::tests::value(&f.job.key, "art");
        if failure {
            wire["failure"] =
                serde_json::json!({"code":"wall_surface_action_failed","requestId":"fixture"});
        } else {
            wire["generation"] = 2.into();
        }
        let state = Arc::new(
            State::parse(
                &serde_json::to_vec(&wire).unwrap(),
                &f.state.view.binding,
                "art",
                Some(&f.state),
            )
            .unwrap(),
        );
        s.actions.observe(state, "o", "d", now);
        assert!(!s.actions.holds(&ticket));
        assert!(!edit.active());
        drop(guard);
        drop(c);
        drop(ticket);
        assert_eq!(s.actions.held_cost(), (0, 0));
        assert!(!edit.in_flight());
    }
}
