//! 临时队列过载释放输入，但不能等同于用户关闭控制；积压不重放。
use super::*;

#[test]
fn overload_preserves_intent_but_discards_pending_input_and_buttons() {
    let mut input = Control {
        allowed: true,
        desired: true,
        enabled: true,
        buttons: 1,
        ..Control::default()
    };
    for _ in 0..queue::CAPACITY {
        assert!(input.push(Event::Character { unit: 65 }));
    }
    assert!(!input.push(Event::Character { unit: 66 }));
    assert!(input.desired);
    assert!(input.allowed);
    assert!(!input.enabled);
    assert!(!input.pending);
    assert_eq!(input.request, Some(false));
    assert!(input.events.is_empty());
    assert_eq!(input.buttons, 0);
    assert!(!input.failed);
}

#[test]
fn explicit_cancel_still_revokes_intent_and_pause_does_not_restore_it() {
    let mut input = Control {
        desired: true,
        enabled: true,
        ..Control::default()
    };
    input.cancel();
    input.pause();
    assert!(!input.desired);
    assert!(!input.enabled);
    assert_eq!(input.request, Some(false));
}
