//! UI edit-intent FIFO; coalesce only same-field tail input, reserving in-flight slots for return when not started.
use crate::surface_input_event::Phase;
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;
use std::time::{Duration, Instant};
#[cfg(test)]
#[path = "surface_edit_keyboard_queue_tests.rs"]
mod keyboard_tests;
#[path = "surface_edit_pointer_queue.rs"]
mod pointer;
pub(super) use pointer::PointerIntent;
#[path = "surface_edit_hover_queue.rs"]
mod hover;
pub(super) use hover::HoverIntent;
#[derive(Clone)]
pub struct Fence {
    scope: Rc<()>,
    sequence: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FenceState {
    Pending,
    InFlight,
    Ready,
    Invalid,
}
const LIFETIME: Duration = Duration::from_secs(30);
#[derive(Clone)]
pub struct Intent {
    pub node: String,
    pub edge: Edge,
    pub value: String,
    expires: Instant,
    sequence: u64,
    scope: Rc<()>,
}
#[derive(Clone)]
pub enum Edge {
    Text(Phase),
    Pointer(PointerIntent),
    Hover(HoverIntent),
    Keyboard {
        flags: u32,
        focus_epoch: u64,
        change_epoch: u64,
    },
}
impl Intent {
    pub(super) fn local_id(&self) -> String {
        let kind = match self.edge {
            Edge::Keyboard { .. } => "keyboard",
            Edge::Pointer(_) => "pointer",
            Edge::Hover(_) => "hover",
            Edge::Text(_) => "input",
        };
        format!("javascript-{kind}:{}", self.sequence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn surface_number_step_admission_reserves_both_edges_and_sequence_space() {
        let mut queue = Queue::default();
        for _ in 0..31 {
            assert!(queue.push("a", Phase::Change, "1"));
        }
        assert!(!queue.can_push_step("a"));
        queue.prepared(None);
        assert!(queue.can_push_step("a"));
        queue.sequence = u64::MAX - 1;
        assert!(!queue.can_push_step("a"));
        queue.sequence = 0;
        assert!(queue.push("a", Phase::Input, "2"));
        assert!(queue.can_push_step("a"));
        queue.seal();
        assert!(!queue.can_push_step("a"));
    }
    #[test]
    fn surface_edit_dependency_fence_waits_for_original_prefix_and_invalidates_on_failure() {
        let mut queue = Queue::default();
        queue.push("a", Phase::Input, "before");
        let fence = queue.seal();
        queue.push("a", Phase::Input, "after");
        assert_eq!(queue.state(&fence), FenceState::Pending);
        queue.prepared(Some("first"));
        assert_eq!(queue.state(&fence), FenceState::InFlight);
        queue.cancel("first");
        assert_eq!(queue.state(&fence), FenceState::Pending);
        queue.prepared(Some("retry"));
        queue.settle("retry");
        assert_eq!(queue.state(&fence), FenceState::Ready);
        let newer = queue.seal();
        queue.prepared(Some("later"));
        queue.failed("later");
        assert_eq!(queue.state(&newer), FenceState::Invalid);
    }
    #[test]
    fn surface_edit_deadline_expiry_drops_dependents_not_prepared_evidence() {
        let start = Instant::now();
        let mut queue = Queue::default();
        queue.push_at("a", Phase::Input, "in flight", start);
        queue.prepared(Some("event"));
        queue.push_at("a", Phase::Change, "old", start);
        queue.push_at("b", Phase::Input, "later", start + Duration::from_secs(10));
        assert!(!queue.expire(start + LIFETIME - Duration::from_nanos(1)));
        assert!(queue.expire(start + LIFETIME));
        assert!(queue.front().is_none());
        assert_eq!(queue.cancel("event").as_deref(), Some("a"));
        assert!(queue.expire(start + LIFETIME));
    }
    #[test]
    fn surface_edit_deadline_only_fresh_continuous_input_renews_tail() {
        let start = Instant::now();
        let mut queue = Queue::default();
        queue.push_at("a", Phase::Input, "old", start);
        queue.push_at("a", Phase::Input, "fresh", start + Duration::from_secs(20));
        assert!(!queue.expire(start + LIFETIME));
        queue.prepared(Some("event"));
        queue.cancel("event");
        assert!(queue.expire(start + Duration::from_secs(50)));
    }
    #[test]
    fn surface_edit_order_capacity_reserves_room_for_cancel_and_only_coalesces_tail() {
        let mut queue = Queue::default();
        for n in 0..32 {
            assert!(queue.push("a", Phase::Change, &n.to_string()));
        }
        queue.prepared(Some("event"));
        assert!(!queue.push("b", Phase::Input, "overflow"));
        assert_eq!(queue.cancel("event").as_deref(), Some("a"));
        assert_eq!(queue.front().unwrap().value, "0");
        queue.prepared(Some("done"));
        queue.settle("done");
        assert!(queue.push("b", Phase::Input, "first"));
        assert!(queue.push("b", Phase::Input, "latest"));
        assert!(!queue.push("a", Phase::Input, "cannot cross field"));
        assert!(!queue.push("b", Phase::Change, "cannot cross phase"));
    }
}
#[derive(Default)]
pub struct Queue {
    waiting: VecDeque<Intent>,
    prepared: BTreeMap<String, Intent>,
    sealed_tail: bool,
    scope: Rc<()>,
    sequence: u64,
    // Reserve only terminal slots/sequences, not another Pointer queue; bool indicates down was delivered for preparation.
    pointer_reservation: Option<(u64, bool)>,
    pointer_delivered: Option<PointerIntent>,
    hover_reserved: bool,
    hover_delivered: Option<HoverIntent>,
}
impl Queue {
    pub fn can_push_change(&self) -> bool {
        self.room(1)
    }
    /// While the UI owner is held there is no second writer; admit both step edges before modifying HWND.
    pub fn can_push_step(&self, node: &str) -> bool {
        let coalesces = !self.sealed_tail
            && self.waiting.back().is_some_and(|last| {
                last.node == node && matches!(last.edge, Edge::Text(Phase::Input))
            });
        let slots = if coalesces { 1 } else { 2 };
        self.room(slots)
    }
    pub fn push(&mut self, node: &str, phase: Phase, value: &str) -> bool {
        self.push_at(node, phase, value, Instant::now())
    }
    fn push_at(&mut self, node: &str, phase: Phase, value: &str, now: Instant) -> bool {
        self.push_edge(node, Edge::Text(phase), value, now)
    }
    pub(super) fn push_keyboard(
        &mut self,
        node: &str,
        value: &str,
        flags: u32,
        focus_epoch: u64,
        change_epoch: u64,
    ) -> bool {
        self.push_edge(
            node,
            Edge::Keyboard {
                flags,
                focus_epoch,
                change_epoch,
            },
            value,
            Instant::now(),
        )
    }
    fn push_edge(&mut self, node: &str, edge: Edge, value: &str, now: Instant) -> bool {
        if !self.sealed_tail && matches!(edge, Edge::Text(Phase::Input)) {
            if let Some(last) = self
                .waiting
                .back_mut()
                .filter(|last| last.node == node && matches!(last.edge, Edge::Text(Phase::Input)))
            {
                last.value = value.to_owned();
                last.expires = now + LIFETIME;
                return true;
            }
        }
        if !self.room(1) {
            return false;
        }
        let Some(sequence) = self.sequence.checked_add(1) else {
            return false;
        };
        self.sequence = sequence;
        self.waiting.push_back(Intent {
            node: node.to_owned(),
            edge,
            value: value.to_owned(),
            expires: now + LIFETIME,
            sequence,
            scope: self.scope.clone(),
        });
        self.sealed_tail = false;
        true
    }
    /// Discrete activation creates a coalescing boundary; subsequent input cannot overwrite undispatched pre-click values.
    pub fn seal(&mut self) -> Fence {
        self.sealed_tail = true;
        Fence {
            scope: self.scope.clone(),
            sequence: self.sequence,
        }
    }
    pub fn state(&self, fence: &Fence) -> FenceState {
        if !Rc::ptr_eq(&self.scope, &fence.scope) {
            return FenceState::Invalid;
        }
        if self
            .prepared
            .values()
            .any(|intent| intent.sequence <= fence.sequence)
        {
            return FenceState::InFlight;
        }
        if self
            .waiting
            .iter()
            .any(|intent| intent.sequence <= fence.sequence)
        {
            FenceState::Pending
        } else {
            FenceState::Ready
        }
    }
    pub fn front(&self) -> Option<&Intent> {
        self.waiting.front()
    }
    pub(super) fn prepared_node(&self, id: &str) -> Option<&str> {
        self.prepared.get(id).map(|intent| intent.node.as_str())
    }
    pub(super) fn prepared_intent(&self, id: &str) -> Option<&Intent> {
        self.prepared.get(id)
    }
    pub(super) fn default_current(&self, intent: &Intent) -> bool {
        Rc::ptr_eq(&intent.scope, &self.scope) && Instant::now() < intent.expires
    }
    /// Default actions replace the original in-flight slot/sequence rather than appending behind later keyboard events, click fences, or text.
    pub(super) fn complete_local(&mut self, id: &str, change: Option<&str>) {
        if let Some(mut intent) = self.prepared.remove(id) {
            if let Some(value) = change {
                intent.edge = Edge::Text(Phase::Change);
                intent.value = value.to_owned();
                self.waiting.push_front(intent);
            }
        }
    }
    pub fn has_node(&self, id: &str) -> bool {
        self.waiting.iter().any(|i| i.node == id)
    }
    pub fn prepared(&mut self, event: Option<&str>) {
        if let Some(intent) = self.waiting.pop_front() {
            if let Some(event) = event {
                if let Edge::Pointer(pointer) = &intent.edge {
                    self.pointer_delivered = (!pointer.input.terminal()).then(|| pointer.clone());
                    if let Some((gesture, started)) = self.pointer_reservation.as_mut() {
                        if *gesture == pointer.gesture {
                            *started = true;
                        }
                    }
                }
                if let Edge::Hover(hover) = &intent.edge {
                    self.hover_delivered = hover.input.target.as_ref().map(|_| hover.clone());
                }
                self.prepared.insert(event.to_owned(), intent);
            }
        }
    }
    pub fn cancel(&mut self, event: &str) -> Option<String> {
        let intent = self.prepared.remove(event)?;
        let node = intent.node.clone();
        self.waiting.push_front(intent);
        Some(node)
    }
    pub fn settle(&mut self, event: &str) {
        self.prepared.remove(event);
    }
    /// Only prerequisite events registered by the current session may truncate dependency chains; old-session receipts cannot clear new input.
    pub fn failed(&mut self, event: &str) -> Option<String> {
        let intent = self.prepared.remove(event)?;
        self.waiting.clear();
        self.pointer_reservation = None;
        self.pointer_delivered = None;
        self.hover_reserved = false;
        self.hover_delivered = None;
        self.scope = Rc::new(());
        Some(intent.node)
    }
    pub fn retain(&mut self, mut keep: impl FnMut(&str) -> bool) {
        let before = self.waiting.len() + self.prepared.len();
        // Canvas is not among fields; Pointer preparation rechecks its target/epoch, and cancel may reference detached nodes.
        self.waiting
            .retain(|i| matches!(i.edge, Edge::Pointer(_) | Edge::Hover(_)) || keep(&i.node));
        self.prepared
            .retain(|_, i| matches!(i.edge, Edge::Pointer(_) | Edge::Hover(_)) || keep(&i.node));
        if before != self.waiting.len() + self.prepared.len() {
            self.scope = Rc::new(());
        }
    }
    pub fn clear_waiting(&mut self) {
        self.waiting.clear();
        self.pointer_reservation = None;
        self.pointer_delivered = None;
        self.hover_reserved = false;
        self.hover_delivered = None;
        self.scope = Rc::new(());
    }
    /// Expiry of unsent prerequisite intents truncates the whole dependency chain; local queue deadlines do not rewrite in-flight evidence.
    pub fn expire(&mut self, now: Instant) -> bool {
        if self.waiting.iter().any(|intent| now >= intent.expires) {
            self.clear_waiting();
            true
        } else {
            false
        }
    }
}
