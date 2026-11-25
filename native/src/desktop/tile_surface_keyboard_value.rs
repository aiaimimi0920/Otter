//! One active value control reuses ordinary Select/Slider HWNDs; notifications use the original bounded queue without another business worker.
use super::*;
use crate::{
    desktop::{surface_select_control, surface_slider_control},
    surface_input_event::Phase,
    tile_surface_edit::Edit,
    tile_surface_slider::Drag,
};
enum Control {
    Select(surface_select_control::Control, Option<usize>),
    Slider(surface_slider_control::Control, Box<Drag>),
}
pub(in crate::desktop::tile_window) struct Owner {
    pub focus: Focus,
    pub edit: Arc<Edit>,
    control: Control,
    popup: text::popup::Popup,
}
impl Owner {
    pub fn new(hwnd: HWND, state: &State, focus: Focus) -> Result<Option<Self>> {
        let scene = crate::surface_scene::Scene::parse(&focus.anchor.snapshot.scene)
            .map_err(|_| Error::new(E_INVALIDARG, "Tile keyboard scene"))?;
        let node = scene
            .root()
            .visible_node(&focus.node)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Tile keyboard node"))?;
        let popup = text::popup::Popup::new(hwnd)?;
        let [l, t, r, b] = focus.rect;
        let rect = RECT {
            left: 0,
            top: 0,
            right: r - l,
            bottom: b - t,
        };
        let edit = text::peer(
            state,
            &focus.anchor.view,
            focus.node.clone(),
            focus.contract,
        )
        .unwrap_or_else(|| Edit::new(focus.node.clone(), focus.contract));
        let control = match focus.kind {
            Kind::Select => {
                let options = crate::surface_select::Options::parse(node)
                    .map_err(|_| Error::new(E_INVALIDARG, "Tile options"))?;
                let accepted = match local::value(state, &focus) {
                    Some(crate::tile_surface_local::Value::Select(v)) => v,
                    _ => options.source_index(),
                };
                let c = surface_select_control::Control::new(popup.hwnd()?, rect, options)?;
                c.select(accepted)?;
                let font = actions::active(state).and_then(|frame| {
                    state
                        .renderer
                        .borrow()
                        .as_ref()
                        .and_then(|r| r.control_font(&frame, &focus))
                });
                if let Some(font) = font {
                    c.set_font(96, &font)?;
                }
                Control::Select(c, accepted)
            }
            Kind::Slider => {
                let Some(frame) = actions::active(state) else {
                    return Ok(None);
                };
                let Some(field) = state
                    .renderer
                    .borrow()
                    .as_ref()
                    .and_then(|r| r.slider(&frame, focus.anchor.pixel))
                else {
                    return Ok(None);
                };
                let range = crate::surface_slider::Range::parse(node)
                    .map_err(|_| Error::new(E_INVALIDARG, "Tile range"))?;
                let c = surface_slider_control::Control::new(popup.hwnd()?, rect, range)?;
                let mut drag = Drag::new(field);
                if let Some(crate::tile_surface_local::Value::Slider(v)) =
                    local::value(state, &focus)
                {
                    drag.local_start(v);
                    c.set_value(v)?;
                }
                drag.edit = edit.clone();
                Control::Slider(c, Box::new(drag))
            }
            _ => return Ok(None),
        };
        let owner = Self {
            focus,
            edit,
            control,
            popup,
        };
        match &owner.control {
            Control::Select(..) => owner.popup.install_select(owner.child()?)?,
            _ => owner.popup.install(owner.child()?)?,
        }
        owner.popup.show(hwnd, owner.focus.rect)?;
        match &owner.control {
            Control::Select(c, _) => {
                c.show()?;
                c.focus()?;
            }
            Control::Slider(c, _) => {
                c.show()?;
                c.focus()?;
            }
        }
        Ok(Some(owner))
    }
    pub fn child(&self) -> Result<HWND> {
        match &self.control {
            Control::Select(c, _) => c.hwnd(),
            Control::Slider(c, _) => c.hwnd(),
        }
    }
    pub fn dismiss(&self, wp: WPARAM, lp: LPARAM) -> bool {
        self.popup.matches(wp, lp)
    }
    pub fn synchronize(&mut self, state: &State) -> Result<()> {
        let Some(frame) = actions::active(state) else {
            return Ok(());
        };
        let Some(current) = current_in_frame(state, &self.focus, &frame) else {
            return Ok(());
        };
        self.focus = current;
        let pending = state.shared.lock().actions.preview_pending(
            &self.edit,
            &frame,
            &self.focus.anchor.view.instance_id,
        );
        match &mut self.control {
            Control::Select(c, accepted) if !pending => {
                let field =
                    state.renderer.borrow().as_ref().and_then(|r| {
                        r.value(&frame, self.focus.anchor.pixel, actions::modifiers())
                    });
                if let Some(field) = field {
                    if let crate::tile_surface_value::Control::Select(o) = field.control {
                        let desired = match local::value(state, &self.focus) {
                            Some(crate::tile_surface_local::Value::Select(v)) => v,
                            _ => o.source_index(),
                        };
                        if *accepted != desired {
                            c.select(desired)?;
                            *accepted = desired;
                        }
                    }
                }
            }
            Control::Slider(c, drag) => {
                let field = state
                    .renderer
                    .borrow()
                    .as_ref()
                    .and_then(|r| r.slider(&frame, self.focus.anchor.pixel));
                let source = field.as_ref().map(|field| field.source());
                if let Some(field) = field {
                    drag.field = field;
                }
                if !pending {
                    let source = if source.is_some() {
                        source
                    } else {
                        let scene =
                            crate::surface_scene::Scene::parse(&self.focus.anchor.snapshot.scene)
                                .map_err(|_| Error::new(E_INVALIDARG, "Tile range scene"))?;
                        scene
                            .root()
                            .visible_node(&self.focus.node)
                            .and_then(|node| crate::surface_slider::Range::parse(node).ok())
                            .map(|range| range.source())
                    };
                    if let Some(source) = source {
                        drag.value = match local::value(state, &self.focus) {
                            Some(crate::tile_surface_local::Value::Slider(v)) => v,
                            _ => source,
                        };
                        // Same-value synchronization does not cancel mouse dragging held inside native child controls.
                        if c.value()? != drag.value {
                            c.set_value(drag.value)?;
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    pub fn notify(&mut self, state: &State, msg: u32, wp: WPARAM) -> Result<()> {
        if !self.edit.active() {
            return Ok(());
        }
        let Some(focus) = current(state, &self.focus) else {
            // Native children process keys before synchronous notification. During blank/version-transition intervals, restore the last accepted value immediately,
            // preventing unauthorized local changes from surviving until the next valid key and being indirectly committed.
            match &self.control {
                Control::Select(c, accepted) => c.select(*accepted)?,
                Control::Slider(c, drag) => c.set_value(drag.value)?,
            }
            return Ok(());
        };
        self.focus = focus;
        let mut commands = match &mut self.control {
            Control::Select(c, accepted)
                if msg == WM_COMMAND
                    && matches!((wp.0 >> 16) as u32, CBN_SELCHANGE | CBN_CLOSEUP) =>
            {
                let selected = c.selected_index()?;
                if *accepted == selected {
                    return Ok(());
                }
                let Some(field) = actions::values::field(state, self.focus.anchor.pixel) else {
                    return Ok(());
                };
                let commands = field
                    .commands_after(selected, *accepted)
                    .map_err(|e| Error::new(E_INVALIDARG, e))?;
                if matches!(
                    local::value(state, &self.focus),
                    Some(crate::tile_surface_local::Value::Select(_))
                ) {
                    local::set(
                        state,
                        self.focus.clone(),
                        crate::tile_surface_local::Value::Select(selected),
                    )?;
                }
                *accepted = selected;
                commands
            }
            Control::Slider(c, drag) if msg == surface_slider_control::NOTIFY => {
                let field = actions::active(state).and_then(|frame| {
                    state
                        .renderer
                        .borrow()
                        .as_ref()?
                        .slider(&frame, self.focus.anchor.pixel)
                });
                let Some(field) = field else {
                    return Ok(());
                };
                drag.field = field;
                drag.value = c.value()?;
                if matches!(
                    local::value(state, &self.focus),
                    Some(crate::tile_surface_local::Value::Slider(_))
                ) {
                    local::set(
                        state,
                        self.focus.clone(),
                        crate::tile_surface_local::Value::Slider(drag.value),
                    )?;
                }
                drag.command(if wp.0 == 0 {
                    Phase::Input
                } else {
                    Phase::Change
                })
                .map_err(|e| Error::new(E_INVALIDARG, e))?
                .into_iter()
                .collect()
            }
            Control::Slider(_, _) if msg == surface_slider_control::CLICK_END => {
                actions::prepare(state, self.focus.anchor.pixel)
            }
            _ => return Ok(()),
        };
        text::decorate(state, &mut commands)?;
        for command in &mut commands {
            command.edit = Some(self.edit.clone());
        }
        if !commands.is_empty() && !state.shared.lock().actions.offer(commands, Instant::now()) {
            state.shared.lock().actions.cancel_edit(&self.edit);
        }
        state.text_dirty.set(true);
        Ok(())
    }
}
#[cfg(test)]
impl Owner {
    pub(in crate::desktop::tile_window) fn displayed_value(&self) -> serde_json::Value {
        match &self.control {
            Control::Select(c, _) => serde_json::json!(c.value().unwrap()),
            Control::Slider(c, _) => serde_json::json!(c.value().unwrap()),
        }
    }
}
