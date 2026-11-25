//! Pin-specific settings projection; the theme library owns only vertical controls, while this module owns targets, geometry, commands, and lifecycle.
use super::{popup_menu::target::MenuTarget, tool_panel_availability::Availability, State};
use crate::{
    editor::{DocumentStamp, Tool},
    pin_geometry::PinRect,
};
use neuro_mini_ui::{PropertyField as Field, PropertyKind as Kind, PropertyPanel};
use std::rc::Rc;
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
    },
};
#[path = "pin_settings_connections.rs"]
mod connections;
#[path = "pin_settings_fields.rs"]
mod fields;
#[path = "pin_settings_position.rs"]
mod position;
#[cfg(test)]
pub(in crate::desktop) use connections::target_list::tests::Confirmed as ConfirmedTargets;
pub(in crate::desktop) use connections::translate_targets;
pub(in crate::desktop) use position::{follow, moving_position};
#[path = "pin_settings_text.rs"]
pub(in crate::desktop) mod text;
pub(super) use text::{timer as text_timer, TIMER as TEXT_TIMER};
pub(super) const WIDTH: u16 = 101;
pub(super) const HEIGHT: u16 = 102;
pub(super) const OPACITY: u16 = 103;
pub(super) const FLIP_X: u16 = 104;
pub(super) const FLIP_Y: u16 = 105;
pub(super) const FIT: u16 = 106;
pub(super) const BORDER: u16 = 107;
pub(super) const RADIUS: u16 = 108;

#[derive(Default)]
pub(super) struct Session {
    panel: Option<Rc<PropertyPanel>>,
    open: bool,
    context: u64,
    target: Option<MenuTarget>,
    document: Option<DocumentStamp>,
    tool: Option<(Tool, bool)>,
    frame: Option<PinRect>,
    opacity: f32,
    mini: bool,
    display: Option<std::sync::Arc<()>>,
    style: crate::pin_frame_style::Style,
    text_stamp: Option<super::extension_document::Stamp>,
    text_feedback: Option<text::Feedback>,
    connections: connections::Session,
}
fn ready(state: &State) -> bool {
    ready_with(state, Availability::capture(state))
}
fn ready_with(state: &State, available: Option<Availability>) -> bool {
    !state.session_end.paused()
        && !state.group_hidden.get()
        && !state.group_locked.get()
        && super::capture_input::exclusive_available(state)
        && available.is_some_and(|a| a.allows(super::context_menu::OPEN))
        && state
            .canvas
            .try_borrow()
            .is_ok_and(|c| c.pinned && !c.blocks_wheel() && c.editor.draft().is_none())
}
pub(super) unsafe fn window(state: &State) -> Option<HWND> {
    state
        .pin_settings
        .try_borrow()
        .ok()?
        .panel
        .as_ref()?
        .window()
}
pub(super) fn is_open(state: &State) -> bool {
    state.pin_settings.try_borrow().is_ok_and(|s| s.open)
}
pub(super) unsafe fn close(state: &State) {
    connections::close_targets(state);
    let panel = {
        let mut s = state.pin_settings.borrow_mut();
        s.open = false;
        s.target = None;
        if let Some(owner) = neuro_mini_ui::owner(state.tool_panel.get()) {
            text::clear(owner, &mut s);
        }
        s.panel.clone()
    };
    if let Some(p) = panel {
        p.show(false);
    }
}
pub(super) unsafe fn toggle(owner: HWND, state: &State) -> Result<()> {
    if is_open(state) {
        close(state);
        return Ok(());
    }
    if !ready(state) || !IsWindowVisible(state.tool_panel.get()).as_bool() {
        return Ok(());
    }
    super::menu_transition::prepare(owner, state)?;
    let panel = state.pin_settings.borrow().panel.clone();
    let panel = match panel {
        Some(p) if p.window().is_some() => p,
        _ => Rc::new(PropertyPanel::new(state.tool_panel.get(), 125)?),
    };
    super::application::chrome::collapse_for_settings(state, owner)?;
    {
        let c = state.canvas.borrow();
        let mut s = state.pin_settings.borrow_mut();
        s.panel = Some(panel);
        s.open = true;
        if !s
            .document
            .as_ref()
            .is_some_and(|d| d.same_document(&c.editor))
        {
            s.connections.receive = connections::source(state).is_some();
        }
        s.document = Some(c.editor.document_stamp());
        s.tool = Some((c.tool, c.crop_mode));
        s.target = None;
    }
    if let Err(e) = refresh_at(owner, state) {
        close(state);
        return Err(e);
    }
    Ok(())
}
pub(super) unsafe fn refresh(state: &State) {
    if super::tool_panel::popup::active(state) {
        return;
    }
    let Some(owner) = neuro_mini_ui::owner(state.tool_panel.get()) else {
        return;
    };
    connections::poll_targets(owner, state);
    if let Err(e) = refresh_at(owner, state) {
        eprintln!("native-pin-settings-refresh: {e}");
        close(state);
    }
}
unsafe fn refresh_at(owner: HWND, state: &State) -> Result<()> {
    let Ok(c) = state.canvas.try_borrow() else {
        return Ok(());
    };
    super::extension_document::observe(state);
    let Ok(mut s) = state.pin_settings.try_borrow_mut() else {
        return Ok(());
    };
    if !s.open {
        return Ok(());
    }
    if !IsWindowVisible(state.tool_panel.get()).as_bool()
        || s.tool != Some((c.tool, c.crop_mode))
        || !s
            .document
            .as_ref()
            .is_some_and(|d| d.same_document(&c.editor))
    {
        drop(s);
        drop(c);
        close(state);
        return Ok(());
    }
    let Some(panel) = s.panel.clone() else {
        return Ok(());
    };
    if s.target.is_some() && !panel.is_open() {
        drop(s);
        drop(c);
        close(state);
        return Ok(());
    }
    let frame = super::pin_settings_history::frame(owner)?;
    if !s
        .text_stamp
        .as_ref()
        .is_some_and(|stamp| stamp.matches(&state.extension_document.borrow()))
    {
        s.context = s.context.wrapping_add(1);
        s.text_stamp = Some(state.extension_document.borrow().stamp());
        text::clear(owner, &mut s);
    }
    let dpi = GetDpiForWindow(owner).max(96);
    let mini = c.minified.is_some();
    let identity = state
        .resolved_display
        .borrow()
        .as_ref()
        .map(|f| f.identity.clone());
    let same_display = match (&s.display, &identity) {
        (None, None) => true,
        (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, b),
        _ => false,
    };
    if !s.target.as_ref().is_some_and(|t| t.matches(&c))
        || s.frame
            .is_none_or(|old| old.width != frame.width || old.height != frame.height)
        || s.opacity != c.window_opacity
        || s.mini != mini
        || !same_display
        || s.style != c.frame_style
    {
        s.context = s.context.wrapping_add(1);
        s.target = Some(MenuTarget::capture(&c));
        s.opacity = c.window_opacity;
        s.mini = mini;
        s.display = identity;
        s.style = c.frame_style;
    }
    // Translation does not change field semantics or retire drafts; commits still validate the latest screen rectangle, and size changes still advance generation.
    s.frame = Some(frame);
    let mirror = state
        .resolved_display
        .borrow()
        .as_ref()
        .and_then(|f| f.editor.as_deref())
        .unwrap_or(&c.editor)
        .mirror_state();
    let mut sections = vec![
        fields::fields(frame, dpi, c.window_opacity, c.frame_style, mirror)
            .into_iter()
            .map(|f| (f, true))
            .collect(),
        text::fields(s.text_feedback.as_ref())
            .into_iter()
            .map(|f| (f, true))
            .collect(),
    ];
    sections.extend(connections::refresh(state, &mut s)?);
    let context = s.context;
    let can_flip = !mini
        && !state.is_art()
        && (c.editor.layers().is_some() || state.resolved_display.borrow().is_some());
    let can_fit = !mini && fields::image_size(state, &c).is_some();
    drop(s);
    drop(c);
    let enabled = ready_with(
        state,
        Availability::capture(state).map(Availability::presentation),
    );
    let sections: Vec<Vec<(Field, bool)>> = sections
        .into_iter()
        .map(|fields| {
            fields
                .into_iter()
                .map(|(field, permitted)| {
                    let available = enabled
                        && permitted
                        && match field.id {
                            FLIP_X | FLIP_Y => can_flip,
                            FIT => can_fit,
                            WIDTH => !mini,
                            text::OCR | text::TRANSLATION => text::available(state, field.id),
                            _ => true,
                        };
                    (field, available)
                })
                .collect()
        })
        .collect();
    let captions = super::property_panel_captions::Captions::new(
        sections.iter().flatten().map(|(field, _)| field),
    );
    panel.update_sections(context, sections)?;
    let toolbar = state.tool_panel.get();
    let current = || {
        state.lifetime.owns(owner)
            && state.tool_panel.get() == toolbar
            && neuro_mini_ui::owner(toolbar) == Some(owner)
            && state.pin_settings.try_borrow().is_ok_and(|session| {
                session.open
                    && session.context == context
                    && session
                        .panel
                        .as_ref()
                        .is_some_and(|p| Rc::ptr_eq(p, &panel))
            })
    };
    let Some(root) = panel.window() else {
        return Ok(());
    };
    match captions.sync(root, owner, || current() && panel.window() == Some(root)) {
        Ok(false) => return Ok(()),
        Err(error) => eprintln!("native-pin-settings-captions: {error}"),
        Ok(true) => {}
    }
    if !current() || panel.window() != Some(root) {
        return Ok(());
    }
    position::place(owner, &panel)?;
    if current() && panel.window() == Some(root) {
        panel.show(true);
    }
    Ok(())
}
pub(super) unsafe fn execute(owner: HWND, state: &State, id: u16) -> Result<()> {
    execute_with(owner, state, id, |owner, state, id| {
        text::copy(owner, state, id)
    })
}
pub(in crate::desktop) unsafe fn execute_with(
    owner: HWND,
    state: &State,
    id: u16,
    copy: impl FnOnce(HWND, &State, u16) -> Result<()>,
) -> Result<()> {
    let panel = state.pin_settings.borrow().panel.clone();
    let Some(edit) = panel.and_then(|p| p.take_edit(id)) else {
        return Ok(());
    };
    if !ready(state)
        || !state.lifetime.owns(owner)
        || !IsWindowVisible(state.tool_panel.get()).as_bool()
    {
        return Ok(());
    }
    let current = super::pin_settings_history::frame(owner)?;
    {
        let c = state.canvas.borrow();
        let s = state.pin_settings.borrow();
        let display = state.resolved_display.borrow();
        let same_display = match (&s.display, display.as_ref()) {
            (None, None) => true,
            (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, &b.identity),
            _ => false,
        };
        if !s.open
            || edit.context != s.context
            || s.frame != Some(current)
            || s.opacity != c.window_opacity
            || s.mini != c.minified.is_some()
            || !same_display
            || s.style != c.frame_style
            || !s.target.as_ref().is_some_and(|t| t.matches(&c))
            || !s
                .text_stamp
                .as_ref()
                .is_some_and(|stamp| stamp.matches(&state.extension_document.borrow()))
        {
            return Ok(());
        }
    }
    if connections::execute(owner, state, id)? {
        super::tool_panel::refresh(owner);
        return Ok(());
    }
    match id {
        text::OCR | text::TRANSLATION => copy(owner, state, id)?,
        BORDER => {
            let active = state.edit_palette.borrow().active;
            let next = state
                .canvas
                .borrow()
                .frame_style
                .toggle(crate::surface_color::Color([
                    f32::from(active.r) / 255.0,
                    f32::from(active.g) / 255.0,
                    f32::from(active.b) / 255.0,
                    f32::from(active.a) / 255.0,
                ]));
            super::pin_settings_history::style(owner, state, next)?;
        }
        RADIUS => {
            let value = edit
                .value
                .as_deref()
                .and_then(|s| s.parse::<u8>().ok())
                .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid frame corner radius"))?;
            let next = state
                .canvas
                .borrow()
                .frame_style
                .with_radius(value)
                .map_err(|message| Error::new(E_INVALIDARG, message))?;
            super::pin_settings_history::style(owner, state, next)?;
        }
        WIDTH if state.canvas.borrow().minified.is_none() => {
            let value = edit
                .value
                .as_deref()
                .and_then(|v| v.parse::<i32>().ok())
                .filter(|v| (32..=8192).contains(v))
                .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid pin settings width"))?;
            let current_width = (f64::from(current.width) * 96.0
                / f64::from(GetDpiForWindow(owner).max(96)))
            .round() as i32;
            if value == current_width.max(32) {
                return Ok(());
            }
            let next =
                super::pin_settings_geometry::width(current, value, GetDpiForWindow(owner).max(96))
                    .ok_or_else(|| {
                        Error::new(E_BOUNDS, "Pin settings width exceeds renderer budget")
                    })?;
            super::pin_settings_history::resize(owner, state, next)?;
        }
        OPACITY => {
            let value = edit
                .value
                .as_deref()
                .and_then(|v| v.parse::<u32>().ok())
                .filter(|v| *v <= 100)
                .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid pin settings opacity"))?;
            if value == (state.canvas.borrow().window_opacity * 100.0).round() as u32 {
                return Ok(());
            }
            super::pin_settings_history::opacity(owner, state, value as f32 / 100.0)?;
        }
        FIT if state.canvas.borrow().minified.is_none() => {
            let size = {
                let c = state.canvas.borrow();
                fields::image_size(state, &c)
            };
            if let Some(size) = size {
                let next = super::pin_settings_geometry::fit(
                    current,
                    size,
                    GetDpiForWindow(owner).max(96),
                )
                .ok_or_else(|| Error::new(E_BOUNDS, "Invalid fitted pin settings frame"))?;
                super::pin_settings_history::resize(owner, state, next)?;
            }
        }
        FLIP_X | FLIP_Y if state.canvas.borrow().minified.is_none() && !state.is_art() => {
            super::flip_input::start(
                owner,
                state,
                if id == FLIP_X {
                    super::flip_input::HORIZONTAL
                } else {
                    super::flip_input::VERTICAL
                },
            );
        }
        _ => {}
    }
    super::tool_panel::refresh(owner);
    Ok(())
}
pub(super) unsafe fn translate(state: &State, message: &MSG) -> bool {
    let panel = state.pin_settings.borrow().panel.clone();
    let Some(panel) = panel.filter(|p| p.contains(message.hwnd)) else {
        return false;
    };
    if let Some(owner) = neuro_mini_ui::owner(state.tool_panel.get()) {
        if super::clipboard_input::translate(owner, message) {
            return true;
        }
    }
    if message.message == WM_KEYDOWN
        && message.wParam.0 == 0x45
        && windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState(0x11) < 0
    {
        if let Some(owner) = neuro_mini_ui::owner(state.tool_panel.get()) {
            SendMessageW(
                owner,
                message.message,
                Some(message.wParam),
                Some(message.lParam),
            );
            return true;
        }
    }
    panel.translate(message)
}
