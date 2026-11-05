//! 原编辑 FIFO 的 Canvas 边沿/容量接缝；move 只覆盖未封存且同手势的队尾。
use super::*;
use crate::surface_javascript_wire::{PointerInput, PointerPhase};
#[derive(Clone)]
pub(crate) struct PointerIntent {
    pub input: PointerInput,
    pub gesture: u64,
    pub viewport_epoch: u64,
}
impl Queue {
    pub(in crate::desktop::surface_edit_mount) fn cancel_pointers(&mut self, epoch: u64) -> bool {
        let mut cancel = self.pointer_delivered.take().map(|mut pointer| {
            pointer.input.phase = PointerPhase::Pointercancel;
            pointer.input.click = false;
            pointer.viewport_epoch = epoch;
            pointer
        });
        let mut changed = false;
        self.waiting.retain_mut(|intent| {
            let Edge::Pointer(pointer) = &mut intent.edge else {
                return true;
            };
            if pointer.input.phase == PointerPhase::Pointercancel {
                pointer.viewport_epoch = epoch;
                return true;
            }
            changed = true;
            // 已排 up/move 的槽和序号可改成终止通知；MAX 序号时也不丢 terminal，且不越过后续输入。
            if let Some(replacement) = cancel.take() {
                intent.node = replacement.input.target.clone();
                *pointer = replacement;
                true
            } else {
                false
            }
        });
        if changed {
            self.scope = Rc::new(());
        }
        self.pointer_reservation = None;
        let Some(pointer) = cancel else {
            return true;
        };
        let node = pointer.input.target.clone();
        self.push_edge(&node, Edge::Pointer(pointer), "", Instant::now())
    }
    pub(super) fn room(&self, slots: usize) -> bool {
        let slots = slots
            + usize::from(self.pointer_reservation.is_some())
            + usize::from(self.hover_reserved);
        self.waiting.len() + self.prepared.len() + slots <= 32
            && self.sequence.checked_add(slots as u64).is_some()
    }
    pub(in crate::desktop::surface_edit_mount) fn push_pointer(
        &mut self,
        pointer: PointerIntent,
    ) -> bool {
        let down = pointer.input.phase == PointerPhase::Pointerdown;
        let terminal = pointer.input.terminal();
        if down {
            if self.pointer_reservation.is_some() || !self.room(2) {
                return false;
            }
            self.pointer_reservation = Some((pointer.gesture, false));
        } else if self.pointer_reservation.map(|r| r.0) != Some(pointer.gesture) {
            return false;
        }
        if pointer.input.phase == PointerPhase::Pointermove && !self.sealed_tail {
            if let Some(last) = self.waiting.back_mut() {
                if matches!(&last.edge, Edge::Pointer(old) if old.gesture == pointer.gesture
                    && old.viewport_epoch == pointer.viewport_epoch && old.input.target == pointer.input.target
                    && old.input.phase == PointerPhase::Pointermove)
                {
                    last.edge = Edge::Pointer(pointer);
                    last.expires = Instant::now() + LIFETIME;
                    return true;
                }
            }
        }
        if terminal {
            self.pointer_reservation = None;
        }
        let node = pointer.input.target.clone();
        let accepted = self.push_edge(&node, Edge::Pointer(pointer), "", Instant::now());
        if down && !accepted {
            self.pointer_reservation = None;
        }
        accepted
    }
    pub(in crate::desktop::surface_edit_mount) fn cancel_pointer(
        &mut self,
        pointer: PointerIntent,
    ) -> bool {
        let Some((gesture, _)) = self.pointer_reservation else {
            // terminal 已排队后 reservation 已消费，重复单手势取消仍需更新其 epoch。
            for intent in &mut self.waiting {
                if let Edge::Pointer(existing) = &mut intent.edge {
                    if existing.gesture == pointer.gesture
                        && existing.input.phase == PointerPhase::Pointercancel
                    {
                        existing.viewport_epoch = pointer.viewport_epoch;
                        return true;
                    }
                }
            }
            if self
                .pointer_delivered
                .as_ref()
                .is_some_and(|p| p.gesture == pointer.gesture)
            {
                return self.cancel_pointers(pointer.viewport_epoch);
            }
            return true;
        };
        if gesture != pointer.gesture {
            return false;
        }
        self.cancel_pointers(pointer.viewport_epoch)
    }
}
#[cfg(test)]
#[path = "surface_edit_pointer_queue_tests.rs"]
mod tests;
