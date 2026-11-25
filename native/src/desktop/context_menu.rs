//! Build business-menu snapshots on demand; owner-drawn hosts reenter message loops, so hold no Canvas borrow across calls.
use super::tool_commands::tool_for_command;
use super::{application as app, State};
use crate::editor::Tool;
#[cfg(test)]
use windows::core::w;
#[path = "context_menu_pin.rs"]
mod pin;
#[path = "context_menu_shapes.rs"]
mod shapes;
use windows::{
    core::Result,
    Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*},
};

pub(super) struct Menu(pub(super) HMENU);
impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyMenu(self.0);
        }
    }
}

pub(super) const COLOR: usize = 0x1001;
pub(super) const FILL_COLOR: usize = 0x1002;
pub(super) const WIDTH_DOWN: usize = 0x1003;
pub(super) const WIDTH_UP: usize = 0x1004;
pub(super) const RADIUS_DOWN: usize = 0x1005;
pub(super) const RADIUS_UP: usize = 0x1006;
pub(super) const UNDO: usize = 0x1007;
pub(super) const REDO: usize = 0x1008;
pub(super) const EXPORT: usize = 0x1009;
pub(super) const OPEN: usize = 0x100A;
const REMOVE_LAST: usize = 0x100B;
pub(super) const APPLY_STYLE: usize = 0x100C;
const ENLARGE: usize = 0x100D;
const SHRINK: usize = 0x100E;
const ROTATE_LEFT: usize = 0x100F;
const ROTATE_RIGHT: usize = 0x1010;
const WIDEN: usize = 0x1011;
const NARROW: usize = 0x1012;
const TALLER: usize = 0x1013;
const SHORTER: usize = 0x1014;
const DUPLICATE: usize = 0x1015;
const ANGLE_SNAP: usize = 0x1016;
const LINE_ARROW: usize = 0x1017;
const BRUSH_SMOOTHING: usize = 0x1018;
const EDIT_TEXT: usize = 0x101E;
const SHAPE_LOCK: usize = 0x1019;
const SHAPE_STEP: usize = 0x101A;
const SHAPE_RATIO: usize = 0x101B;
const SHAPE_CUSTOM_RATIO: usize = 0x101C;
const SHAPE_CUSTOM_STEP: usize = 0x101D;

use super::popup_menu::target::MenuTarget;

pub(super) struct Pending {
    pub origin: crate::workspace_id::UnitId,
    pub target: MenuTarget,
    pub point: POINT,
}

pub(super) unsafe fn show(hwnd: HWND, state: &State, position: LPARAM) -> Result<()> {
    super::menu_transition::prepare(hwnd, state)?;
    let _ = InvalidateRect(Some(hwnd), None, false);
    let target = MenuTarget::capture(&state.canvas.borrow());
    let mut point = POINT {
        x: position.0 as i16 as i32,
        y: (position.0 >> 16) as i16 as i32,
    };
    if position.0 == -1 {
        point = POINT { x: 16, y: 16 };
        if !ClientToScreen(hwnd, &mut point).as_bool() {
            return Err(windows::core::Error::from_win32());
        }
    }
    let mut view = state.frozen_library.borrow_mut();
    if state.file_dialog.get() || view.menu.is_some() {
        return Ok(());
    }
    // Callbacks deliver only open requests; Application must own the menu loop to advance original deletion transactions.
    view.menu = Some(Pending {
        origin: state.unit_id.clone(),
        target,
        point,
    });
    if let Err(error) = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) {
        view.menu = None;
        return Err(error);
    }
    Ok(())
}

pub(super) unsafe fn build(state: &State) -> Result<Menu> {
    let menu = pin::build(state)?;
    super::group_lock::restrict_menu(menu.0, state);
    Ok(menu)
}

/// Shared execution entry for immediate UI commands; delayed UI must validate target snapshots before calling.
pub(super) unsafe fn execute(hwnd: HWND, state: &State, command: usize) -> Result<()> {
    if command == 0 || !IsWindow(Some(hwnd)).as_bool() {
        return Ok(());
    }
    if super::render_commands::handles(command) {
        return super::render_commands::execute(hwnd, state, command);
    }
    if state.canvas.borrow().minified.is_some()
        && (((COLOR..=EDIT_TEXT).contains(&command) && !matches!(command, EXPORT | OPEN))
            || tool_for_command(command).is_some())
    {
        return Ok(());
    }
    if state.group_locked.get() && !super::group_lock::allows(command) {
        return Ok(());
    }
    super::menu_transition::prepare(hwnd, state)?;
    match command {
        command if super::group_menu::handles(command) => {
            super::group_menu::execute(hwnd, state, command)?
        }
        command if app::handles(command) => app::execute(hwnd, state, command)?,
        super::execution_cancel::COMMAND => super::execution_cancel::stop(hwnd, state)?,
        super::capability_catalog::COMMAND => super::capability_catalog::request(hwnd, state)?,
        super::execution_start::COMMAND => super::execution_start::request(hwnd, state)?,
        super::sticker_propagation::COMMAND => super::sticker_propagation::toggle(hwnd, state)?,
        command if super::execution_settings::handles(command) => {
            super::execution_settings::toggle(hwnd, state, command)?
        }
        super::capability_parameters::COMMAND => super::capability_parameters::show(hwnd, state)?,
        super::blur_default::COMMAND => super::blur_default::show(hwnd, state),
        super::blur_numeric::COMMAND => super::blur_numeric::show(hwnd, state),
        super::global_capture::COMMAND => super::global_capture::toggle(hwnd, state),
        super::tray::COMMAND => super::tray::hide(hwnd, state)?,
        super::capture_protection::COMMAND => super::capture_protection::toggle(hwnd, state),
        super::capture_input::info::COMMAND => super::capture_input::info::show(hwnd, state),
        super::export_job::original::COMMAND => super::export_job::original::start(hwnd, state),
        super::capture_input::COMMAND
        | super::capture_input::FULL_SCREEN
        | super::capture_input::NEW_PIN
        | super::capture_input::NEW_FULL_PIN
        | super::capture_input::LONG_RECORDING
        | super::capture_input::LONG_VERTICAL
        | super::capture_input::LONG_HORIZONTAL => {
            super::capture_input::start(hwnd, state, command)
        }
        super::numeric_style::WIDTH
        | super::numeric_style::OPACITY
        | super::numeric_style::MOSAIC => super::numeric_style::show(hwnd, state, command),
        super::paste_input::COMMAND => super::paste_input::start(hwnd, state),
        super::paste_input::REPLACE => super::paste_input::start_with(hwnd, state, false),
        super::copy_job::COMMAND => super::copy_job::start(hwnd, state),
        super::flip_input::HORIZONTAL | super::flip_input::VERTICAL => {
            super::flip_input::start(hwnd, state, command)
        }
        super::crop_input::START | super::crop_input::RESET => {
            super::crop_input::command(hwnd, state, command)
        }
        0x47 => super::blur_input::start(hwnd, state),
        EDIT_TEXT => super::text_input::edit_selected(hwnd, state),
        super::text_font::COMMAND => super::text_font::show(hwnd, state),
        super::serial_input::COMMAND => super::serial_input::show(hwnd, state),
        SHAPE_CUSTOM_RATIO | SHAPE_CUSTOM_STEP => {
            if !shapes::edit(hwnd, state, command == SHAPE_CUSTOM_RATIO) {
                return Ok(());
            }
        }
        SHAPE_LOCK | SHAPE_STEP | SHAPE_RATIO => {
            let mut canvas = state.canvas.borrow_mut();
            let tool = canvas.tool;
            let shapes = &mut canvas.tool_profiles.shapes;
            if crate::shape_options::ShapeOptions::supports(tool) {
                match command {
                    SHAPE_LOCK => shapes.lock_aspect = !shapes.lock_aspect,
                    SHAPE_STEP => shapes.cycle_step(),
                    SHAPE_RATIO if crate::shape_options::ShapeOptions::supports_ratio(tool) => {
                        shapes.cycle_ratio()
                    }
                    _ => {}
                }
            }
        }
        BRUSH_SMOOTHING => {
            let mut canvas = state.canvas.borrow_mut();
            let tool = canvas.tool;
            canvas.tool_profiles.cycle_smoothing(tool);
        }
        LINE_ARROW => {
            let mut canvas = state.canvas.borrow_mut();
            if matches!(canvas.tool, Tool::Line | Tool::Bezier) {
                let style = canvas.editor.style();
                canvas.editor.set_style(style.with_arrow(!style.arrow()));
            }
        }
        ANGLE_SNAP => {
            let mut canvas = state.canvas.borrow_mut();
            let tool = canvas.tool;
            canvas.tool_profiles.toggle_angle_snap(tool);
        }
        command if super::style_input::execute(hwnd, state, command)? => (),
        UNDO | REDO => {
            super::pin_settings_history::step(hwnd, state, command == REDO)?;
        }
        super::erase_input::MODE_COMMAND => super::erase_input::toggle_mode(state),
        super::transform_mode::COMMAND..=super::transform_mode::SCALE_Y => {
            super::transform_mode::dispatch(state, command)
        }
        super::rasterize_input::SELECTED => super::rasterize_input::start(hwnd, state, true),
        super::rasterize_input::ALL => super::rasterize_input::start(hwnd, state, false),
        EXPORT => super::export_job::start(hwnd, state),
        OPEN => super::open_image::start(hwnd, state),
        super::open_image::REPLACE => super::open_image::start_destination(hwnd, state, false),
        DUPLICATE => {
            state.canvas.borrow_mut().duplicate_selected();
        }
        WIDEN | NARROW | TALLER | SHORTER => {
            super::resize_input::axis_step(
                hwnd,
                state,
                matches!(command, WIDEN | NARROW),
                matches!(command, WIDEN | TALLER),
            );
        }
        ROTATE_LEFT | ROTATE_RIGHT => {
            let degrees = if command == ROTATE_LEFT { -15.0 } else { 15.0 };
            state.canvas.borrow_mut().rotate_selected(degrees);
        }
        ENLARGE => super::resize_input::uniform(hwnd, state, 1.1),
        SHRINK => super::resize_input::uniform(hwnd, state, 1.0 / 1.1),
        REMOVE_LAST => {
            state.canvas.borrow_mut().delete_target();
        }
        APPLY_STYLE => {
            state.canvas.borrow_mut().restyle_target();
        }
        key => {
            if let Some(tool) = tool_for_command(key) {
                state.canvas.borrow_mut().select_tool(tool);
            }
        }
    }
    super::menu_transition::finish(hwnd, state)
}

unsafe fn label(menu: HMENU, id: usize, text: &str, flags: MENU_ITEM_FLAGS) -> Result<()> {
    let text = windows::core::HSTRING::from(text);
    AppendMenuW(menu, flags, id, windows::core::PCWSTR(text.as_ptr()))
}

#[cfg(test)]
#[path = "context_menu_tests.rs"]
mod tests;
