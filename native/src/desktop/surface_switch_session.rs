//! 无 HWND 的布尔字段会话；复用既有 Draft ticket 规则，字符串只用于内部 FIFO。
use crate::{
    surface_click_event::ClickEvent,
    surface_input_draft::{Draft, Ticket},
    surface_input_event::{self, Phase, Value},
    surface_live_binding::Binding,
    surface_scene::Node,
};
use std::collections::BTreeMap;
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_FAIL,
};

pub(super) struct Session {
    pub contract: [u8; 32],
    pub disabled: bool,
    draft: Draft,
    pending: BTreeMap<String, Ticket>,
    unsent: Option<Ticket>,
    change: Option<(bool, bool)>,
}
fn failure() -> Error {
    Error::new(E_FAIL, "Surface switch state unavailable")
}
fn text(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}
impl Session {
    pub fn new(node: &Node, contract: [u8; 32]) -> Result<Self> {
        Ok(Self {
            contract,
            disabled: false,
            draft: Draft::new(text(crate::surface_switch::checked(node))).map_err(|_| failure())?,
            pending: BTreeMap::new(),
            unsent: None,
            change: None,
        })
    }
    pub fn value(&self) -> bool {
        self.draft.value() == "true"
    }
    pub fn snapshot(&self) -> &'static str {
        text(self.value())
    }
    pub fn protect(&mut self) -> Result<()> {
        if self.unsent.is_none() {
            self.unsent = Some(self.draft.begin().map_err(|_| failure())?);
        }
        Ok(())
    }
    pub fn toggle(&mut self) -> Result<()> {
        let before = self.value();
        self.protect()?;
        self.draft.edit(text(!before)).map_err(|_| failure())?;
        self.record_change(before);
        Ok(())
    }
    pub fn synchronize(&mut self, node: &Node) -> Result<()> {
        let before = self.value();
        self.disabled = node.props().get("disabled") == Some(&serde_json::Value::Bool(true));
        self.draft
            .synchronize(text(crate::surface_switch::checked(node)))
            .map_err(|_| failure())?;
        self.record_change(before);
        Ok(())
    }
    pub fn prepare(
        &mut self,
        node: &Node,
        binding: &Binding,
        revision: u64,
        snapshot: &str,
    ) -> Result<Option<ClickEvent>> {
        let value = match snapshot {
            "true" => true,
            "false" => false,
            _ => return Err(failure()),
        };
        if self.disabled || self.pending.len() >= crate::surface_input_draft::MAX_PENDING - 1 {
            return Err(failure());
        }
        let event = surface_input_event::encode(
            node,
            binding,
            revision,
            Phase::Change,
            Value::Boolean(value),
        )
        .map_err(|_| failure())?;
        let Some(event) = event else {
            // 无 change handler 时保留本地值，但允许后续不同权威值正常同步。
            if self.pending.is_empty() {
                if let Some(ticket) = self.unsent.take() {
                    self.draft.cancel(ticket);
                }
            }
            return Ok(None);
        };
        if self.pending.contains_key(event.id()) {
            return Err(failure());
        }
        let ticket = if snapshot != self.draft.value() {
            self.draft.begin().map_err(|_| failure())?
        } else {
            self.protect()?;
            self.unsent.take().unwrap()
        };
        self.pending.insert(event.id().to_owned(), ticket);
        Ok(Some(event))
    }
    pub fn cancel(&mut self, id: &str) {
        if let Some(ticket) = self.pending.remove(id) {
            if self.unsent.is_none() {
                self.unsent = Some(ticket);
            } else {
                self.draft.cancel(ticket);
            }
        }
    }
    pub fn settle(&mut self, id: &str) {
        let before = self.value();
        if let Some(ticket) = self.pending.remove(id) {
            self.draft.settle(ticket);
        }
        self.record_change(before);
    }
    fn record_change(&mut self, before: bool) {
        let after = self.value();
        if before != after {
            let first = self.change.map_or(before, |change| change.0);
            self.change = (first != after).then_some((first, after));
        }
    }
    pub fn take_change(&mut self) -> Option<(bool, bool)> {
        self.change.take()
    }
}
