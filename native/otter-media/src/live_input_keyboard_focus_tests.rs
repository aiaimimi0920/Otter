//! New keys/text follow focus, while held repeats/releases retain original targets; no system-focus changes.
use super::*;

#[derive(Default)]
struct Sink {
    focus: Option<u8>,
    sent: Vec<(u8, Event)>,
    released: Vec<(u8, Event)>,
}
impl Backend for Sink {
    type Target = u8;
    type Error = ();
    fn preflight(&mut self) -> Result<(), ()> {
        Ok(())
    }
    fn resolve(&mut self, _: Event, preferred: Option<u8>) -> Result<u8, ()> {
        Ok(preferred.unwrap_or(1))
    }
    fn resolve_keyboard(&mut self, event: Event, fallback: Option<u8>) -> Result<u8, ()> {
        match self.focus {
            Some(focus) => Ok(focus),
            None => self.resolve(event, fallback),
        }
    }
    fn send(&mut self, target: u8, event: Event, _: u8, _: u32) -> Result<(), ()> {
        self.sent.push((target, event));
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
fn key(down: bool) -> Event {
    Event::Key { code: 65, down }
}

#[test]
fn fresh_keys_follow_focus_but_repeats_up_and_cancel_keep_original_destination() {
    let mut owner = Controller::new(Sink::default());
    owner.enable().unwrap();
    // Previously clicked controls must not permanently capture subsequent new keys.
    for (sequence, down) in [(1, true), (2, false)] {
        owner
            .dispatch(
                sequence,
                Event::Button {
                    point: [0.5, 0.5],
                    button: Button::Left,
                    down,
                    double: false,
                },
            )
            .unwrap();
    }
    owner.backend.focus = Some(2);
    owner.dispatch(3, key(true)).unwrap();
    owner.backend.focus = Some(3);
    owner.dispatch(4, key(true)).unwrap();
    owner.dispatch(5, key(false)).unwrap();
    owner.dispatch(6, key(true)).unwrap();
    assert_eq!(
        &owner.backend.sent[2..],
        &[
            (2, key(true)),
            (2, key(true)),
            (2, key(false)),
            (3, key(true)),
        ]
    );
    owner.backend.focus = Some(4);
    assert_eq!(owner.cancel().released, 1);
    assert_eq!(owner.backend.released, [(3, key(false))]);
}

#[test]
fn committed_text_uses_current_focus_instead_of_last_key_destination() {
    let mut owner = Controller::new(Sink::default());
    owner.enable().unwrap();
    owner.dispatch(1, key(true)).unwrap();
    owner.dispatch(2, key(false)).unwrap();
    owner.backend.focus = Some(2);
    let text = Event::Character { unit: 0x4e2d };
    owner.dispatch(3, text).unwrap();
    owner.backend.focus = Some(3);
    owner.dispatch(4, text).unwrap();
    assert_eq!(&owner.backend.sent[2..], &[(2, text), (3, text)]);
    assert_eq!(owner.cancel().released, 0);
}
