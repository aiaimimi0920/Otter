//! 选择开关与静默命中；变换只由当前标注操作柄/修饰键发起，不另存变换模式。
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

/// 空白/静默命中放行给系统拖窗；当前新建标注的操作柄仍归标注输入。
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
            && c.selected_indices().is_empty())
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
                || (self.tool == Tool::Select && self.selected_indices().is_empty()))
    }
    pub(super) fn can_select_annotation(&self, index: usize) -> bool {
        self.selection_enabled
            || self.selected_index() == Some(index)
            || self
                .selected_group()
                .is_some_and(|indices| indices.contains(&index))
    }
    /// 静默只禁止重新点选；刚创建的标注保留一次当前选择，仍能直接调整。
    pub(super) fn select_created_annotation(&mut self, index: usize) {
        if self.pinned && (!self.selection_enabled || self.tool == Tool::Bezier) {
            let enabled = self.selection_enabled;
            self.select_tool(Tool::Select);
            self.selection_enabled = enabled;
            self.selected = Some((index, self.editor.revision()));
        }
    }
    /// 修饰键只改变本次手势，不切换工具栏模式；空白处仍沿原选择/拖窗规则处理。
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
            Ok(Some(index)) if !self.selected_indices().contains(&index) => {
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
        if self.selected_indices().is_empty() {
            return false;
        }
        if control {
            self.begin_rotation_at(screen)
        } else {
            let indices = self.selected_indices();
            if indices.len() > 1 {
                self.group_preview = indices
                    .iter()
                    .map(|index| (*index, self.editor.elements()[*index].clone()))
                    .collect();
            } else {
                let stroke = &self.editor.elements()[indices[0]];
                self.selection_effect_preview =
                    matches!(stroke.tool, Tool::Blur | Tool::Mosaic).then(|| stroke.clone());
            }
            self.selection_drag = Some((point, Point::default()));
            true
        }
    }
}
#[cfg(test)]
#[path = "edit_mode_tests.rs"]
mod tests;
