//! Hover 与 Pointer 共用 FIFO 的容量、合并、取消和在途证据，不增加第二队列。
use super::*;
fn hover(target: Option<&str>, x: f64) -> HoverIntent {
    HoverIntent {
        input: HoverInput {
            target: target.map(str::to_owned),
            client: [x, 1.],
            modifiers: 0,
        },
        viewport_epoch: 4,
    }
}
fn pointer(phase: crate::surface_javascript_wire::PointerPhase) -> PointerIntent {
    PointerIntent {
        input: crate::surface_javascript_wire::PointerInput {
            target: "dom:2".into(),
            phase,
            client: [0., 0.],
            modifiers: 0,
            click: false,
        },
        gesture: 1,
        viewport_epoch: 4,
    }
}
#[test]
fn javascript_hover_queue_both_terminal_reservations_share_32_slots_and_sequence() {
    use crate::surface_javascript_wire::PointerPhase::*;
    for max_sequence in [false, true] {
        let mut q = Queue::default();
        if max_sequence {
            q.sequence = u64::MAX - 32;
        }
        for _ in 0..28 {
            assert!(q.push("text", Phase::Change, "x"));
        }
        assert!(q.push_hover(hover(Some("dom:1"), 0.)));
        assert!(q.push_pointer(pointer(Pointerdown)));
        assert!(!q.push("text", Phase::Change, "x"));
        assert!(!q.push_keyboard("text", "x", 0, 0, 0));
        assert!(!q.can_push_step("text") && !q.can_push_change());
        assert!(!q.push_hover(hover(Some("dom:3"), 1.)));
        assert!(!q.push_pointer(pointer(Pointermove)));
        assert!(q.push_hover(hover(None, 2.)));
        assert!(q.push_pointer(pointer(Pointerup)));
        assert_eq!(q.waiting.len(), 32);
        assert!(!q.hover_reserved && q.pointer_reservation.is_none());
        if max_sequence {
            assert_eq!(q.sequence, u64::MAX);
        }
    }
}
#[test]
fn javascript_hover_queue_coalesces_only_same_unsealed_unprepared_tail() {
    let mut q = Queue::default();
    assert!(q.push_hover(hover(Some("dom:1"), 1.)));
    assert!(q.push_hover(hover(Some("dom:1"), 2.)));
    assert_eq!(q.waiting.len(), 1);
    q.seal();
    assert!(q.push_hover(hover(Some("dom:1"), 3.)));
    q.prepared(Some("first"));
    assert!(q.push_hover(hover(Some("dom:1"), 4.)));
    let Edge::Hover(h) = &q.prepared_intent("first").unwrap().edge else {
        panic!()
    };
    assert_eq!(h.input.client[0], 2.);
    assert!(q.push_hover(hover(Some("dom:2"), 5.)));
    let mut newer = hover(Some("dom:2"), 6.);
    newer.viewport_epoch = 5;
    assert!(q.push_hover(newer));
    assert!(q.push_keyboard("text", "x", 0, 0, 0));
    assert!(q.push_hover(hover(Some("dom:2"), 7.)));
    assert_eq!(q.waiting.len(), 5);
}
#[test]
fn javascript_hover_queue_cancel_never_invents_enter_leave_and_keeps_prepared() {
    for started in [false, true] {
        let mut q = Queue::default();
        assert!(q.push_hover(hover(Some("dom:1"), 1.)));
        if started {
            q.prepared(Some("move"));
        }
        assert!(q.push_hover(hover(None, 2.)));
        assert!(q.push("a", Phase::Change, "x"));
        assert!(q.push_hover(hover(Some("dom:2"), 3.)));
        assert!(q.push_keyboard("b", "y", 0, 0, 0));
        let fence = q.seal();
        assert!(q.cancel_hovers(6));
        assert!(q.cancel_hovers(7));
        assert_eq!(q.state(&fence), FenceState::Invalid);
        assert_eq!(q.prepared.len(), usize::from(started));
        assert_eq!(q.waiting.len(), 2 + usize::from(started));
        assert_eq!(
            q.waiting
                .iter()
                .filter(|i| !matches!(i.edge, Edge::Hover(_)))
                .map(|i| i.node.as_str())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
        if started {
            let Edge::Hover(h) = &q.front().unwrap().edge else {
                panic!()
            };
            assert!(h.input.target.is_none());
            assert_eq!(h.viewport_epoch, 7);
        }
    }
}
#[test]
fn javascript_hover_queue_cancel_reuses_waiting_sequence_at_max_and_keeps_single_leave() {
    let mut q = Queue {
        sequence: u64::MAX - 2,
        ..Default::default()
    };
    assert!(q.push_hover(hover(Some("dom:1"), 1.)));
    q.prepared(Some("move"));
    q.complete_local("move", None);
    assert!(q.push_hover(hover(None, 2.)));
    assert_eq!(q.sequence, u64::MAX);
    assert!(q.cancel_hovers(5));
    assert!(q.cancel_hovers(6));
    assert_eq!(q.waiting.len(), 1);
    assert_eq!(q.front().unwrap().sequence, u64::MAX);
    q.prepared(Some("leave"));
    q.complete_local("leave", None);
    assert!(!q.has_hovers());
}
#[test]
fn javascript_hover_queue_retention_failure_clear_and_unstarted_return_preserve_owner_contract() {
    let mut q = Queue::default();
    assert!(!q.has_hovers());
    assert!(q.push_hover(hover(Some("dom:1"), 1.)));
    q.prepared(Some("move"));
    q.retain(|_| false);
    assert!(q.has_hovers());
    assert_eq!(q.cancel("move").as_deref(), Some("dom:1"));
    q.prepared(Some("retry"));
    assert!(q.push_hover(hover(Some("dom:2"), 2.)));
    assert_eq!(q.failed("retry").as_deref(), Some("dom:1"));
    assert!(!q.has_hovers());
    assert!(q.push_hover(hover(Some("dom:1"), 1.)));
    q.prepared(Some("move"));
    q.clear_waiting();
    assert!(q.has_hovers()); // clear 保留在途证据，settle 才释放。
    q.complete_local("move", None);
    assert!(!q.has_hovers());
}
