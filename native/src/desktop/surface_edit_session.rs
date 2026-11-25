//! Single-field edit sessions: EDIT, drafts, and event tickets share one UI owner; no network requests.
use super::surface_edit_control::{Control, MAX_UNITS};
use crate::{
    surface_click_event::ClickEvent,
    surface_input_draft::{Draft, Ticket, MAX_PENDING},
    surface_input_event::{self, Phase, Value},
    surface_live_binding::Binding,
    surface_scene::{Kind, Node},
};
use std::collections::BTreeMap;
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_FAIL, E_INVALIDARG, HWND, RECT},
};

pub struct Session {
    control: Control,
    binding: Option<Binding>,
    node: String,
    kind: Kind,
    number: Option<crate::surface_number::Limits>,
    number_selection: Option<(u32, u32)>,
    user_limit: usize,
    draft: Draft,
    pending: BTreeMap<String, Ticket>,
    unsent: Option<Ticket>,
    composing: bool,
    composition_reserved: bool,
    composition_reconcile: bool,
}
fn invalid(message: &str) -> Error {
    Error::new(E_INVALIDARG, message)
}
fn enabled(node: &Node) -> bool {
    node.visible() && node.props().get("disabled") != Some(&serde_json::Value::Bool(true))
}
fn source(node: &Node) -> &str {
    let value = node
        .props()
        .get("value")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if node.kind() == Kind::Number {
        crate::surface_number::sanitized(value)
    } else {
        value
    }
}
fn user_limit(node: &Node) -> usize {
    if node.kind() != Kind::Textarea {
        return MAX_UNITS;
    }
    node.props()
        .get("maxLength")
        .and_then(|value| value.as_f64())
        .unwrap_or(MAX_UNITS as f64)
        .clamp(1., MAX_UNITS as f64)
        .trunc() as usize
}
fn normalized(kind: Kind, value: &str) -> std::borrow::Cow<'_, str> {
    if kind == Kind::Textarea && value.contains('\r') {
        std::borrow::Cow::Owned(value.replace("\r\n", "\n").replace('\r', "\n"))
    } else {
        std::borrow::Cow::Borrowed(value)
    }
}
fn native_text(kind: Kind, value: &str) -> std::borrow::Cow<'_, str> {
    if kind == Kind::Textarea && value.contains('\n') {
        std::borrow::Cow::Owned(value.replace('\n', "\r\n"))
    } else {
        std::borrow::Cow::Borrowed(value)
    }
}
impl Session {
    /// Create hidden. The outer layer must still validate ancestor visibility/input eligibility and show only after notification registration.
    pub fn new(parent: HWND, node: &Node, binding: &Binding, rect: RECT) -> Result<Self> {
        Self::create(parent, node, Some(binding.clone()), rect)
    }
    /// Reuse only local EDIT/drafts/IME without ordinary Surface mounting; callers separately hold actual-host eligibility.
    pub(crate) fn unbound(parent: HWND, node: &Node, rect: RECT) -> Result<Self> {
        Self::create(parent, node, None, rect)
    }
    fn create(parent: HWND, node: &Node, binding: Option<Binding>, rect: RECT) -> Result<Self> {
        if !matches!(node.kind(), Kind::Input | Kind::Textarea | Kind::Number) {
            return Err(invalid("Unsupported or disabled edit node"));
        }
        let control = Control::new(parent, rect, node.kind() == Kind::Textarea, MAX_UNITS)?;
        control.install_placeholder(
            node.props()
                .get("placeholder")
                .and_then(|value| value.as_str())
                .unwrap_or(""),
        )?;
        let value = normalized(node.kind(), source(node));
        control.set_text(&native_text(node.kind(), &value))?;
        let draft = Draft::new(&value).map_err(|_| invalid("Invalid initial draft"))?;
        Ok(Self {
            control,
            binding,
            node: node.id().to_owned(),
            kind: node.kind(),
            number: (node.kind() == Kind::Number)
                .then(|| crate::surface_number::Limits::from_node(node)),
            number_selection: None,
            user_limit: user_limit(node),
            draft,
            pending: BTreeMap::new(),
            unsent: None,
            composing: false,
            composition_reserved: false,
            composition_reconcile: false,
        })
    }
    pub fn control(&self) -> &Control {
        &self.control
    }
    pub fn value(&self) -> &str {
        self.draft.value()
    }
    pub(crate) fn authoritative_value(&self) -> &str {
        self.draft.source()
    }
    pub fn form_value(&self) -> &str {
        if self.number.is_some() {
            crate::surface_number::sanitized(self.value())
        } else {
            self.value()
        }
    }
    pub fn step_number(&mut self, up: bool) -> Result<bool> {
        let Some(limits) = self.number.filter(|_| !self.composing) else {
            return Ok(false);
        };
        let text = self.read_candidate()?;
        let Some(next) = limits.step(&text, up) else {
            return Ok(false);
        };
        self.reserve_local()?;
        self.control.set_text(&next)?;
        self.draft
            .edit(&next)
            .map_err(|_| invalid("Invalid stepped number"))?;
        Ok(true)
    }
    pub fn pending(&self) -> usize {
        self.pending.len()
    }
    pub fn has_unsent(&self) -> bool {
        self.unsent.is_some()
    }
    fn matches(&self, node: &Node, binding: &Binding) -> Result<()> {
        self.matches_node(node)?;
        if self.binding.as_ref() != Some(binding) {
            return Err(invalid("Edit scope changed"));
        }
        Ok(())
    }
    fn matches_node(&self, node: &Node) -> Result<()> {
        self.control.hwnd()?;
        if self.node != node.id() || self.kind != node.kind() {
            return Err(invalid("Edit scope changed"));
        }
        Ok(())
    }
    fn reserve_local(&mut self) -> Result<()> {
        if self.unsent.is_none() {
            self.unsent = Some(
                self.draft
                    .begin()
                    .map_err(|_| Error::new(E_FAIL, "Edit ticket capacity exhausted"))?,
            );
        }
        Ok(())
    }
    /// Input notifications update local values only; busy background work must not create unbounded action queues.
    pub fn capture(&mut self) -> Result<bool> {
        if self.composing {
            return Ok(false);
        }
        let text = self.read_candidate()?;
        if text == self.draft.value() {
            return Ok(false);
        }
        self.reserve_local()?;
        self.draft
            .edit(&text)
            .map_err(|_| invalid("Invalid edit draft"))?;
        Ok(true)
    }
    pub fn begin_edit(&mut self) -> Result<()> {
        if self.number.is_some() && !self.composing {
            self.number_selection = Some(self.control.selection()?);
        }
        Ok(())
    }
    pub fn finish_edit(&mut self, notified: bool) -> Result<bool> {
        let result = if notified { self.capture() } else { Ok(false) };
        if !self.composing {
            self.number_selection = None;
        }
        result
    }
    fn read_candidate(&self) -> Result<String> {
        let candidate = self.control.text().and_then(|raw| {
            let text = match normalized(self.kind, &raw) {
                std::borrow::Cow::Borrowed(_) => raw,
                std::borrow::Cow::Owned(text) => text,
            };
            let units = text.encode_utf16().count();
            // HTML maxlength limits only user edits; longer existing authoritative values may still be shortened incrementally.
            if units > self.user_limit && units > self.draft.value().encode_utf16().count() {
                return Err(invalid("Surface edit exceeds maxLength"));
            }
            if self.number.is_some() && !crate::surface_number::editable(&text) {
                // Ordinary invalid edits are not application errors. Atomically restore the previous draft and pre-edit selection without events.
                self.control.set_text(self.draft.value())?;
                if let Some(selection) = self.number_selection {
                    self.control.select(selection)?;
                }
                return Ok(self.draft.value().to_owned());
            }
            Ok(text)
        });
        if candidate.is_err() {
            // Restore visible values on rejection, preventing divergence from drafts or sending unaccepted text.
            self.control
                .set_text(&native_text(self.kind, self.draft.value()))?;
        }
        candidate
    }
    /// Prepare only after outer commit eligibility passes. Reserve one ticket for later local input/IME.
    #[cfg(test)]
    pub fn prepare(
        &mut self,
        node: &Node,
        binding: &Binding,
        revision: u64,
        phase: Phase,
    ) -> Result<Option<ClickEvent>> {
        self.prepare_value(node, binding, revision, phase, None)
    }
    pub fn protect_queued(&mut self) -> Result<()> {
        self.reserve_local()
    }
    /// Captured edge values do not reread HWND; newer drafts retain their own local tickets.
    pub fn prepare_value(
        &mut self,
        node: &Node,
        binding: &Binding,
        revision: u64,
        phase: Phase,
        snapshot: Option<&str>,
    ) -> Result<Option<ClickEvent>> {
        self.matches(node, binding)?;
        self.prepare_using(node, snapshot, |value| {
            surface_input_event::encode(node, binding, revision, phase, value)
                .map(|event| event.map(|e| (e.id().to_owned(), e)))
        })
    }
    /// Call only after outer hosts such as Wall verify identity; encode/track drafts here without granting mount permission.
    pub(crate) fn prepare_unbound(
        &mut self,
        node: &Node,
        identity: crate::surface_click_event::Identity<'_>,
        revision: u64,
        phase: Phase,
        snapshot: Option<&str>,
    ) -> Result<Option<(String, Vec<u8>)>> {
        self.matches_unbound(node)?;
        self.prepare_using(node, snapshot, |value| {
            surface_input_event::encode_identity(node, identity, revision, phase, value)
                .map(|event| event.map(|(id, bytes)| (id.clone(), (id, bytes))))
        })
    }
    fn prepare_using<R>(
        &mut self,
        node: &Node,
        snapshot: Option<&str>,
        encode: impl FnOnce(
            Value<'_>,
        )
            -> std::result::Result<Option<(String, R)>, surface_input_event::Error>,
    ) -> Result<Option<R>> {
        if !enabled(node) {
            return Err(invalid("Edit node disabled"));
        }
        self.user_limit = user_limit(node);
        if self.composing || self.pending.len() >= MAX_PENDING - 1 {
            return Err(Error::new(E_FAIL, "Edit submission deferred"));
        }
        if snapshot.is_none() {
            self.capture()?;
        }
        let event = encode(if self.number.is_some() {
            Value::Number(crate::surface_number::parse(
                snapshot.unwrap_or(self.draft.value()),
            ))
        } else {
            Value::Text(snapshot.unwrap_or(self.draft.value()))
        })
        .map_err(|_| invalid("Edit event encoding failed"))?;
        let Some((id, event)) = event else {
            // A phase without a handler must not release protection for newer input awaiting another phase's commit.
            if !node.events().contains_key("input")
                && !node.events().contains_key("change")
                && self.pending.is_empty()
            {
                if let Some(ticket) = self.unsent.take() {
                    self.draft.cancel(ticket);
                }
            }
            return Ok(None);
        };
        self.track_submission(&id, snapshot)?;
        Ok(Some(event))
    }
    /// Local listeners and remote commits share a ticket owner, but local input encodes no fake remote action.
    pub(crate) fn prepare_local(
        &mut self,
        node: &Node,
        binding: &Binding,
        id: &str,
        value: &str,
    ) -> Result<()> {
        self.matches(node, binding)?;
        if node.kind() != Kind::Input
            || !enabled(node)
            || self.composing
            || self.pending.len() >= MAX_PENDING - 1
        {
            return Err(invalid("Local text input inactive"));
        }
        self.track_submission(id, Some(value))
    }
    fn track_submission(&mut self, id: &str, snapshot: Option<&str>) -> Result<()> {
        if self.pending.contains_key(id) {
            return Err(Error::new(E_FAIL, "Duplicate edit event identity"));
        }
        let ticket = if snapshot.is_some_and(|value| value != self.draft.value()) {
            self.draft
                .begin()
                .map_err(|_| Error::new(E_FAIL, "Edit ticket capacity exhausted"))?
        } else {
            self.reserve_local()?;
            self.unsent.take().unwrap()
        };
        self.pending.insert(id.to_owned(), ticket);
        Ok(())
    }
    /// Restore unsent requests as pending local values; if newer input exists, release only the old reservation.
    pub fn cancel(&mut self, event_id: &str) -> bool {
        let Some(ticket) = self.pending.remove(event_id) else {
            return false;
        };
        if self.unsent.is_none() {
            self.unsent = Some(ticket);
        } else {
            self.draft.cancel(ticket);
        }
        true
    }
    /// The outer evidence gate still accepts new snapshots; different bindings/nodes require destroying the old session before creating another.
    pub fn synchronize(&mut self, node: &Node, binding: &Binding) -> Result<()> {
        self.matches(node, binding)?;
        self.synchronize_value(node)
    }
    pub(crate) fn synchronize_unbound(&mut self, node: &Node) -> Result<()> {
        self.matches_unbound(node)?;
        self.synchronize_value(node)
    }
    fn matches_unbound(&self, node: &Node) -> Result<()> {
        self.matches_node(node)?;
        if self.binding.is_some() {
            return Err(invalid("Bound edit requires its verified Surface scope"));
        }
        Ok(())
    }
    fn synchronize_value(&mut self, node: &Node) -> Result<()> {
        let value = normalized(node.kind(), source(node));
        self.control
            .validate_text(&native_text(self.kind, &value))?;
        self.composition_reconcile |= self.composing && value != self.draft.source();
        self.draft
            .synchronize(&value)
            .map_err(|_| invalid("Invalid authoritative edit value"))?;
        self.user_limit = user_limit(node);
        self.number =
            (node.kind() == Kind::Number).then(|| crate::surface_number::Limits::from_node(node));
        self.apply_value()
    }
    /// Accept only event IDs from outer-validated results; duplicate/foreign IDs do not change state.
    pub fn settle(&mut self, event_id: &str) -> Result<bool> {
        self.control.hwnd()?;
        let Some(ticket) = self.pending.remove(event_id) else {
            return Ok(false);
        };
        let settled = self.draft.settle(ticket);
        self.composition_reconcile |= self.composing && settled;
        self.apply_value()?;
        Ok(settled)
    }
    fn apply_value(&self) -> Result<()> {
        // Never interrupt composition with WM_SETTEXT or reset selection/undo state when text is unchanged.
        if !self.composing && normalized(self.kind, &self.control.text()?) != self.draft.value() {
            self.control
                .set_text(&native_text(self.kind, self.draft.value()))?;
        }
        Ok(())
    }
    pub fn begin_composition(&mut self) -> Result<()> {
        self.control.hwnd()?;
        if !self.composing {
            self.begin_edit()?;
            self.composition_reserved = self.unsent.is_none();
            self.reserve_local()?;
            self.composition_reconcile = false;
            self.composing = true;
        }
        Ok(())
    }
    pub fn composing(&self) -> bool {
        self.composing
    }
    pub fn end_composition(&mut self) -> Result<bool> {
        if !self.composing {
            return Ok(false);
        }
        let (text, failure) = match self.read_candidate() {
            Ok(text) => (text, None),
            Err(error) => (self.draft.value().to_owned(), Some(error)),
        };
        let changed = text != self.draft.value();
        if changed {
            self.draft
                .edit(&text)
                .map_err(|_| invalid("Invalid composed draft"))?;
        } else if self.composition_reserved {
            if let Some(ticket) = self.unsent.take() {
                // IME cancellation is not a remote result; retain existing local text without a new authoritative value/receipt.
                if self.composition_reconcile {
                    self.draft.settle(ticket);
                } else {
                    self.draft.cancel(ticket);
                }
            }
        }
        self.composing = false;
        self.number_selection = None;
        self.composition_reserved = false;
        self.composition_reconcile = false;
        self.apply_value()?;
        if let Some(error) = failure {
            return Err(error);
        }
        Ok(changed)
    }
}
#[cfg(test)]
#[path = "surface_edit_session_tests.rs"]
mod tests;
