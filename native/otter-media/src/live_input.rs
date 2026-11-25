//! Bounded Live-input control state; transport owns identity/permissions/coordinate conversion/timeouts without implicit authorization.
pub const MAX_SEQUENCE: u64 = 9_007_199_254_740_991;
const MAX_KEYS: usize = 32;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left,
    Right,
    Middle,
}
impl Button {
    fn mask(self) -> u8 {
        match self {
            Self::Left => 1,
            Self::Right => 2,
            Self::Middle => 4,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Local window messages do not automatically expose text/hover capabilities in remote contracts.
    Character {
        unit: u16,
    },
    Hover {
        point: [f64; 2],
    },
    Leave,
    Move {
        point: [f64; 2],
    },
    Button {
        point: [f64; 2],
        button: Button,
        down: bool,
        double: bool,
    },
    Wheel {
        point: [f64; 2],
        delta: i32,
        horizontal: bool,
    },
    Key {
        code: u16,
        down: bool,
    },
}
impl Event {
    pub fn message_timeout_ms(self) -> u32 {
        if matches!(self, Self::Move { .. } | Self::Hover { .. } | Self::Leave) {
            50
        } else {
            250
        }
    }
    fn point(self) -> Option<[f64; 2]> {
        match self {
            Self::Move { point }
            | Self::Hover { point }
            | Self::Button { point, .. }
            | Self::Wheel { point, .. } => Some(point),
            _ => None,
        }
    }
    pub fn valid(self) -> bool {
        if self.point().is_some_and(|point| {
            point
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        }) {
            return false;
        }
        match self {
            Self::Key { code, .. } => (1..=254).contains(&code),
            Self::Character { unit } => unit != 0,
            Self::Wheel { delta, .. } => delta != 0 && delta.unsigned_abs() <= 1200,
            Self::Button {
                down: false,
                double: true,
                ..
            } => false,
            _ => true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error<E> {
    Disabled,
    Sequence,
    InvalidEvent,
    KeyLimit,
    Backend(E),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cleanup {
    pub released: u8,
    pub failed: u8,
    pub skipped: u8,
}
/// Completed means window procedures returned; Pending means only local modal input submitted in bounded slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delivery {
    Completed,
    Pending,
    IgnoredWhileModal,
}
/// Input workers only. Implementations revalidate root/child and reject raw-HWND reuse.
pub trait Backend {
    type Target: Clone;
    type Error;
    /// Observe asynchronous completion; Controller revokes on failure and restores original held state without waiting for new input.
    fn idle(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn delivery(&mut self) -> Delivery {
        Delivery::Completed
    }
    fn pending_delivery(&mut self) -> bool {
        false
    }
    /// While modal release remains unfinished, duplicate gestures may be explicitly consumed without claiming delivery.
    fn skip_modal_input(&mut self, _event: Event) -> bool {
        false
    }
    /// Composite backends may declare independent finite send budgets; ordinary messages/remote paths retain original budgets.
    fn send_timeout_ms(&self, event: Event) -> u32 {
        event.message_timeout_ms()
    }
    fn preflight(&mut self) -> Result<(), Self::Error>;
    /// Clean the last hover target without new hit tests or system-pointer movement.
    fn leave(&mut self) -> Cleanup {
        Cleanup::default()
    }
    /// preferred is an existing gesture/key target; invalidation permits only validated root fallback.
    fn resolve(
        &mut self,
        event: Event,
        preferred: Option<Self::Target>,
    ) -> Result<Self::Target, Self::Error>;
    /// New keys/text may follow source focus; fallback is the previous keyboard target, not a held target awaiting release.
    fn resolve_keyboard(
        &mut self,
        event: Event,
        fallback: Option<Self::Target>,
    ) -> Result<Self::Target, Self::Error> {
        self.resolve(event, fallback)
    }
    /// timeout_ms bounds sending. Default completion follows API status; modal admission must use delivery
    /// to explicitly mark Pending, never infer completion from message return values or disguise admission as completion.
    fn send(
        &mut self,
        target: Self::Target,
        event: Event,
        buttons: u8,
        timeout_ms: u32,
    ) -> Result<(), Self::Error>;
    /// Cleanup must not trigger click compensation. Invalid source identity/permissions return None, never sending to reused handles.
    fn cleanup_target(&mut self, target: Self::Target) -> Option<Self::Target>;
    /// Separate from ordinary send: releases only, clearing deferred click actions, with a 50 ms limit.
    fn release(
        &mut self,
        target: Self::Target,
        event: Event,
        buttons: u8,
    ) -> Result<(), Self::Error>;
}
pub struct Controller<B: Backend> {
    backend: B,
    enabled: bool,
    sequence: u64,
    mouse: Option<B::Target>,
    keyboard: Option<B::Target>,
    buttons: u8,
    point: [f64; 2],
    keys: [Option<(u16, B::Target)>; MAX_KEYS],
    cleanup: Cleanup,
}
impl<B: Backend> Controller<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            enabled: false,
            sequence: 0,
            mouse: None,
            keyboard: None,
            buttons: 0,
            point: [0.0; 2],
            keys: std::array::from_fn(|_| None),
            cleanup: Cleanup::default(),
        }
    }
    /// Explicit control requests only; never auto-enable during capture startup/ordinary drawing.
    pub fn enable(&mut self) -> Result<(), Error<B::Error>> {
        if let Err(error) = self.backend.preflight() {
            self.cancel();
            return Err(Error::Backend(error));
        }
        self.enabled = true;
        Ok(())
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn last_sequence(&self) -> u64 {
        self.sequence
    }
    pub fn last_cleanup(&self) -> Cleanup {
        self.cleanup
    }
    pub fn dispatch(&mut self, sequence: u64, event: Event) -> Result<(), Error<B::Error>> {
        self.dispatch_status(sequence, event).map(|_| ())
    }
    pub fn idle(&mut self) -> Result<(), Error<B::Error>> {
        self.backend.idle().map_err(|error| {
            self.cancel();
            Error::Backend(error)
        })
    }
    pub fn pending_delivery(&mut self) -> bool {
        self.backend.pending_delivery()
    }
    pub fn dispatch_status(
        &mut self,
        sequence: u64,
        event: Event,
    ) -> Result<Delivery, Error<B::Error>> {
        let result = self.dispatch_inner(sequence, event);
        // Match mainline dispatcher: any input error revokes control, without replaying failed edges.
        if result.is_err() {
            self.cancel();
        }
        result.map(|_| self.backend.delivery())
    }
    fn dispatch_inner(&mut self, sequence: u64, event: Event) -> Result<(), Error<B::Error>> {
        if !self.enabled {
            return Err(Error::Disabled);
        }
        if sequence == 0 || sequence > MAX_SEQUENCE || sequence <= self.sequence {
            return Err(Error::Sequence);
        }
        self.backend.preflight().map_err(Error::Backend)?;
        // Consume sequence after source validation but before conversion/sending; failure/timeout cannot resend the same edge.
        self.sequence = sequence;
        if !event.valid() {
            return Err(Error::InvalidEvent);
        }
        if self.backend.skip_modal_input(event) {
            return Ok(());
        }
        let key_slot = match event {
            Event::Key { code, down } => {
                let existing = self
                    .keys
                    .iter()
                    .position(|held| held.as_ref().is_some_and(|(key, _)| *key == code));
                if down {
                    Some(
                        existing
                            .or_else(|| self.keys.iter().position(Option::is_none))
                            .ok_or(Error::KeyLimit)?,
                    )
                } else {
                    existing
                }
            }
            _ => None,
        };
        // Register potentially pressed state first so delivery failure still releases; definite nondelivery restores original held/keyboard targets.
        // Do not simply clear slots: a real earlier press may still await up before repeated down.
        let key_before =
            key_slot.map(|slot| (slot, self.keys[slot].clone(), self.keyboard.clone()));
        let preferred = match event {
            Event::Character { .. } => self.keyboard.clone(),
            Event::Key { .. } => key_slot
                .and_then(|slot| self.keys[slot].as_ref().map(|(_, target)| target.clone()))
                .or_else(|| self.keyboard.clone()),
            _ if self.buttons != 0 => self.mouse.clone(),
            _ => None,
        };
        let fresh_keyboard = matches!(event, Event::Character { .. })
            || matches!(event, Event::Key { .. })
                && key_slot.is_none_or(|slot| self.keys[slot].is_none());
        let target = if fresh_keyboard {
            self.backend.resolve_keyboard(event, preferred)
        } else {
            self.backend.resolve(event, preferred)
        }
        .map_err(Error::Backend)?;
        if let Some(point) = event.point() {
            self.point = point;
        }
        let mut next_buttons = self.buttons;
        match event {
            Event::Button { button, down, .. } => {
                if down {
                    next_buttons |= button.mask();
                    self.mouse = Some(target.clone());
                    self.keyboard = Some(target.clone());
                    // Record potentially pressed state first; send timeout still requires attempted release.
                    self.buttons = next_buttons;
                } else {
                    next_buttons &= !button.mask();
                }
            }
            Event::Key { code, down: true } => {
                self.keys[key_slot.unwrap()] = Some((code, target.clone()));
                self.keyboard = Some(target.clone());
            }
            _ => {}
        }
        let timeout_ms = self.backend.send_timeout_ms(event);
        self.backend
            .send(target, event, next_buttons, timeout_ms)
            .map_err(Error::Backend)?;
        if matches!(event, Event::Key { .. })
            && self.backend.delivery() == Delivery::IgnoredWhileModal
        {
            if let Some((slot, held, keyboard)) = key_before {
                self.keys[slot] = held;
                self.keyboard = keyboard;
            }
            return Ok(());
        }
        match event {
            Event::Button { .. } => {
                self.buttons = next_buttons;
                if self.buttons == 0 {
                    self.mouse = None;
                }
            }
            Event::Key { down: false, .. } => {
                if let Some(slot) = key_slot {
                    self.keys[slot] = None;
                }
            }
            _ => {}
        }
        Ok(())
    }
    /// Focus/capture loss, revocation, and close all use this entry; at most 35 key/button releases plus one hover exit, without unbounded retry queues.
    /// Preserve explicit failure/skip counts; clearing local held state does not prove OS-confirmed release.
    pub fn cancel(&mut self) -> Cleanup {
        self.enabled = false;
        let mut report = Cleanup::default();
        for button in [Button::Left, Button::Right, Button::Middle] {
            if self.buttons & button.mask() == 0 {
                continue;
            }
            self.buttons &= !button.mask();
            if let Some(target) = self.mouse.clone() {
                self.release(
                    target,
                    Event::Button {
                        point: self.point,
                        button,
                        down: false,
                        double: false,
                    },
                    &mut report,
                );
            } else {
                report.skipped += 1;
            }
        }
        for slot in 0..MAX_KEYS {
            if let Some((code, target)) = self.keys[slot].take() {
                self.release(target, Event::Key { code, down: false }, &mut report);
            }
        }
        let hover = self.backend.leave();
        report.released += hover.released;
        report.failed += hover.failed;
        report.skipped += hover.skipped;
        self.mouse = None;
        self.keyboard = None;
        self.cleanup = report;
        report
    }
    fn release(&mut self, target: B::Target, event: Event, report: &mut Cleanup) {
        let Some(target) = self.backend.cleanup_target(target) else {
            report.skipped += 1;
            return;
        };
        if self.backend.release(target, event, self.buttons).is_ok() {
            report.released += 1;
        } else {
            report.failed += 1;
        }
    }
}
#[cfg(test)]
#[path = "live_input_keyboard_focus_tests.rs"]
mod keyboard_focus_tests;
impl<B: Backend> Drop for Controller<B> {
    fn drop(&mut self) {
        self.cancel();
    }
}
#[cfg(test)]
#[path = "live_input_ignored_key_tests.rs"]
mod ignored_key_tests;
#[cfg(test)]
#[path = "live_input_tests.rs"]
mod tests;
