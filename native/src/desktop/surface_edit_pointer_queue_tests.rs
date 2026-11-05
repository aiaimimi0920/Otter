//! Pointer 共享 FIFO 的终止容量、序号、合并和取消/fence 合同。
use super::*;
#[test]
fn javascript_pointer_queue_cancel_reuses_terminal_sequence_at_max() {
    let mut queue = Queue {
        sequence: u64::MAX - 2,
        ..Default::default()
    };
    assert!(queue.push_pointer(pointer(PointerPhase::Pointerdown, 0.)));
    queue.prepared(Some("down"));
    queue.complete_local("down", None);
    assert!(queue.push_pointer(pointer(PointerPhase::Pointerup, 1.)));
    assert_eq!(queue.sequence, u64::MAX);
    assert!(queue.cancel_pointers(5));
    let intent = queue.front().unwrap();
    assert_eq!(intent.sequence, u64::MAX);
    let Edge::Pointer(p) = &intent.edge else {
        panic!()
    };
    assert_eq!(p.input.phase, PointerPhase::Pointercancel);
    assert!(queue.cancel_pointers(5));
    assert_eq!(queue.waiting.len(), 1);
}
#[test]
fn javascript_pointer_queue_mixed_cancel_updates_epoch_and_keeps_nonpointer_order() {
    let mut queue = Queue::default();
    assert!(queue.push_pointer(pointer(PointerPhase::Pointerdown, 0.)));
    queue.prepared(Some("down"));
    assert!(queue.push_pointer(pointer(PointerPhase::Pointermove, 1.)));
    let sequence = queue.front().unwrap().sequence;
    assert!(queue.push("text-a", Phase::Change, "a"));
    assert!(queue.push_pointer(pointer(PointerPhase::Pointerup, 2.)));
    assert!(queue.push_keyboard("text-b", "b", 0, 0, 0));
    let fence = queue.seal();
    assert!(queue.cancel_pointers(5));
    let mut cancel = pointer(PointerPhase::Pointercancel, 3.);
    cancel.viewport_epoch = 6;
    assert!(queue.cancel_pointer(cancel.clone()));
    assert_eq!(
        queue
            .waiting
            .iter()
            .map(|i| i.node.as_str())
            .collect::<Vec<_>>(),
        ["dom:2", "text-a", "text-b"]
    );
    assert_eq!(queue.front().unwrap().sequence, sequence);
    let Edge::Pointer(p) = &queue.front().unwrap().edge else {
        panic!()
    };
    assert_eq!(p.input.phase, PointerPhase::Pointercancel);
    assert_eq!(p.viewport_epoch, 6);
    assert_eq!(queue.state(&fence), FenceState::Invalid);
    assert_eq!(queue.prepared.len(), 1);
    assert!(queue.cancel_pointer(cancel));
    assert!(queue.cancel_pointers(7));
    assert_eq!(queue.waiting.len(), 3);
    let Edge::Pointer(p) = &queue.front().unwrap().edge else {
        panic!()
    };
    assert_eq!(p.viewport_epoch, 7);
}
#[test]
fn javascript_pointer_queue_hard_cancel_covers_queued_up_and_repeated_cancel_keeps_one_terminal() {
    for started in [false, true] {
        let mut queue = Queue::default();
        assert!(queue.push_pointer(pointer(PointerPhase::Pointerdown, 0.)));
        if started {
            queue.prepared(Some("down"));
            queue.complete_local("down", None);
        }
        assert!(queue.push_pointer(pointer(PointerPhase::Pointerup, 1.)));
        assert!(queue.pointer_reservation.is_none());
        let fence = queue.seal();
        assert!(queue.cancel_pointers(5));
        assert!(queue.cancel_pointers(6));
        assert_eq!(queue.state(&fence), FenceState::Invalid);
        assert_eq!(queue.waiting.len(), usize::from(started));
        if started {
            let Edge::Pointer(p) = &queue.front().unwrap().edge else {
                panic!()
            };
            assert_eq!(p.input.phase, PointerPhase::Pointercancel);
            assert_eq!(p.viewport_epoch, 6);
        }
    }
}
fn pointer(phase: PointerPhase, x: f64) -> PointerIntent {
    PointerIntent {
        input: PointerInput {
            target: "dom:2".into(),
            phase,
            client: [x, 1.],
            modifiers: 0,
            click: false,
        },
        gesture: 1,
        viewport_epoch: 4,
    }
}
#[test]
fn javascript_pointer_queue_reserves_terminal_slot_and_sequence_against_all_writers() {
    let mut queue = Queue::default();
    for _ in 0..30 {
        assert!(queue.push("text", Phase::Change, "a"));
    }
    assert!(queue.push_pointer(pointer(PointerPhase::Pointerdown, 0.)));
    assert!(!queue.can_push_change());
    assert!(!queue.can_push_step("text"));
    assert!(!queue.push_keyboard("text", "a", 0, 0, 0));
    assert!(!queue.push_pointer(pointer(PointerPhase::Pointermove, 2.)));
    assert!(queue.push_pointer(pointer(PointerPhase::Pointerup, 3.)));
    assert_eq!(queue.waiting.len(), 32);
    assert!(!queue.push_pointer(pointer(PointerPhase::Pointerup, 4.)));
    for remaining in 0..=2 {
        let mut queue = Queue {
            sequence: u64::MAX - remaining,
            ..Default::default()
        };
        assert_eq!(
            queue.push_pointer(pointer(PointerPhase::Pointerdown, 0.)),
            remaining == 2
        );
        if remaining == 2 {
            assert!(!queue.push("text", Phase::Input, "a"));
            assert!(queue.push_pointer(pointer(PointerPhase::Pointerup, 1.)));
            assert_eq!(queue.sequence, u64::MAX);
        }
    }
}
#[test]
fn javascript_pointer_queue_coalesces_only_same_unsealed_tail_not_inflight_or_other_input() {
    let mut queue = Queue::default();
    assert!(queue.push_pointer(pointer(PointerPhase::Pointerdown, 0.)));
    queue.prepared(Some("down"));
    queue.complete_local("down", None);
    assert!(queue.push_pointer(pointer(PointerPhase::Pointermove, 1.)));
    assert!(queue.push_pointer(pointer(PointerPhase::Pointermove, 2.)));
    assert_eq!(queue.waiting.len(), 1);
    let fence = queue.seal();
    assert!(queue.push_pointer(pointer(PointerPhase::Pointermove, 3.)));
    assert_eq!(queue.waiting.len(), 2);
    queue.prepared(Some("move"));
    assert!(queue.push_pointer(pointer(PointerPhase::Pointermove, 4.)));
    let Edge::Pointer(prepared) = &queue.prepared_intent("move").unwrap().edge else {
        panic!()
    };
    assert_eq!(prepared.input.client[0], 2.);
    assert!(queue.push_keyboard("text", "a", 0, 0, 0));
    assert!(queue.push_pointer(pointer(PointerPhase::Pointermove, 5.)));
    assert_eq!(queue.waiting.len(), 3);
    assert_eq!(queue.state(&fence), FenceState::InFlight);
    queue.complete_local("move", None);
    assert_eq!(queue.state(&fence), FenceState::Ready);
}
#[test]
fn javascript_pointer_queue_cancel_does_not_invent_down_or_rewrite_prepared_and_invalidates_fence()
{
    for started in [false, true] {
        let mut queue = Queue::default();
        assert!(queue.push_pointer(pointer(PointerPhase::Pointerdown, 0.)));
        if started {
            queue.prepared(Some("down"));
        }
        assert!(queue.push_pointer(pointer(PointerPhase::Pointermove, 1.)));
        let fence = queue.seal();
        assert!(queue.cancel_pointer(pointer(PointerPhase::Pointercancel, 2.)));
        assert_eq!(queue.state(&fence), FenceState::Invalid);
        assert_eq!(queue.waiting.len(), usize::from(started));
        assert_eq!(queue.prepared.len(), usize::from(started));
        if started {
            let Edge::Pointer(input) = &queue.front().unwrap().edge else {
                panic!()
            };
            assert_eq!(input.input.phase, PointerPhase::Pointercancel);
        }
        assert!(queue.pointer_reservation.is_none());
    }
}
#[test]
fn javascript_pointer_queue_field_retention_keeps_canvas_ticket_but_policy_clear_revokes_reservation(
) {
    let mut queue = Queue::default();
    assert!(queue.push_pointer(pointer(PointerPhase::Pointerdown, 0.)));
    queue.prepared(Some("down"));
    assert!(queue.push_pointer(pointer(PointerPhase::Pointermove, 2.)));
    queue.retain(|_| false);
    assert_eq!(queue.prepared.len(), 1);
    assert_eq!(queue.waiting.len(), 1);
    queue.clear_waiting();
    assert!(queue.pointer_reservation.is_none());
    assert!(!queue.push_pointer(pointer(PointerPhase::Pointerup, 2.)));
    queue.complete_local("down", None);
    assert!(queue.prepared.is_empty());
}
