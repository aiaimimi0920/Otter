use super::*;
use std::{cell::RefCell, rc::Rc};
#[derive(Default)]
struct Trace {
    denied: bool,
    gone: bool,
    fail_next: bool,
    fail_cleanup: bool,
    target: u8,
    sent: Vec<(u8, Event, u8, u32)>,
}
struct Mock(Rc<RefCell<Trace>>);
impl Backend for Mock {
    type Target = u8;
    type Error = &'static str;
    fn preflight(&mut self) -> Result<(), Self::Error> {
        if self.0.borrow().denied {
            Err("denied")
        } else {
            Ok(())
        }
    }
    fn resolve(&mut self, _: Event, preferred: Option<u8>) -> Result<u8, Self::Error> {
        Ok(preferred.unwrap_or(self.0.borrow().target))
    }
    fn send(
        &mut self,
        target: u8,
        event: Event,
        buttons: u8,
        timeout: u32,
    ) -> Result<(), Self::Error> {
        let mut trace = self.0.borrow_mut();
        trace.sent.push((target, event, buttons, timeout));
        if std::mem::take(&mut trace.fail_next) || (trace.fail_cleanup && timeout == 50) {
            Err("timeout")
        } else {
            Ok(())
        }
    }
    fn cleanup_target(&mut self, target: u8) -> Option<u8> {
        (!self.0.borrow().gone).then_some(target)
    }
    fn release(&mut self, target: u8, event: Event, buttons: u8) -> Result<(), Self::Error> {
        self.send(target, event, buttons, 50)
    }
}
fn fixture() -> (Controller<Mock>, Rc<RefCell<Trace>>) {
    let trace = Rc::new(RefCell::new(Trace {
        target: 1,
        ..Default::default()
    }));
    (Controller::new(Mock(trace.clone())), trace)
}
fn key(code: u16, down: bool) -> Event {
    Event::Key { code, down }
}
fn button(button: Button, down: bool) -> Event {
    Event::Button {
        point: [0.5, 0.5],
        button,
        down,
        double: false,
    }
}
#[test]
fn capture_does_not_enable_and_sequence_survives_cancel_and_reenable() {
    let (mut owner, trace) = fixture();
    assert_eq!(owner.dispatch(1, key(65, true)), Err(Error::Disabled));
    assert!(trace.borrow().sent.is_empty());
    owner.enable().unwrap();
    owner.dispatch(2, key(65, true)).unwrap();
    assert_eq!(
        owner.cancel(),
        Cleanup {
            released: 1,
            ..Default::default()
        }
    );
    owner.enable().unwrap();
    assert_eq!(owner.dispatch(2, key(65, true)), Err(Error::Sequence));
    assert!(!owner.enabled());
    assert_eq!(owner.last_sequence(), 2);
    owner.enable().unwrap();
    assert_eq!(
        owner.dispatch(MAX_SEQUENCE + 1, key(65, true)),
        Err(Error::Sequence)
    );
}
#[test]
fn ambiguous_down_is_consumed_and_released_without_replaying_down() {
    let (mut owner, trace) = fixture();
    owner.enable().unwrap();
    trace.borrow_mut().fail_next = true;
    assert_eq!(
        owner.dispatch(1, button(Button::Left, true)),
        Err(Error::Backend("timeout"))
    );
    assert_eq!(owner.last_sequence(), 1);
    assert!(!owner.enabled());
    assert_eq!(owner.last_cleanup().released, 1);
    assert_eq!(
        trace.borrow().sent,
        vec![
            (1, button(Button::Left, true), 1, 250),
            (1, button(Button::Left, false), 0, 50),
        ]
    );
    owner.enable().unwrap();
    assert_eq!(
        owner.dispatch(1, button(Button::Left, true)),
        Err(Error::Sequence)
    );
    assert_eq!(trace.borrow().sent.len(), 2);
}
#[test]
fn key_limit_is_bounded_and_repeat_keeps_original_key_target() {
    let (mut owner, trace) = fixture();
    owner.enable().unwrap();
    for code in 1..=32 {
        owner.dispatch(u64::from(code), key(code, true)).unwrap();
    }
    trace.borrow_mut().target = 2;
    owner.dispatch(33, key(1, true)).unwrap();
    assert_eq!(trace.borrow().sent.last().unwrap().0, 1);
    assert_eq!(owner.dispatch(34, key(33, true)), Err(Error::KeyLimit));
    assert_eq!(owner.last_cleanup().released, 32);
    assert!(!owner.enabled());
    assert_eq!(trace.borrow().sent.len(), 65);
}
#[test]
fn multi_button_gesture_pins_target_and_keyboard_up_returns_to_down_target() {
    let (mut owner, trace) = fixture();
    owner.enable().unwrap();
    owner.dispatch(1, button(Button::Left, true)).unwrap();
    owner.dispatch(2, key(65, true)).unwrap();
    trace.borrow_mut().target = 2;
    owner.dispatch(3, button(Button::Right, true)).unwrap();
    owner.dispatch(4, button(Button::Left, false)).unwrap();
    owner.dispatch(5, button(Button::Right, false)).unwrap();
    owner.dispatch(6, button(Button::Left, true)).unwrap();
    owner.dispatch(7, key(65, false)).unwrap();
    let events = &trace.borrow().sent;
    assert_eq!(events[2].0, 1);
    assert_eq!(events[3].2, 2);
    assert_eq!(events[4].2, 0);
    assert_eq!(events[5].0, 2);
    assert_eq!(events[6].0, 1);
}
#[test]
fn invalid_payload_consumes_admitted_sequence_without_delivery() {
    for event in [
        Event::Move {
            point: [f64::NAN, 0.0],
        },
        Event::Move { point: [-0.1, 0.0] },
        Event::Wheel {
            point: [0.0; 2],
            delta: i32::MIN,
            horizontal: false,
        },
        Event::Wheel {
            point: [0.0; 2],
            delta: 0,
            horizontal: true,
        },
        key(0, true),
        key(255, true),
    ] {
        let (mut owner, trace) = fixture();
        owner.enable().unwrap();
        assert_eq!(owner.dispatch(1, event), Err(Error::InvalidEvent));
        assert_eq!(owner.last_sequence(), 1);
        assert!(trace.borrow().sent.is_empty());
        assert!(!owner.enabled());
    }
}
#[test]
fn revoked_authority_cleans_old_state_without_consuming_new_sequence() {
    let (mut owner, trace) = fixture();
    owner.enable().unwrap();
    owner.dispatch(1, key(65, true)).unwrap();
    trace.borrow_mut().denied = true;
    trace.borrow_mut().gone = true;
    assert_eq!(
        owner.dispatch(2, key(66, true)),
        Err(Error::Backend("denied"))
    );
    assert_eq!(owner.last_sequence(), 1);
    assert_eq!(owner.last_cleanup().skipped, 1);
    assert_eq!(trace.borrow().sent.len(), 1);
    assert!(!owner.enabled());
}
#[test]
fn cleanup_failure_is_reported_and_drop_does_not_repeat_attempted_releases() {
    let (mut owner, trace) = fixture();
    owner.enable().unwrap();
    owner.dispatch(1, key(65, true)).unwrap();
    trace.borrow_mut().fail_cleanup = true;
    assert_eq!(owner.cancel().failed, 1);
    drop(owner);
    assert_eq!(trace.borrow().sent.len(), 2);
    let (mut owner, trace) = fixture();
    owner.enable().unwrap();
    owner.dispatch(1, key(65, true)).unwrap();
    drop(owner);
    assert_eq!(trace.borrow().sent.last().unwrap().1, key(65, false));
}

#[test]
fn failed_up_still_gets_cleanup_but_acknowledged_up_is_not_repeated() {
    let (mut owner, trace) = fixture();
    owner.enable().unwrap();
    owner.dispatch(1, key(65, true)).unwrap();
    trace.borrow_mut().fail_next = true;
    assert_eq!(
        owner.dispatch(2, key(65, false)),
        Err(Error::Backend("timeout"))
    );
    assert_eq!(owner.last_cleanup().released, 1);
    assert_eq!(trace.borrow().sent.len(), 3);
    owner.enable().unwrap();
    owner.dispatch(3, key(65, true)).unwrap();
    owner.dispatch(4, key(65, false)).unwrap();
    assert_eq!(owner.cancel(), Cleanup::default());
    assert_eq!(trace.borrow().sent.len(), 5);
}
