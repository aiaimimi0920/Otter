//! 独立构造能力与队列状态；不依赖HTTP接受回包充当实际Source执行。
use super::*;
fn configured(names: &[&str]) -> Queue {
    let queue = Queue::default();
    let bytes =
        serde_json::to_vec(&serde_json::json!({"session":{"interactionCapabilities":names}}))
            .unwrap();
    {
        let mut state = queue.state();
        state.ready = true;
        state.capabilities = Capabilities::parse(&bytes).unwrap();
    }
    queue
}
fn key() -> Event {
    Event::Key {
        code: 65,
        down: true,
    }
}
#[test]
fn media_and_control_recovery_cannot_clear_each_other_or_reactivate_input() {
    let queue = configured(&["keyboard", "cancel"]);
    queue.request();
    queue.mark_possible();
    queue.acknowledge(true);
    queue.push(key());
    queue.control_recovering(true);
    queue.media_recovering(true);
    queue.media_recovering(false);
    assert!(queue.status().recovering);
    assert!(queue.control_unavailable());
    assert!(!queue.request());
    queue.acknowledge(true);
    assert!(!queue.status().enabled);
    queue.control_recovering(false);
    assert!(!queue.status().recovering);
    assert!(queue.releasing());
    assert!(queue.take().is_none());
    queue.media_recovering(true);
    queue.control_recovering(true);
    queue.control_recovering(false);
    assert!(queue.status().recovering);
    assert!(!queue.request());
    queue.media_recovering(false);
    assert!(!queue.status().enabled);
}
#[test]
fn media_loss_discards_edges_and_late_grant_requires_fresh_explicit_request() {
    let queue = configured(&["keyboard", "cancel"]);
    assert!(queue.request());
    queue.mark_possible();
    queue.acknowledge(true);
    assert!(queue.push(key()));
    queue.media_recovering(true);
    assert!(queue.status().recovering);
    assert!(!queue.request());
    assert!(!queue.push(key()));
    queue.acknowledge(true);
    assert!(!queue.status().enabled);
    assert!(queue.releasing());
    assert!(queue.take().is_none());
    queue.media_recovering(false);
    assert!(!queue.status().enabled);
    assert!(queue.request());
    assert!(queue.take().is_none());
    queue.acknowledge(false);
    queue.mark_possible();
    queue.acknowledge(true);
    assert!(queue.status().enabled);
    assert!(queue.take().is_none());
    queue.stop();
    queue.media_recovering(false);
    assert!(!queue.request());
}
#[test]
fn default_or_missing_capability_never_grants_control() {
    for queue in [Queue::default(), configured(&[]), configured(&["keyboard"])] {
        assert!(!queue.request());
        assert!(!queue.push(key()));
        assert!(!queue.status().enabled);
    }
    for bytes in [
        br#"{"session":{"interactionCapabilities":["cancel","cancel"]}}"#.as_slice(),
        br#"{"session":{"interactionCapabilities":["invented"]}}"#,
    ] {
        assert!(Capabilities::parse(bytes).is_err());
    }
}
#[test]
fn retired_host_is_unavailable_but_keeps_release_until_acknowledged() {
    let queue = configured(&["keyboard", "cancel"]);
    assert!(queue.request());
    queue.mark_possible();
    queue.acknowledge(true);
    assert!(queue.push(key()));
    queue.retire_host();
    assert!(queue.cleanup_needed());
    assert!(queue.releasing());
    assert!(!queue.status().available);
    assert!(!queue.status().enabled);
    assert!(!queue.request());
    assert!(queue.take().is_none());
    queue.acknowledge(false);
    assert!(!queue.cleanup_needed());
    assert!(!queue.request());
    assert!(!queue.status().available);
}
#[test]
fn explicit_request_requires_actual_grant_and_stopping_is_terminal() {
    let queue = configured(&["keyboard", "cancel"]);
    assert!(queue.request());
    assert!(queue.status().pending);
    assert!(!queue.push(key()));
    queue.mark_possible();
    queue.acknowledge(true);
    assert!(queue.status().enabled);
    assert!(queue.push(key()));
    queue.stop();
    assert!(queue.cleanup_needed());
    assert!(!queue.request());
    assert!(queue.take().is_none());
    queue.acknowledge(true);
    assert!(!queue.status().enabled);
    queue.acknowledge(false);
    assert!(!queue.cleanup_needed());
}
#[test]
fn coalescing_is_bounded_and_edge_overflow_forces_release() {
    let queue = configured(&["pointer_move", "keyboard", "cancel"]);
    queue.request();
    queue.mark_possible();
    queue.acknowledge(true);
    for value in 0..100 {
        assert!(queue.push(Event::Move {
            point: [f64::from(value) / 100.0, 0.5]
        }));
    }
    assert_eq!(queue.state().events.len(), 1);
    assert_eq!(queue.take(), Some(Event::Move { point: [0.99, 0.5] }));
    for _ in 0..16 {
        assert!(queue.push(key()));
    }
    assert!(!queue.push(key()));
    assert!(queue.status().failed);
    assert!(queue.releasing());
    assert!(queue.take().is_none());
    assert!(!queue.status().enabled);
}
#[test]
fn cancel_then_immediate_reenable_cannot_skip_old_release_barrier() {
    let queue = configured(&["keyboard", "cancel"]);
    queue.request();
    queue.mark_possible();
    queue.acknowledge(true);
    queue.push(key());
    queue.release();
    assert!(queue.request());
    queue.acknowledge(true);
    assert!(queue.releasing());
    assert!(!queue.status().enabled);
    assert!(queue.take().is_none());
    queue.acknowledge(false);
    assert!(queue.status().pending);
    queue.mark_possible();
    queue.acknowledge(true);
    assert!(queue.status().enabled);
}
#[test]
fn unsupported_or_invalid_input_cannot_enter_queue_or_leave_held_state_active() {
    for input in [
        Event::Move { point: [0.5, 0.5] },
        Event::Key {
            code: 0,
            down: true,
        },
    ] {
        let queue = configured(&["keyboard", "cancel"]);
        queue.request();
        queue.mark_possible();
        queue.acknowledge(true);
        assert!(!queue.push(input));
        assert!(!queue.status().enabled);
        assert!(queue.releasing());
    }
}
