//! Hover shares the original FIFO; same-target tail moves coalesce, and leave slots/sequences count with Canvas terminals toward the 32-slot limit.
use super::*;
use crate::surface_javascript_wire::HoverInput;
#[derive(Clone)]
pub(crate) struct HoverIntent {
    pub input: HoverInput,
    pub viewport_epoch: u64,
}
impl Queue {
    pub(in crate::desktop::surface_edit_mount) fn has_hovers(&self) -> bool {
        self.hover_reserved
            || self.hover_delivered.is_some()
            || self
                .waiting
                .iter()
                .chain(self.prepared.values())
                .any(|i| matches!(i.edge, Edge::Hover(_)))
    }
    pub(in crate::desktop::surface_edit_mount) fn push_hover(
        &mut self,
        hover: HoverIntent,
    ) -> bool {
        let entering = hover.input.target.is_some() && !self.hover_reserved;
        if entering && !self.room(2) {
            return false;
        }
        if hover.input.target.is_none() && !self.hover_reserved {
            return true;
        }
        if !self.sealed_tail {
            if let Some(last) = self.waiting.back_mut() {
                if matches!(&last.edge, Edge::Hover(old) if old.input.target == hover.input.target
                    && old.viewport_epoch == hover.viewport_epoch)
                {
                    last.edge = Edge::Hover(hover);
                    last.expires = Instant::now() + LIFETIME;
                    return true;
                }
            }
        }
        if entering {
            self.hover_reserved = true;
        }
        if hover.input.target.is_none() {
            self.hover_reserved = false;
        }
        let node = hover.input.target.clone().unwrap_or_default();
        let accepted = self.push_edge(&node, Edge::Hover(hover), "", Instant::now());
        if entering && !accepted {
            self.hover_reserved = false;
        }
        accepted
    }
    pub(in crate::desktop::surface_edit_mount) fn cancel_hovers(&mut self, epoch: u64) -> bool {
        let mut leave = self.hover_delivered.take().map(|mut hover| {
            hover.input.target = None;
            hover.viewport_epoch = epoch;
            hover
        });
        let mut changed = false;
        let mut retained_leave = false;
        let mut removed_enter = false;
        self.waiting.retain_mut(|intent| {
            let Edge::Hover(hover) = &mut intent.edge else {
                return true;
            };
            if hover.input.target.is_none() && leave.is_none() && !retained_leave && !removed_enter
            {
                hover.viewport_epoch = epoch;
                retained_leave = true;
                return true;
            }
            removed_enter |= hover.input.target.is_some();
            changed = true;
            if let Some(replacement) = leave.take() {
                intent.node.clear();
                *hover = replacement;
                retained_leave = true;
                true
            } else {
                false
            }
        });
        if changed {
            self.scope = Rc::new(());
        }
        self.hover_reserved = false;
        let Some(hover) = leave else {
            return true;
        };
        self.push_edge("", Edge::Hover(hover), "", Instant::now())
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "surface_edit_hover_queue_tests.rs"]
mod tests;
