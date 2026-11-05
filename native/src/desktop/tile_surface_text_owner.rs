//! UI字段会话：复用原Session/草稿ticket，只有当前paint身份才把意图交给原Art队列。
use super::*;
use crate::desktop::surface_edit_session::Session;
use crate::{
    surface_input_event::Phase,
    surface_scene::Scene,
    tile_surface_actions::{Command, Kind},
    tile_surface_edit::Edit,
};
use std::collections::{BTreeMap, BTreeSet};
pub(super) type Id = (String, String);
pub(super) struct Slot {
    pub field: Field,
    pub session: Session,
    pub edit: Arc<Edit>,
    events: BTreeSet<String>,
    pub depth: u32,
    pub notified: bool,
    input: bool,
    commit: Option<String>,
    committed: String,
    dirty: bool,
    preview: std::rc::Rc<str>,
}
pub(in crate::desktop::tile_window) struct Owner {
    pub(super) fields: BTreeMap<Id, Slot>,
    pub(super) active: Option<Id>,
    pub(super) popup: popup::Popup,
    snapshot: std::rc::Rc<Vec<crate::tile_surface_text::Preview>>,
}
impl Owner {
    pub(in crate::desktop::tile_window) fn counts(
        &self,
        view: &crate::tile_surface_wire::View,
    ) -> (usize, usize) {
        (
            self.fields.len(),
            self.fields
                .values()
                .filter(|s| &s.field.anchor.view == view)
                .count(),
        )
    }
    pub fn new(parent: HWND) -> Result<Self> {
        Ok(Self {
            fields: BTreeMap::new(),
            active: None,
            popup: popup::Popup::new(parent)?,
            snapshot: Default::default(),
        })
    }
    pub fn owns(&self, child: HWND) -> Option<Id> {
        self.fields
            .iter()
            .find(|(_, s)| s.session.control().hwnd().ok() == Some(child))
            .map(|(id, _)| id.clone())
    }
    pub fn open(&mut self, hwnd: HWND, field: Field, state: &State) -> Result<()> {
        let id = (field.anchor.view.instance_id.clone(), field.node.clone());
        self.close(state, true)?;
        let compatible = self.fields.get(&id).is_some_and(|s| {
            s.field.anchor.view == field.anchor.view
                && s.field.anchor.generation == field.anchor.generation
                && s.field.contract == field.contract
                && s.edit.active()
        });
        if !compatible {
            if let Some(old) = self.fields.remove(&id) {
                state.shared.lock().actions.cancel_edit(&old.edit);
            }
            let local = state.local.borrow().counts(&field.anchor.view);
            if self.fields.len() + local.0 >= 256
                || self
                    .fields
                    .values()
                    .filter(|s| s.field.anchor.view == field.anchor.view)
                    .count()
                    + local.1
                    >= 64
            {
                return Err(Error::new(
                    E_OUTOFMEMORY,
                    "Tile text field capacity exhausted",
                ));
            }
            let scene = Scene::parse(&field.anchor.snapshot.scene)
                .map_err(|_| Error::new(E_INVALIDARG, "Tile text scene"))?;
            let node = scene
                .root()
                .visible_node(&field.node)
                .ok_or_else(|| Error::new(E_INVALIDARG, "Tile text field missing"))?;
            let session = Session::unbound(
                self.popup.hwnd()?,
                node,
                RECT {
                    left: 0,
                    top: 0,
                    right: field.rect[2] - field.rect[0],
                    bottom: field.rect[3] - field.rect[1],
                },
            )?;
            let edit = super::peer(
                state,
                &field.anchor.view,
                field.node.clone(),
                field.contract,
            )
            .unwrap_or_else(|| {
                self.fields
                    .values()
                    .find(|s| s.field.anchor.view == field.anchor.view && s.edit.active())
                    .map_or_else(
                        || Edit::new(field.node.clone(), field.contract),
                        |s| Edit::peer(field.node.clone(), field.contract, &s.edit),
                    )
            });
            crate::desktop::surface_edit_messages::install(
                session.control().hwnd()?,
                field.kind == crate::surface_scene::Kind::Number,
            )?;
            self.popup.install(session.control().hwnd()?)?;
            let value = session.value().to_owned();
            self.fields.insert(
                id.clone(),
                Slot {
                    field: field.clone(),
                    session,
                    edit,
                    events: BTreeSet::new(),
                    depth: 0,
                    notified: false,
                    input: false,
                    commit: None,
                    committed: value.clone(),
                    dirty: false,
                    preview: value.into(),
                },
            );
        }
        let slot = self.fields.get_mut(&id).unwrap();
        slot.field = field;
        let rect = slot.field.rect;
        slot.session.control().place(RECT {
            left: 0,
            top: 0,
            right: rect[2] - rect[0],
            bottom: rect[3] - rect[1],
        })?;
        slot.session.control().set_font(96, &slot.field.font)?;
        self.active = Some(id);
        self.popup.show(hwnd, rect)?;
        slot.session.control().show()?;
        slot.session.control().focus()?;
        Ok(())
    }
    pub fn close(&mut self, state: &State, commit: bool) -> Result<()> {
        if let Some(id) = self.active.take() {
            let slot = self.fields.get_mut(&id).unwrap();
            if commit {
                if slot.session.composing() {
                    self.active = Some(id);
                    return Err(Error::new(E_FAIL, "Tile composition still active"));
                }
                slot.capture(true)?;
                self.flush_one(&id, state)?;
            } else {
                state.shared.lock().actions.cancel_edit(&slot.edit);
                slot.input = false;
                slot.commit = None;
                for event in std::mem::take(&mut slot.events) {
                    slot.session.cancel(&event);
                }
            }
            if let Some(slot) = self.fields.get(&id) {
                unsafe {
                    let _ = ShowWindow(slot.session.control().hwnd()?, SW_HIDE);
                }
            }
        }
        self.popup.hide();
        Ok(())
    }
    pub fn retire(&mut self, state: &State) {
        for slot in self.fields.values() {
            state.shared.lock().actions.cancel_edit(&slot.edit);
        }
        self.active = None;
        self.fields.clear();
        self.popup.hide();
    }
    pub fn flush_one(&mut self, id: &Id, state: &State) -> Result<()> {
        let Some(slot) = self.fields.get_mut(id) else {
            return Ok(());
        };
        if !slot.edit.active() {
            slot.input = false;
            slot.commit = None;
            return Ok(());
        }
        slot.reconcile(state)?;
        if !slot.input && slot.commit.is_none() {
            return Ok(());
        }
        let Some(frame) = super::current_frame(state) else {
            return Ok(());
        };
        if !super::field(state, slot.field.anchor.pixel).is_some_and(|f| slot.field.same(&f)) {
            return Ok(());
        }
        let scene = Scene::parse(&slot.field.anchor.snapshot.scene)
            .map_err(|_| Error::new(E_INVALIDARG, "Tile text scene"))?;
        let node = scene.root().visible_node(&slot.field.node).unwrap();
        if slot.session.composing() {
            return Ok(());
        }
        let mut commands = Vec::new();
        let mut created = Vec::new();
        let input = slot.input;
        let commit = slot.commit.clone();
        let prepared = (|| -> Result<()> {
            for (phase, value) in [
                (Phase::Input, input.then(|| slot.session.value().to_owned())),
                (Phase::Change, commit),
            ] {
                let Some(value) = value else {
                    continue;
                };
                let a = &slot.field.anchor;
                let identity = crate::surface_click_event::Identity {
                    instance: &a.view.instance_id,
                    attachment: &a.view.attachment_id,
                    generation: a.generation,
                };
                if let Some((event, bytes)) = slot.session.prepare_unbound(
                    node,
                    identity,
                    a.snapshot.revision,
                    phase,
                    Some(&value),
                )? {
                    created.push(event);
                    commands.push(Command {
                        anchor: a.clone(),
                        kind: Kind::Event(
                            serde_json::from_slice(&bytes)
                                .map_err(|_| Error::new(E_INVALIDARG, "Tile text event"))?,
                        ),
                        until: Instant::now() + std::time::Duration::from_millis(1500),
                        edit: Some(slot.edit.clone()),
                    });
                }
            }
            Ok(())
        })();
        if let Err(error) = prepared {
            for id in &created {
                slot.session.cancel(id);
            }
            return Err(error);
        }
        slot.input = false;
        slot.commit = None;
        let _ = frame;
        if !commands.is_empty() && !state.shared.lock().actions.offer(commands, Instant::now()) {
            for id in &created {
                slot.session.cancel(id);
            }
            state.shared.lock().actions.cancel_edit(&slot.edit);
        } else {
            slot.events.extend(created);
        }
        slot.reconcile(state)?;
        Ok(())
    }
    /// 值来自当前本地会话，而非旧权威scene；复用表单的16字段/48KiB预算及64KiB事件门。
    pub fn decorate(&self, commands: &mut [Command]) -> Result<()> {
        let mut total = 0;
        for command in commands {
            let Kind::Event(event) = &mut command.kind else {
                continue;
            };
            if event["event"] != "click" {
                continue;
            }
            if let Some(original) = event["payload"]["surfaceValues"].as_object() {
                let scene = Scene::parse(&command.anchor.snapshot.scene)
                    .map_err(|_| Error::new(E_INVALIDARG, "Tile form scene"))?;
                let mut values = crate::surface_form_values::Values::default();
                for (id, value) in original {
                    if let Some(value) = value.as_str() {
                        values.update(id, value);
                    }
                }
                for slot in self.fields.values().filter(|s| {
                    s.field.anchor.view == command.anchor.view
                        && s.field.anchor.generation == command.anchor.generation
                        && scene
                            .root()
                            .visible_node(&s.field.node)
                            .and_then(|n| crate::surface_edit_contract::fingerprint(n).ok())
                            == Some(s.field.contract)
                }) {
                    values.update(&slot.field.node, slot.session.form_value());
                }
                event["payload"]["surfaceValues"] = values.entries().clone().into();
            }
            total += serde_json::to_vec(event)
                .map_err(|_| Error::new(E_INVALIDARG, "Tile form encoding"))?
                .len();
        }
        if total > crate::surface_click_event::MAX_EVENT_BYTES {
            return Err(Error::new(E_INVALIDARG, "Tile form event budget"));
        }
        Ok(())
    }
    pub fn synchronize(&mut self, state: &State) -> Result<()> {
        let mut remove = Vec::new();
        let frame = super::current_frame(state);
        for (id, slot) in &mut self.fields {
            if frame.as_ref().is_some_and(|f| slot.field.current(f)) {
                slot.reconcile(state)?;
                if !slot.dirty && slot.commit.is_none() && slot.committed != slot.session.value() {
                    slot.committed = slot.session.value().to_owned();
                }
            } else if let Some(next) = frame.as_ref().and_then(|frame| {
                state
                    .renderer
                    .borrow()
                    .as_ref()?
                    .retained_text(frame, &slot.field)
            }) {
                slot.field = next;
                let scene = Scene::parse(&slot.field.anchor.snapshot.scene)
                    .map_err(|_| Error::new(E_INVALIDARG, "Tile text scene"))?;
                slot.session
                    .synchronize_unbound(scene.root().visible_node(&slot.field.node).unwrap())?;
                slot.reconcile(state)?;
                if !slot.dirty && slot.commit.is_none() && slot.committed != slot.session.value() {
                    slot.committed = slot.session.value().to_owned();
                }
            } else if !slot.field.awaiting(&state.shared.lock(), Instant::now()) {
                state.shared.lock().actions.cancel_edit(&slot.edit);
                remove.push(id.clone());
            }
        }
        for id in remove {
            self.fields.remove(&id);
            if self.active.as_ref() == Some(&id) {
                self.active = None;
                self.popup.hide();
            }
        }
        let ids = self.fields.keys().cloned().collect::<Vec<_>>();
        for id in ids {
            self.flush_one(&id, state)?;
        }
        Ok(())
    }
    pub fn previews(&mut self) -> std::rc::Rc<Vec<crate::tile_surface_text::Preview>> {
        if self.snapshot.len() == self.fields.len()
            && self
                .snapshot
                .iter()
                .zip(self.fields.values())
                .all(|(row, s)| {
                    row.field.same(&s.field)
                        && Arc::ptr_eq(&row.field.anchor.snapshot, &s.field.anchor.snapshot)
                        && row.value.as_ref() == s.session.value()
                })
        {
            return self.snapshot.clone();
        }
        let mut rows = Vec::new();
        for slot in self.fields.values_mut() {
            if slot.preview.as_ref() != slot.session.value() {
                slot.preview = slot.session.value().into();
            }
            rows.push(crate::tile_surface_text::Preview {
                field: slot.field.clone(),
                value: slot.preview.clone(),
            });
        }
        self.snapshot = std::rc::Rc::new(rows);
        self.snapshot.clone()
    }
}
impl Slot {
    pub fn input_changed(&mut self) {
        self.input = true;
        self.dirty = true;
    }
    pub fn capture(&mut self, commit: bool) -> Result<()> {
        if self.session.capture()? {
            self.input_changed();
        }
        if commit
            && self.dirty
            && !self.session.composing()
            && self.session.value() != self.committed
        {
            self.committed = self.session.value().to_owned();
            self.commit = Some(self.committed.clone());
            self.dirty = false;
        }
        Ok(())
    }
    fn reconcile(&mut self, state: &State) -> Result<()> {
        // 新权威已到但尚未paint时，不能用旧scene结算ticket并短暂回滚活动EDIT。
        if state
            .shared
            .lock()
            .actions
            .states
            .get(&self.field.anchor.view.instance_id)
            .is_some_and(|s| {
                s.view == self.field.anchor.view
                    && s.generation == self.field.anchor.generation
                    && !Arc::ptr_eq(&s.snapshot, &self.field.anchor.snapshot)
            })
        {
            return Ok(());
        }
        let done = self
            .events
            .iter()
            .filter(|id| {
                !state
                    .shared
                    .lock()
                    .actions
                    .edit_event_pending(id, &self.edit)
            })
            .cloned()
            .collect::<Vec<_>>();
        for id in done {
            self.events.remove(&id);
            if self.edit.active() {
                self.session.settle(&id)?;
            } else {
                self.session.cancel(&id);
            }
        }
        Ok(())
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.fields.clear();
    }
}
