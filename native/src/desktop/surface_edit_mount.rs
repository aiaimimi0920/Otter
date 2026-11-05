//! Surface 字段挂载 owner；最多 64 个字段，输入合并为每字段一份本地草稿，不在消息回调联网。
use super::{surface_edit_geometry, surface_edit_messages, surface_edit_session::Session};
use crate::{
    surface_click_event::ClickEvent,
    surface_input_event::Phase,
    surface_live_binding::Binding,
    surface_presentation::Presentation,
    surface_scene::{Kind, Node},
};
use std::collections::BTreeMap;
#[path = "surface_edit_controls.rs"]
mod controls;
#[path = "surface_edit_hover.rs"]
pub(crate) mod hover;
#[path = "surface_edit_intents.rs"]
mod intents;
#[path = "surface_edit_pending.rs"]
mod pending;
use intents::Edge;
#[path = "surface_edit_javascript.rs"]
mod javascript;
#[path = "surface_edit_keyboard.rs"]
mod keyboard;
#[path = "surface_edit_pointer.rs"]
pub(crate) mod pointer;
pub(crate) use javascript::Prepared as JavaScriptInput;
#[path = "surface_edit_prepare.rs"]
mod prepare;
#[path = "surface_edit_selects.rs"]
mod selects;
#[path = "surface_edit_sliders.rs"]
mod sliders;
#[path = "surface_number_spinner.rs"]
mod spinner;
#[path = "surface_switch_session.rs"]
mod switch_session;
#[path = "surface_edit_switches.rs"]
mod switches;
pub(crate) use intents::{Fence, FenceState};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
    },
};
struct Field {
    spinner: Option<spinner::Spinner>,
    spinner_visible: bool,
    session: Session,
    kind: Kind,
    contract: [u8; 32],
    changed: bool,
    change_epoch: u64,
    committed_epoch: u64,
    disabled: bool,
    edit_depth: u32,
    edit_notified: bool,
}
#[derive(Default)]
pub struct Mount {
    binding: Option<Binding>,
    fields: BTreeMap<String, Field>,
    fields_changed: bool,
    switches: BTreeMap<String, switch_session::Session>,
    selects: BTreeMap<String, selects::Field>,
    sliders: BTreeMap<String, sliders::Field>,
    switch_changes: bool,
    editing: bool,
    values: crate::surface_form_values::Values,
    intents: intents::Queue,
}
fn collect<'a>(node: &'a Node, out: &mut BTreeMap<&'a str, &'a Node>) {
    if !node.visible() {
        return;
    }
    if matches!(
        node.kind(),
        Kind::Input | Kind::Textarea | Kind::Number | Kind::Switch | Kind::Select | Kind::Slider
    ) {
        out.insert(node.id(), node);
    }
    for child in node.visible_children() {
        collect(child, out);
    }
}
impl Mount {
    pub fn bind(&mut self, binding: &Binding) {
        if self.binding.as_ref() != Some(binding) {
            self.fields_changed |= !self.fields.is_empty() || !self.selects.is_empty();
            self.fields.clear();
            self.selects.clear();
            self.fields_changed |= !self.sliders.is_empty();
            self.sliders.clear();
            self.switches.clear();
            self.switch_changes = false;
            self.values = Default::default();
            self.intents = Default::default();
            self.binding = Some(binding.clone());
        }
    }
    pub fn sync(&mut self, parent: HWND, presentation: &Presentation, dpi: u32) -> Result<()> {
        self.cancel_spinners();
        let Some(binding) = self.binding.clone() else {
            return Ok(());
        };
        let mut nodes = BTreeMap::new();
        collect(presentation.scene().root(), &mut nodes);
        if nodes.len() > 64 {
            return Err(Error::new(
                E_INVALIDARG,
                "Surface edit field limit exceeded",
            ));
        }
        let contracts = nodes
            .iter()
            .map(|(id, node)| {
                crate::surface_edit_contract::fingerprint(node)
                    .map(|contract| (*id, contract))
                    .map_err(|_| Error::new(E_INVALIDARG, "Surface edit contract encoding failed"))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let previous_count = self.fields.len();
        self.fields.retain(|id, field| {
            nodes
                .get(id.as_str())
                // 只允许权威 value 更新跨越会话，其他合同变化不能继承旧草稿/预留。
                .is_some_and(|node| {
                    node.kind() == field.kind && contracts.get(id.as_str()) == Some(&field.contract)
                })
        });
        self.fields_changed |= self.fields.len() != previous_count;
        self.sync_sliders(parent, presentation, &nodes, &contracts, dpi)?;
        self.sync_selects(parent, presentation, &nodes, &contracts, dpi)?;
        self.sync_switches(presentation, &nodes, &contracts)?;
        self.intents.retain(|id| {
            self.fields.contains_key(id)
                || self.switches.contains_key(id)
                || self.selects.contains_key(id)
                || self.sliders.contains_key(id)
        });
        let overlays = presentation.scrollbars();
        for (index, geometry) in presentation.geometry().nodes().iter().enumerate() {
            let Some(node) = nodes.get(geometry.id()) else {
                continue;
            };
            if matches!(node.kind(), Kind::Switch | Kind::Select | Kind::Slider) {
                continue;
            }
            if geometry.size().iter().any(|v| *v < 1.) {
                self.fields_changed |= self.fields.remove(geometry.id()).is_some();
                continue;
            }
            if !self.fields.contains_key(geometry.id()) {
                let session = Session::new(
                    parent,
                    node,
                    &binding,
                    RECT {
                        left: 0,
                        top: 0,
                        right: 1,
                        bottom: 1,
                    },
                )?;
                surface_edit_messages::install(
                    session.control().hwnd()?,
                    node.kind() == Kind::Number,
                )?;
                self.values.update(node.id(), session.form_value());
                self.fields.insert(
                    geometry.id().to_owned(),
                    Field {
                        spinner: if node.kind() == Kind::Number {
                            Some(spinner::Spinner::new(parent)?)
                        } else {
                            None
                        },
                        spinner_visible: false,
                        session,
                        kind: node.kind(),
                        contract: contracts[node.id()],
                        changed: false,
                        change_epoch: 0,
                        committed_epoch: 0,
                        disabled: false,
                        edit_depth: 0,
                        edit_notified: false,
                    },
                );
                self.fields_changed = true;
            }
            let field = self.fields.get_mut(geometry.id()).unwrap();
            let before = field.session.value().to_owned();
            field.session.synchronize(node, &binding)?;
            if field.session.value() != before {
                self.values.update(node.id(), field.session.form_value());
            }
            field.disabled = node.props().get("disabled") == Some(&serde_json::Value::Bool(true));
            let hwnd = field.session.control().hwnd()?;
            if let Some(colors) = presentation.edit_colors(node.id()) {
                field.session.control().set_colors(colors)?;
            }
            if field.kind == Kind::Textarea {
                field
                    .session
                    .control()
                    .set_alignment(node.style().get("textAlign").and_then(|v| v.as_str()))?;
            }
            if let Some(font) = presentation.control_font(node.id()) {
                field.session.control().set_font(dpi, font)?;
            }
            surface_edit_geometry::place(
                hwnd,
                geometry,
                &presentation.geometry().nodes()[index + 1..],
                &overlays,
                dpi,
                if let Some(style) = presentation.textarea_box(node.id()) {
                    surface_edit_geometry::Part::Textarea(style)
                } else if field.spinner.is_some() {
                    surface_edit_geometry::Part::Edit
                } else {
                    surface_edit_geometry::Part::Whole
                },
            )?;
            if let Some(spinner) = &field.spinner {
                field.spinner_visible = surface_edit_geometry::place(
                    spinner.hwnd()?,
                    geometry,
                    &presentation.geometry().nodes()[index + 1..],
                    &overlays,
                    dpi,
                    surface_edit_geometry::Part::Spinner,
                )?;
                spinner.enable(self.editing && !field.disabled, field.spinner_visible)?;
            }
            unsafe {
                let _ = EnableWindow(hwnd, self.editing && !field.disabled);
            }
            if field.disabled {
                surface_edit_messages::install(hwnd, field.kind == Kind::Number)?;
                field.session.end_composition()?;
                self.values.update(node.id(), field.session.form_value());
                self.intents.retain(|id| id != node.id());
                field.changed = false;
                field.edit_depth = 0;
            }
            field.session.control().show()?;
        }
        self.intents.retain(|id| {
            self.fields.contains_key(id)
                || self.switches.contains_key(id)
                || self.selects.contains_key(id)
                || self.sliders.contains_key(id)
        });
        Ok(())
    }
    pub fn policy(&mut self, editing: bool) -> Result<()> {
        self.editing = editing;
        for field in self.sliders.values() {
            field
                .control
                .set_enabled(editing && !field.disabled && field.visible)?;
        }
        for field in self.selects.values() {
            field
                .control
                .set_enabled(editing && !field.disabled && field.visible)?;
        }
        if !editing {
            self.intents.clear_waiting();
        }
        for (id, field) in &mut self.fields {
            if let Some(spinner) = &field.spinner {
                spinner.enable(editing && !field.disabled, field.spinner_visible)?;
            }
            if !editing {
                field.edit_depth = 0;
            }
            unsafe {
                let _ = EnableWindow(field.session.control().hwnd()?, editing && !field.disabled);
            }
            // EnableWindow 的同步结束通知可能发生在 Host 暂时取出期间。
            // 显式收尾仅保留本地文字，不把门禁关闭变成提交动作。
            if !editing {
                surface_edit_messages::install(
                    field.session.control().hwnd()?,
                    field.kind == Kind::Number,
                )?;
                field.session.end_composition()?;
                self.values.update(id, field.session.form_value());
            }
        }
        Ok(())
    }
    pub(in crate::desktop) fn take_structure_change(&mut self) -> bool {
        std::mem::take(&mut self.fields_changed)
    }
    pub fn number_editable(&self, hwnd: HWND) -> bool {
        self.editing
            && self.fields.values().any(|field| {
                field.kind == Kind::Number
                    && !field.disabled
                    && !field.session.composing()
                    && field.session.control().hwnd().ok() == Some(hwnd)
            })
    }
    pub fn cancel_spinners(&self) {
        for field in self.fields.values() {
            if let Some(spinner) = &field.spinner {
                spinner.cancel();
            }
        }
    }
    pub fn spinner_edit(&self, hwnd: HWND) -> Option<HWND> {
        if !self.editing {
            return None;
        }
        self.fields
            .values()
            .find(|field| {
                !field.disabled
                    && field.spinner_visible
                    && field
                        .spinner
                        .as_ref()
                        .is_some_and(|spinner| spinner.owns(hwnd))
            })
            .and_then(|field| field.session.control().hwnd().ok())
    }
    pub fn values(&self) -> &serde_json::Map<String, serde_json::Value> {
        self.values.entries()
    }
    pub fn seal_input_edge(&mut self) -> Fence {
        self.intents.seal()
    }
    pub fn fence_state(&self, fence: &Fence) -> FenceState {
        self.intents.state(fence)
    }
    pub fn notify(&mut self, hwnd: HWND, message: u32, code: u32) -> Result<()> {
        if self.notify_slider(hwnd, message, code)? {
            return Ok(());
        }
        if self.notify_select(hwnd, message, code)? {
            return Ok(());
        }
        let Some((id, field)) = self
            .fields
            .iter_mut()
            .find(|(_, field)| field.session.control().hwnd().ok() == Some(hwnd))
        else {
            return Ok(());
        };
        if !self.editing || field.disabled {
            return Ok(());
        }
        let mut changed = false;
        let mut commit = false;
        match message {
            surface_edit_messages::STEP_UP | surface_edit_messages::STEP_DOWN => {
                // Host 在处理通知期间已被取出，同步 EDIT 回调不能重入队列。
                // 整对边沿容量不足时不先修改 HWND/草稿，也不留下半个离散动作。
                if !self.intents.can_push_step(id) {
                    return Err(Error::new(E_FAIL, "Surface number step queue full"));
                }
                changed = field
                    .session
                    .step_number(message == surface_edit_messages::STEP_UP)?;
                commit = changed;
            }
            surface_edit_messages::EDIT_BEGIN => {
                if field.edit_depth == 0 {
                    field.session.begin_edit()?;
                    field.edit_notified = false;
                }
                field.edit_depth = field
                    .edit_depth
                    .checked_add(1)
                    .ok_or_else(|| Error::new(E_FAIL, "Edit nesting exceeded"))?;
            }
            surface_edit_messages::EDIT_END => {
                if field.edit_depth == 0 {
                    return Ok(());
                }
                field.edit_depth -= 1;
                if field.edit_depth == 0 {
                    changed = field
                        .session
                        .finish_edit(std::mem::take(&mut field.edit_notified))?;
                }
            }
            surface_edit_messages::COMPOSITION_START => field.session.begin_composition()?,
            surface_edit_messages::COMPOSITION_END => {
                changed = field.session.end_composition()?;
            }
            WM_COMMAND if code == EN_CHANGE => {
                if field.edit_depth != 0 {
                    field.edit_notified = true;
                    return Ok(());
                }
                changed = field.session.capture()?;
            }
            _ if (message == WM_COMMAND && code == EN_KILLFOCUS)
                || message == surface_edit_messages::COMMIT =>
            {
                changed = field.session.capture()?;
                commit = true;
            }
            _ => {}
        }
        if message != surface_edit_messages::EDIT_BEGIN
            && field.edit_depth == 0
            && !field.session.composing()
        {
            self.values.update(id, field.session.form_value());
        }
        // 队列拒收不改变已接纳的可见草稿/表单值，也不能清掉未成功排队的 change 标记。
        field.changed |= changed;
        if changed {
            field.change_epoch = field.change_epoch.saturating_add(1);
        }
        if changed && !self.intents.push(id, Phase::Input, field.session.value()) {
            return Err(Error::new(E_FAIL, "Surface edit queue full"));
        }
        if commit && field.changed {
            if !self.intents.push(id, Phase::Change, field.session.value()) {
                return Err(Error::new(E_FAIL, "Surface edit queue full"));
            }
            field.changed = false;
            field.committed_epoch = field.change_epoch;
        }
        Ok(())
    }
    pub fn cancel(&mut self, events: &[ClickEvent]) {
        for event in events.iter().rev() {
            if let Some(id) = self.intents.cancel(event.id()) {
                if let Some(field) = self.switches.get_mut(&id) {
                    field.cancel(event.id());
                }
                if let Some(field) = self.fields.get_mut(&id) {
                    field.session.cancel(event.id());
                }
            }
        }
    }
    pub fn settle(&mut self, events: &[ClickEvent]) -> Result<()> {
        for event in events {
            for field in self.switches.values_mut() {
                let before = field.value();
                field.settle(event.id());
                self.switch_changes |= before != field.value();
            }
            self.intents.settle(event.id());
        }
        for (id, field) in &mut self.fields {
            for event in events {
                let before = field.session.value().to_owned();
                field.session.settle(event.id())?;
                if field.session.value() != before {
                    self.values.update(id, field.session.form_value());
                }
                self.intents.settle(event.id());
            }
        }
        Ok(())
    }
    pub fn fail(&mut self, events: &[ClickEvent]) {
        let mut failed = false;
        for event in events {
            if let Some(id) = self.intents.failed(event.id()) {
                failed = true;
                if let Some(field) = self.switches.get_mut(&id) {
                    field.cancel(event.id());
                }
                if let Some(field) = self.fields.get_mut(&id) {
                    // 只归还本地预留，不采用权威值、不重新排队已尝试事件。
                    field.session.cancel(event.id());
                }
            }
        }
        if failed {
            for field in self.fields.values_mut() {
                field.changed = false;
            }
        }
    }
}
