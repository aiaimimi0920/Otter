//! Selection toggle and silent hit testing; only current annotation handles/modifiers initiate transforms, without separate transform-mode state.
use super::{canvas::Canvas, State};
use crate::{editor::Tool, viewport::Point};
use windows::{
    core::Result,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
pub(super) unsafe fn execute(hwnd: HWND, state: &State, enabled: bool) -> Result<()> {
    super::menu_transition::prepare(hwnd, state)?;
    {
        let mut canvas = state.canvas.borrow_mut();
        if canvas.tool != Tool::Select || canvas.selection_enabled != enabled {
            canvas.select_tool(Tool::Select);
        }
        canvas.selection_enabled = enabled;
        canvas.crop_mode = false;
    }
    super::menu_transition::finish(hwnd, state)
}
pub(super) unsafe fn toggle(hwnd: HWND, state: &State) -> Result<()> {
    let enabled = {
        let canvas = state.canvas.borrow();
        canvas.tool != Tool::Select || canvas.crop_mode || !canvas.selection_enabled
    };
    execute(hwnd, state, enabled)
}
pub(super) unsafe fn toolbar_mode(hwnd: HWND, state: &State, editing: bool) -> Result<()> {
    super::menu_transition::prepare(hwnd, state)?;
    state
        .canvas
        .borrow_mut()
        .select_tool(if editing { Tool::Select } else { Tool::Pan });
    super::menu_transition::finish(hwnd, state)
}
pub(super) unsafe fn pointer_down(c: &mut Canvas, point: Point, time: f64) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    c.pointer_down_modified_at(
        point,
        time,
        GetKeyState(VK_CONTROL.0 as i32) < 0,
        GetKeyState(VK_MENU.0 as i32) < 0,
    )
}

/// Empty/silent hits pass through to system window dragging; handles on newly created annotations still belong to annotation input.
pub(super) unsafe fn hit_test(hwnd: HWND, state: &State, lp: LPARAM) -> i32 {
    use windows::Win32::{
        Graphics::Gdi::ScreenToClient,
        UI::{HiDpi::GetDpiForWindow, Input::KeyboardAndMouse::*},
    };
    if super::extension_input::screen_hit(hwnd, state, lp) {
        return HTCLIENT as i32;
    }
    let c = state.canvas.borrow();
    if c.tool == Tool::Pan
        || (c.tool == Tool::Select
            && !c.selection_enabled
            && !c.crop_mode
            && !c.blocks_wheel()
            && c.selected_indices_ref().is_empty())
    {
        return HTCAPTION as i32;
    }
    if !c.pin_move_ready()
        || [VK_CONTROL, VK_SHIFT, VK_MENU]
            .iter()
            .any(|k| GetKeyState(k.0 as i32) < 0)
    {
        return HTCLIENT as i32;
    }
    let mut point = POINT {
        x: lp.0 as i16 as i32,
        y: (lp.0 >> 16) as i16 as i32,
    };
    if !ScreenToClient(hwnd, &mut point).as_bool() {
        return HTCLIENT as i32;
    }
    let dpi = GetDpiForWindow(hwnd).max(96) as f32;
    let point = Point {
        x: point.x as f32 * 96.0 / dpi,
        y: point.y as f32 * 96.0 / dpi,
    };
    if let Some(s) = c.selected_index().and_then(|i| c.editor.elements().get(i)) {
        let p = crate::rotation::rotate(
            c.viewport.image_point(point),
            crate::rotation::center(&s.points),
            -s.rotation,
        );
        if let Some((s0, s1)) = s.lens_source_box() {
            let pad = 6.0 / c.viewport.scale;
            if p.x >= s0.x - pad && p.x <= s1.x + pad && p.y >= s0.y - pad && p.y <= s1.y + pad {
                return HTCLIENT as i32;
            }
        }
        if let Some(handles) = s
            .text_handles()
            .or_else(|| s.serial_handles())
            .or_else(|| s.lens_handles())
        {
            if handles
                .iter()
                .any(|h| (h.x - p.x).hypot(h.y - p.y) <= 6.0 / c.viewport.scale)
            {
                return HTCLIENT as i32;
            }
        }
    }
    match c.image_point(point, false).map(|point| {
        super::selection_hit::pick(&c.factory, &c.editor, point, 5.0 / c.viewport.scale)
    }) {
        None | Some(Ok(None)) => HTCAPTION as i32,
        Some(Ok(Some(_))) => HTCLIENT as i32,
        Some(Err(error)) => {
            eprintln!("native-edit-mode-hit: {error}");
            HTCLIENT as i32
        }
    }
}
pub(super) unsafe fn shortcut(
    hwnd: HWND,
    state: &State,
    message: u32,
    key: usize,
    repeat: bool,
) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    if message != WM_KEYDOWN
        || !IsWindowVisible(state.tool_panel.get()).as_bool()
        || !neuro_mini_ui::is_toolbar(state.tool_panel.get())
        || [VK_CONTROL, VK_SHIFT, VK_MENU]
            .iter()
            .any(|key| GetKeyState(key.0 as i32) < 0)
    {
        return false;
    }
    if key != 0x51 {
        return false;
    }
    if !repeat
        && super::tool_panel_availability::Availability::capture(state)
            .is_some_and(|a| a.allows(0x53))
        && !state.session_end.paused()
        && !state.group_hidden.get()
        && super::capture_input::exclusive_available(state)
    {
        if let Err(error) = toggle(hwnd, state) {
            eprintln!("native-edit-mode: {error}");
        }
    }
    true
}
impl Canvas {
    pub(super) fn pin_move_ready(&self) -> bool {
        self.pinned
            && !self.crop_mode
            && !self.blocks_wheel()
            && (self.tool == Tool::Pan
                || (self.tool == Tool::Select && self.selected_indices_ref().is_empty()))
    }
    pub(super) fn can_select_annotation(&self, index: usize) -> bool {
        self.selection_enabled
            || self.selected_index() == Some(index)
            || self
                .selected_group()
                .is_some_and(|indices| indices.contains(&index))
    }
    /// Silent mode only forbids reselection; newly created annotations retain their current selection once and remain directly adjustable.
    pub(super) fn select_created_annotation(&mut self, index: usize) {
        if self
            .editor
            .elements()
            .get(index)
            .is_some_and(|s| s.tool == Tool::Magnifier)
        {
            self.selected = Some((index, self.editor.revision()));
            return;
        }
        if self.pinned && (!self.selection_enabled || self.tool == Tool::Bezier) {
            let enabled = self.selection_enabled;
            self.select_tool(Tool::Select);
            self.selection_enabled = enabled;
            self.selected = Some((index, self.editor.revision()));
        }
    }
    pub(super) fn begin_magnifier_reselect(&mut self, point: Point) -> bool {
        if self.selected_index().is_some() && self.begin_path_handle(point) {
            return true;
        }
        if let Some(hit_point) = self.image_point(point, false) {
            if let Ok(Some(index)) = unsafe {
                super::selection_hit::pick(
                    &self.factory,
                    &self.editor,
                    hit_point,
                    5.0 / self.viewport.scale,
                )
            } {
                if self.editor.elements()[index].tool == Tool::Magnifier {
                    self.selected = Some((index, self.editor.revision()));
                    self.multi_selection = None;
                    let detached = self.editor.elements()[index]
                        .lens
                        .is_some_and(|l| l.offset != Point::default());
                    if detached && self.begin_path_handle(point) {
                        return true;
                    }
                    self.selection_effect_preview = Some(self.editor.elements()[index].clone());
                    self.selection_drag = Some((hit_point, Point::default()));
                    return true;
                }
            }
        }
        self.selected = None;
        self.multi_selection = None;
        false
    }
    /// Modifiers affect only this gesture, not toolbar modes; empty space retains original selection/window-dragging rules.
    pub(super) fn begin_quick_transform(
        &mut self,
        screen: Point,
        control: bool,
        alt: bool,
    ) -> bool {
        if !control && !alt {
            return false;
        }
        let Some(point) = self.image_point(screen, false) else {
            return false;
        };
        match unsafe {
            super::selection_hit::pick(
                &self.factory,
                &self.editor,
                point,
                5.0 / self.viewport.scale,
            )
        } {
            Ok(Some(index)) if !self.can_select_annotation(index) => return false,
            Ok(Some(index)) if !self.selected_indices_ref().contains(&index) => {
                self.selected = Some((index, self.editor.revision()));
                self.multi_selection = None;
            }
            Ok(None) => return false,
            Ok(_) => (),
            Err(error) => {
                eprintln!("native-edit-mode-pick: {error}");
                return false;
            }
        }
        if control {
            return self.begin_rotation_at(screen);
        }
        match self.selected_indices_ref() {
            [] => false,
            [index] => {
                let stroke = &self.editor.elements()[*index];
                self.selection_effect_preview =
                    matches!(stroke.tool, Tool::Blur | Tool::Mosaic | Tool::Magnifier)
                        .then(|| stroke.clone());
                self.selection_drag = Some((point, Point::default()));
                true
            }
            _ => self.begin_group_translation(point),
        }
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "edit_mode_tests.rs"]
mod tests;
