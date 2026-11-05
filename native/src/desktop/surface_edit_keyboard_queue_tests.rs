//! 默认动作保持原 sequence/capacity/fence；取消、过期和策略代次不复活旧请求。
use super::*;
#[test]
fn javascript_native_enter_default_replaces_its_slot_before_keyup_later_input_and_click_fence() {
    let mut queue = Queue::default();
    assert!(queue.push("a", Phase::Input, "prefix"));
    assert!(queue.push_keyboard("a", "prefix", 0, 2, 1));
    let fence = queue.seal();
    assert!(queue.push_keyboard("a", "prefix", 1, 2, 1));
    assert!(queue.push("a", Phase::Input, "later"));
    queue.prepared(Some("input"));
    queue.settle("input");
    let key = queue.front().unwrap().clone();
    let key_id = key.local_id();
    queue.prepared(Some(&key_id));
    assert_eq!(queue.state(&fence), FenceState::InFlight);
    queue.complete_local(&key_id, Some("prefix"));
    let change = queue.front().unwrap();
    assert_eq!(change.sequence, key.sequence);
    assert_eq!(change.value, "prefix");
    assert!(matches!(change.edge, Edge::Text(Phase::Change)));
    assert_ne!(change.local_id(), key_id);
    assert_eq!(queue.state(&fence), FenceState::Pending);
    queue.prepared(Some("default"));
    queue.settle("default");
    assert_eq!(queue.state(&fence), FenceState::Ready);
    assert!(matches!(
        queue.front().unwrap().edge,
        Edge::Keyboard { flags: 1, .. }
    ));
    queue.prepared(Some("up"));
    queue.complete_local("up", None);
    assert_eq!(queue.front().unwrap().value, "later");
}
#[test]
fn javascript_native_enter_queue_capacity_expiry_policy_scope_and_cancel_remain_bounded() {
    let mut queue = Queue::default();
    for _ in 0..32 {
        assert!(queue.push_keyboard("a", "", 0, 0, 0));
    }
    assert!(!queue.push_keyboard("a", "", 1, 0, 0));
    queue.prepared(Some("key"));
    assert!(!queue.push("a", Phase::Input, "full"));
    assert!(queue.default_current(queue.prepared_intent("key").unwrap()));
    let original = queue.prepared_intent("key").unwrap().sequence;
    queue.cancel("key");
    assert_eq!(queue.front().unwrap().sequence, original);
    queue.prepared(Some("key"));
    queue.clear_waiting();
    assert!(!queue.default_current(queue.prepared_intent("key").unwrap()));
    queue.prepared.get_mut("key").unwrap().expires = Instant::now();
    assert!(!queue.default_current(queue.prepared_intent("key").unwrap()));
    queue.complete_local("key", None);
    assert!(queue.front().is_none());
    queue.sequence = u64::MAX;
    assert!(!queue.push_keyboard("a", "", 0, 0, 0));
}
