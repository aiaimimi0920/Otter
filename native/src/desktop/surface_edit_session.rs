//! 单字段编辑会话：EDIT、草稿和事件 ticket 同一 UI owner；不发送网络请求。
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
    /// 隐藏创建。外层仍须验证祖先可见性/输入资格，并在注册通知后显示。
    pub fn new(parent: HWND, node: &Node, binding: &Binding, rect: RECT) -> Result<Self> {
        Self::create(parent, node, Some(binding.clone()), rect)
    }
    /// 仅复用本地EDIT/草稿/IME，不构造普通Surface挂载；调用方另持真实宿主资格。
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
    /// 输入通知只更新本地值；后台忙时也不创建无界动作队列。
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
            let text = normalized(self.kind, &raw).into_owned();
            let units = text.encode_utf16().count();
            // HTML maxlength 只限制用户编辑；已有更长权威值仍允许逐步缩短。
            if units > self.user_limit && units > self.draft.value().encode_utf16().count() {
                return Err(invalid("Surface edit exceeds maxLength"));
            }
            if self.number.is_some() && !crate::surface_number::editable(&text) {
                // 普通无效编辑不是应用错误。原子恢复上一个草稿和编辑前选区，不产生事件。
                self.control.set_text(self.draft.value())?;
                if let Some(selection) = self.number_selection {
                    self.control.select(selection)?;
                }
                return Ok(self.draft.value().to_owned());
            }
            Ok(text)
        });
        if candidate.is_err() {
            // 拒绝后恢复可见值，不让控件与草稿分叉或把未接受的文字送出去。
            self.control
                .set_text(&native_text(self.kind, self.draft.value()))?;
        }
        candidate
    }
    /// 仅在外层提交资格成立后准备。预留一个 ticket 给更晚的本地输入/IME。
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
    /// 已捕获的边沿值不重新读取 HWND；较新的草稿仍保留自己的本地 ticket。
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
    /// Wall等外层已核验身份后才调用；这里只编码并跟踪草稿，不产生挂载权限。
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
            // 无 handler 的 phase 不得解除其他 phase 尚待提交的新输入保护。
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
    /// 本地 listener 与远端提交共用 ticket owner，但本地输入不编码假远端 action。
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
    /// 未发出的请求恢复为待提交本地值；已有更新输入时只释放旧预留。
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
    /// 新快照仍由外层证据门禁接纳；不同绑定/节点必须销毁旧会话再创建。
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
    /// 仅接收外层已验证结果的事件 ID；重复/外来 ID 不改变状态。
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
        // 不用 WM_SETTEXT 打断 composition，也不在文本未变时重置选择区/撤销状态。
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
                // 取消 IME 不是远端结果；没有新权威值/回执时保留既有本地文字。
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
