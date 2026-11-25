//! Window input-control state: queue admission, pause/revocation, and synchronous hover eligibility; no source-thread hook ownership.
use super::*;
impl Control {
    pub(in super::super) fn source_input_allowed(&self) -> bool {
        self.allowed
    }
    pub(in super::super) fn wheel_wanted(&self) -> bool {
        self.allowed && self.desired && !self.remote && !self.failed
    }
    pub(in super::super) fn cancel(&mut self) {
        self.desired = false;
        self.shown = None;
        self.pause();
    }
    pub(super) fn pause(&mut self) {
        self.request = Some(false);
        self.clear();
    }
    pub(super) fn clear(&mut self) {
        self.native_active(false);
        self.hover_active(false);
        self.tracking = pointer_tracking::Tracking::default();
        self.ime.clear();
        self.enabled = false;
        self.pending = false;
        self.buttons = 0;
        self.pointer = None;
        self.held_keys = [0; 4];
        self.events.clear();
        if let Some(stop) = self.stop.take() {
            stop.request();
        }
    }
    pub(super) fn push(&mut self, event: Event) -> bool {
        if let Event::Key { code, down: false } = event {
            if self.held_keys[usize::from(code) / 64] & (1u64 << (code % 64)) == 0 {
                // Handoff already enqueued up for the original target, or current B never admitted down; a late physical up must not
                // be rerouted using new focus. FIFO-queued releases and the worker's cancellation responsibility remain unchanged.
                return true;
            }
        }
        if !queue::push_native(&mut self.events, event, self.buttons) {
            eprintln!("native-live-input-pause: reason=queue-capacity");
            // Overload clears backlog only, preserving user control intent; never replay stale input.
            self.pause();
            return false;
        }
        if let Event::Key { code, down } = event {
            // The local decoder already restricts VK to 1..=254; record only UI admission state here,
            // while actual target binding and release responsibility remain with the worker's Controller.
            let word = &mut self.held_keys[usize::from(code) / 64];
            let mask = 1u64 << (code % 64);
            if down {
                *word |= mask;
            } else {
                *word &= !mask;
            }
        }
        match event {
            Event::Leave => self.hover_active(false),
            Event::Move { point }
            | Event::Hover { point }
            | Event::Button { point, .. }
            | Event::Wheel { point, .. } => {
                self.pointer = Some(point);
                self.hover_active(true);
            }
            _ => {}
        }
        true
    }
    pub(super) fn button_edge(&mut self, mask: u8, down: bool) -> bool {
        if down {
            self.buttons |= mask;
        } else {
            if self.buttons & mask == 0 {
                return false; // Focus handoff already enqueued release; a late physical up must not be delivered again.
            }
            self.buttons &= !mask;
        }
        true
    }
    pub(super) fn handoff_source(&mut self) -> bool {
        if self.buttons != 0 {
            let Some(point) = self.pointer else {
                return false; // Without admitted coordinates, do not invent release; the original Controller performs cancellation cleanup.
            };
            for (button, mask) in [
                (crate::live_input::Button::Left, 1),
                (crate::live_input::Button::Right, 2),
                (crate::live_input::Button::Middle, 4),
            ] {
                if self.buttons & mask != 0 {
                    if !self.push(Event::Button {
                        point,
                        button,
                        down: false,
                        double: false,
                    }) {
                        return false;
                    }
                    self.buttons &= !mask;
                }
            }
        }
        self.handoff_keys()
    }
    pub(super) fn handoff_keys(&mut self) -> bool {
        if self.buttons != 0 {
            return false; // Capture/drag loss still uses the original pause path; do not enter another mirror with held input.
        }
        // Queue after admitted down in the original FIFO. The original worker's held slot retains target/release responsibility;
        // do not stop the worker/revoke epoch, which would also cancel the source modal menu that remains open.
        let held = self.held_keys;
        for code in 1u16..=254 {
            if held[usize::from(code) / 64] & (1u64 << (code % 64)) != 0
                && !self.push(Event::Key { code, down: false })
            {
                return false; // The full-capacity push already revoked control; Controller still owns undelivered releases.
            }
        }
        true
    }
    pub(super) fn native_active(&self, active: bool) {
        if let Some(owner) = &self.hover {
            if let Err(error) = owner.control(active) {
                eprintln!("native-live-control-scope: {error}");
            }
        }
    }
    fn hover_active(&self, active: bool) {
        if let Some(owner) = &self.hover {
            if let Err(error) = owner.active(active) {
                eprintln!("native-live-hover-scope: {error}");
            }
        }
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "composition_live_input_key_tests.rs"]
mod key_tests;
impl LiveWindow {
    pub(crate) fn hover_identity(&self) -> Option<crate::live_hover_owner::Identity> {
        self.state
            .input
            .borrow()
            .hover
            .as_ref()
            .map(|owner| owner.identity())
    }
}
