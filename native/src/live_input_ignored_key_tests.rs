//! 未投递的模态键盘边沿不能制造 held 状态，也不能抹掉旧释放责任。
use super::*;

struct Sink {
    result: Delivery,
    target: u8,
    sent: Vec<(u8, Event)>,
    released: Vec<(u8, Event)>,
}
impl Backend for Sink {
    type Target = u8;
    type Error = ();
    fn preflight(&mut self) -> Result<(), ()> {
        Ok(())
    }
    fn delivery(&mut self) -> Delivery {
        self.result
    }
    fn resolve(&mut self, _: Event, preferred: Option<u8>) -> Result<u8, ()> {
        Ok(preferred.unwrap_or(self.target))
    }
    fn send(&mut self, target: u8, event: Event, _: u8, _: u32) -> Result<(), ()> {
        if self.result != Delivery::IgnoredWhileModal {
            self.sent.push((target, event));
        }
        Ok(())
    }
    fn cleanup_target(&mut self, target: u8) -> Option<u8> {
        Some(target)
    }
    fn release(&mut self, target: u8, event: Event, _: u8) -> Result<(), ()> {
        self.released.push((target, event));
        Ok(())
    }
}
fn owner() -> Controller<Sink> {
    let mut owner = Controller::new(Sink {
        result: Delivery::Completed,
        target: 1,
        sent: vec![],
        released: vec![],
    });
    owner.enable().unwrap();
    owner
}
fn escape(down: bool) -> Event {
    Event::Key { code: 0x1b, down }
}

#[test]
fn ignored_new_escape_down_does_not_create_a_cleanup_release() {
    let mut owner = owner();
    owner.dispatch(1, escape(true)).unwrap();
    owner.dispatch(2, escape(false)).unwrap();
    owner.backend.result = Delivery::IgnoredWhileModal;
    assert_eq!(
        owner.dispatch_status(3, escape(true)),
        Ok(Delivery::IgnoredWhileModal)
    );
    assert_eq!(owner.cancel().released, 0);
    assert!(owner.backend.released.is_empty());
    assert_eq!(owner.backend.sent.len(), 2);
}

#[test]
fn ignored_repeat_preserves_original_held_key_and_destination() {
    let mut owner = owner();
    owner.dispatch(1, escape(true)).unwrap();
    owner.backend.target = 2;
    owner.backend.result = Delivery::IgnoredWhileModal;
    owner.dispatch(2, escape(true)).unwrap();
    assert_eq!(owner.cancel().released, 1);
    assert_eq!(owner.backend.released, [(1, escape(false))]);
}

#[test]
fn ignored_key_down_does_not_pin_the_next_character_to_its_target() {
    let mut owner = owner();
    owner.backend.result = Delivery::IgnoredWhileModal;
    owner.dispatch(1, escape(true)).unwrap();
    owner.backend.target = 2;
    owner.backend.result = Delivery::Completed;
    let text = Event::Character { unit: 65 };
    owner.dispatch(2, text).unwrap();
    assert_eq!(owner.backend.sent, [(2, text)]);
    assert_eq!(owner.cancel().released, 0);
}

#[test]
fn ignored_key_up_does_not_discard_a_real_held_key() {
    let mut owner = owner();
    owner.dispatch(1, escape(true)).unwrap();
    owner.backend.result = Delivery::IgnoredWhileModal;
    owner.dispatch(2, escape(false)).unwrap();
    assert_eq!(owner.cancel().released, 1);
    assert_eq!(owner.backend.released, [(1, escape(false))]);
}
