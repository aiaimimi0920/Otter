//! 本地 UI 准入状态：松开只归持有 down 的 B，交接后迟到 up 不得改绑或挤占队列。
use super::*;

fn control() -> Control {
    Control {
        allowed: true,
        desired: true,
        enabled: true,
        ..Control::default()
    }
}
fn key(code: u16, down: bool) -> Event {
    Event::Key { code, down }
}
fn queued(input: &mut Control) -> Vec<Event> {
    input.events.drain(..).collect()
}

#[test]
fn source_handoff_keeps_original_fifo_release_and_ignores_late_physical_up() {
    let mut input = control();
    let down = key(65, true);
    let up = key(65, false);
    assert!(input.push(down));
    assert!(input.handoff_source());
    assert_eq!(input.held_keys, [0; 4]);
    assert!(input.push(up));
    assert!(input.push(up));
    assert_eq!(queued(&mut input), [down, up]);
    assert!(input.enabled);
    assert_eq!(input.request, None);
    assert!(input.push(down));
    assert!(input.push(up));
    assert_eq!(queued(&mut input), [down, up]);
}

#[test]
fn repeated_down_keeps_repeats_but_handoff_still_releases_only_once() {
    let mut input = control();
    let down = key(66, true);
    let up = key(66, false);
    for _ in 0..3 {
        assert!(input.push(down));
    }
    assert!(input.handoff_keys());
    assert!(input.handoff_keys());
    assert!(input.push(up));
    assert_eq!(queued(&mut input), [down, down, down, up]);
    assert!(input.enabled);
    assert_eq!(input.held_keys, [0; 4]);
}

#[test]
fn unmatched_release_cannot_fill_queue_or_turn_a_full_queue_into_false_overload() {
    let mut input = control();
    for _ in 0..queue::CAPACITY {
        assert!(input.push(Event::Character { unit: 65 }));
    }
    for _ in 0..queue::CAPACITY {
        assert!(input.push(key(65, false)));
    }
    assert_eq!(input.events.len(), queue::CAPACITY);
    assert!(input.enabled);
    assert_eq!(input.request, None);
    // 真正的新边沿仍按原容量失败并暂停，不能借去重放宽有界队列。
    assert!(!input.push(key(65, true)));
    assert!(!input.enabled);
    assert_eq!(input.request, Some(false));
    assert!(input.events.is_empty());
}

#[test]
fn pause_or_cancel_clears_release_ownership_before_a_fresh_enable() {
    for cancel in [true, false] {
        let mut input = control();
        assert!(input.push(key(65, true)));
        if cancel {
            input.cancel();
        } else {
            input.pause();
        }
        assert_eq!(input.held_keys, [0; 4]);
        assert!(input.events.is_empty());
        // 模拟原 worker 清理后的一次明确新启用，不复活上一周期的 held。
        input.enabled = true;
        input.desired = true;
        input.request = None;
        assert!(input.push(key(65, false)));
        assert!(input.events.is_empty());
        assert!(input.enabled);
        assert_eq!(input.request, None);
    }
}

#[test]
fn each_valid_key_only_releases_its_own_down_and_utf16_characters_stay_unchanged() {
    for code in 1..=254 {
        let mut input = control();
        let other = if code == 254 { 1 } else { code + 1 };
        assert!(input.push(key(code, false)));
        assert!(input.events.is_empty());
        assert!(input.push(key(code, true)));
        assert!(input.push(key(other, false)));
        assert!(input.push(key(code, false)));
        assert_eq!(queued(&mut input), [key(code, true), key(code, false)]);
        assert_eq!(input.held_keys, [0; 4]);
    }
    let mut input = control();
    let characters = [0xd83d, 0xde00, 0x4e2d].map(|unit| Event::Character { unit });
    for event in characters {
        assert!(input.push(event));
    }
    assert_eq!(queued(&mut input), characters);
}
