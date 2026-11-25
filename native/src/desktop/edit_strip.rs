//! Attached edit strips for pins: Neuro-mini draws baseline grouped layouts; Otter retains commands and host tracking.
use super::{Availability, State, Window};
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
use windows::core::Error;
use windows::{
    core::{Result, HSTRING},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
#[path = "edit_strip_catalog.rs"]
mod catalog;
#[path = "edit_strip_icons.rs"]
mod icons;
#[path = "edit_strip_menus.rs"]
mod menus;
#[path = "edit_strip_position.rs"]
mod position;
#[cfg(test)]
use position::anchored;
pub(in crate::desktop) use position::moving_positions;
pub(super) use position::place;
#[cfg(test)]
#[path = "edit_strip_tests.rs"]
mod tests;
#[cfg(test)]
pub(super) fn control_ids() -> Vec<i32> {
    catalog::items()
        .iter()
        .filter_map(|item| match item {
            neuro_mini_ui::Item::Button(b) => Some(b.id as i32),
            _ => None,
        })
        .collect()
}
pub(super) unsafe fn create(owner: HWND, state: &State, visible: bool) -> Result<Window> {
    let panel = Window(neuro_mini_ui::create(
        owner,
        "贴图编辑",
        super::EXECUTE,
        &catalog::items(),
    )?);
    state.tool_panel.set(panel.0);
    place(owner, state, None)?;
    if visible {
        // Initially visible edit strips also enable selection by default; hidden creation preserves free pin dragging.
        state
            .canvas
            .borrow_mut()
            .select_tool(crate::editor::Tool::Select);
        super::prepare(panel.0, state)?;
        let _ = ShowWindow(panel.0, SW_SHOWNOACTIVATE);
    }
    Ok(panel)
}

/// Pins lack the old grid's scroll/property controls; use real edit-strip BUTTONs to verify selection-command delivery.
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
pub(in crate::desktop) unsafe fn smoke(owner: HWND, state: &State) -> Result<()> {
    use crate::editor::Tool;
    let panel = state.tool_panel.get();
    super::prepare(panel, state)?;
    let button = GetDlgItem(Some(panel), i32::from(catalog::MODE))?;
    for enabled in [true, false] {
        let _ = SendMessageW(button, BM_CLICK, None, None);
        let mut message = MSG::default();
        if !PeekMessageW(
            &mut message,
            Some(owner),
            super::EXECUTE,
            super::EXECUTE,
            PM_REMOVE,
        )
        .as_bool()
            || message.wParam.0 != usize::from(catalog::MODE)
        {
            return Err(Error::new(E_FAIL, "Pinned edit strip did not route click"));
        }
        DispatchMessageW(&message);
        let canvas = state.canvas.borrow();
        if canvas.tool != Tool::Select || canvas.selection_enabled != enabled {
            return Err(Error::new(
                E_FAIL,
                "Pinned edit strip did not toggle selection",
            ));
        }
    }
    super::super::edit_mode::toolbar_mode(owner, state, false)?;
    println!("native-edit-strip-smoke: button=true selection=true silent=true");
    Ok(())
}

fn key(id: u16) -> HSTRING {
    HSTRING::from(format!("Otter.EditStrip.Choice.{id}"))
}
unsafe fn choice(panel: HWND, id: u16) -> usize {
    if id == catalog::MODE {
        return 0x53;
    }
    let stored = GetPropW(panel, &key(id)).0 as usize;
    let values = catalog::choices(id);
    if values.contains(&stored) {
        stored
    } else {
        values.first().copied().unwrap_or(id as usize)
    }
}
fn allowed(state: &State, available: Availability, command: usize) -> bool {
    if matches!(command, c if c == catalog::OCR as usize || c == catalog::TRANSLATE as usize) {
        return available.allows(0x53)
            && available.allows(super::super::context_menu::COLOR)
            && super::super::extension_overlay::available(
                state,
                command == catalog::TRANSLATE as usize,
            );
    }
    if matches!(command, 0x1007 | 0x1008 | 0x1022 | 0x1023 | 0x102e) {
        if !available.allows(0x53) || !available.allows(super::super::context_menu::COLOR) {
            return false;
        }
        let canvas = state.canvas.borrow();
        match command {
            0x1007 => canvas.editor.history_state().0,
            0x1008 => canvas.editor.history_state().1,
            0x1022 => !canvas.selected_indices().is_empty(),
            0x1023 => !canvas.editor.elements().is_empty(),
            _ => true,
        }
    } else {
        available.allows(command)
    }
}
pub(super) unsafe fn refresh(state: &State) {
    let panel = state.tool_panel.get();
    if !neuro_mini_ui::ready(panel) || super::popup::active(state) {
        return;
    }
    super::super::pin_settings::refresh(state);
    let Some(available) = Availability::capture(state) else {
        return;
    };
    let available = available.presentation();
    let Ok(canvas) = state.canvas.try_borrow() else {
        return;
    };
    let tool = canvas.tool;
    let selection_enabled = canvas.selection_enabled;
    let crop = canvas.crop_mode;
    drop(canvas);
    for item in catalog::items() {
        let neuro_mini_ui::Item::Button(button) = item else {
            continue;
        };
        let mut command = choice(panel, button.id);
        if let Some(entry) = super::super::tool_commands::TOOLS.iter().find(|entry| {
            button.id != catalog::MODE
                && entry.tool == tool
                && catalog::choices(button.id).contains(&entry.id)
        }) {
            command = entry.id;
            if GetPropW(panel, &key(button.id)).0 as usize != command {
                let _ = SetPropW(panel, &key(button.id), Some(HANDLE(command as *mut _)));
            }
        }
        let active = (matches!(button.id, catalog::OCR | catalog::TRANSLATE)
            && state
                .canvas
                .borrow()
                .extension_views
                .as_ref()
                .is_some_and(|v| {
                    v.active(
                        &state.canvas.borrow(),
                        state.resolved_display.borrow().as_ref(),
                        button.id == catalog::TRANSLATE,
                    )
                }))
            || (button.id == catalog::SETTINGS && super::super::pin_settings::is_open(state))
            || !crop
                && if button.id == catalog::MODE {
                    tool == crate::editor::Tool::Select && selection_enabled
                } else {
                    super::super::tool_commands::TOOLS
                        .iter()
                        .any(|entry| entry.id == command && entry.tool == tool)
                }
            || (command == 0x102e && crop);
        let enabled = if button.id == catalog::SETTINGS {
            available.allows(super::super::context_menu::OPEN)
        } else {
            allowed(state, available, command)
        };
        let label = if button.id == catalog::MODE {
            if tool == crate::editor::Tool::Select && !selection_enabled {
                "选择（静默）"
            } else {
                "选择"
            }
        } else {
            super::super::tool_commands::TOOLS
                .iter()
                .find(|entry| entry.id == command)
                .map_or(button.label, |entry| {
                    entry.label.split('\t').next().unwrap_or(entry.label)
                })
        };
        if let Err(error) = neuro_mini_ui::update(
            panel,
            button.id,
            catalog::icon(command),
            active,
            enabled,
            if button.id == catalog::SETTINGS {
                enabled
            } else {
                catalog::choices(button.id)
                    .iter()
                    .any(|command| allowed(state, available, *command))
            },
            label,
        ) {
            eprintln!("native-edit-strip-refresh: {error}");
        }
    }
    state.tool_panel_tool.set(Some(tool));
    state
        .tool_panel_style
        .set(Some(state.canvas.borrow().editor.style()));
    super::super::edit_properties::refresh(state);
    if let Err(error) = super::super::edit_extension_toolbar::refresh(state) {
        eprintln!("native-extension-toolbar-refresh: {error}");
    }
    if let Some(owner) = neuro_mini_ui::owner(panel) {
        if let Err(error) = place(owner, state, None) {
            eprintln!("native-edit-strip-place: {error}");
        }
    }
}
pub(super) unsafe fn close(owner: HWND, state: &State) {
    if state.lifetime.owns(owner)
        && !state.session_end.paused()
        && !state.group_locked.get()
        && !state.group_hidden.get()
        && super::super::capture_input::exclusive_available(state)
    {
        if let Err(error) = super::super::edit_mode::toolbar_mode(owner, state, false) {
            eprintln!("native-edit-strip-close: {error}");
            return;
        }
        let _ = ShowWindow(state.tool_panel.get(), SW_HIDE);
        let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(owner));
    }
}
pub(super) unsafe fn execute(owner: HWND, state: &State, id: usize, dropdown: bool) {
    if super::super::edit_extension_toolbar::handles(id) {
        if let Err(error) = super::super::edit_extension_toolbar::execute(owner, state, id) {
            eprintln!("native-extension-toolbar-command: {error}");
        }
        refresh(state);
        return;
    }
    let panel = state.tool_panel.get();
    if !state.lifetime.owns(owner)
        || state.session_end.paused()
        || state.group_locked.get()
        || state.group_hidden.get()
        || !super::super::capture_input::exclusive_available(state)
    {
        return;
    }
    let Some(available) = Availability::capture(state) else {
        return;
    };
    let id = id as u16;
    if !(if id == catalog::SETTINGS {
        available.allows(super::super::context_menu::OPEN)
    } else {
        available.allows(0x53)
    }) {
        return;
    }
    if !catalog::items()
        .iter()
        .any(|item| matches!(item,neuro_mini_ui::Item::Button(button) if button.id==id))
    {
        return;
    }
    if id == catalog::SETTINGS {
        if let Err(error) = super::super::pin_settings::toggle(owner, state) {
            eprintln!("native-pin-settings-open: {error}");
        }
        refresh(state);
        return;
    }
    if id == catalog::MODE {
        // Do not consume old dropdown requests; primary buttons directly toggle selection admission without entering modal menus.
        if !dropdown {
            if let Err(error) = super::super::edit_mode::toggle(owner, state) {
                eprintln!("native-edit-selection-toggle: {error}");
            }
        }
        refresh(state);
        return;
    }
    let command = if dropdown {
        match super::popup::preserve(state, || menus::show(owner, state, id)) {
            Ok(command) => command,
            Err(error) => {
                eprintln!("native-edit-strip-menu: {error}");
                return;
            }
        }
    } else {
        choice(panel, id)
    };
    let Some(available) = Availability::capture(state) else {
        return;
    };
    if command == 0 || !allowed(state, available, command) {
        return;
    }
    if catalog::choices(id).contains(&command) {
        let _ = SetPropW(panel, &key(id), Some(HANDLE(command as *mut _)));
    }
    // Baseline menus only remember rasterization scope; a later short primary-button press performs this destructive edit.
    if dropdown && id == catalog::RASTER {
        refresh(state);
        return;
    }
    let result = if matches!(id, catalog::OCR | catalog::TRANSLATE) {
        super::super::edit_mode::execute(owner, state, true).and_then(|()| {
            super::super::extension_overlay::toggle(owner, state, id == catalog::TRANSLATE)
        })
    } else {
        super::super::context_menu::execute(owner, state, command)
    };
    if let Err(error) = result {
        eprintln!("native-edit-strip-command: {error}");
    }
    refresh(state);
}
pub(in crate::desktop) unsafe fn follow(owner: HWND, state: &State, message: u32) {
    let panel = state.tool_panel.get();
    if neuro_mini_ui::is_toolbar(panel)
        && IsWindowVisible(panel).as_bool()
        && matches!(message, WM_WINDOWPOSCHANGED | WM_DPICHANGED)
    {
        if let Err(error) = place(owner, state, None) {
            eprintln!("native-edit-strip-follow: {error}");
        }
        super::super::pin_settings::follow(owner, state);
    }
}
