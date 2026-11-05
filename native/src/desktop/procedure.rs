//! 消息转发不得持有跨重入 Win32 调用的 RefCell 借用；panic 不跨越 ABI。
use super::State;
use crate::editor::Tool;
use crate::viewport::Point;
use windows::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    UI::{HiDpi::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

#[path = "procedure_paint.rs"]
mod paint;
#[path = "paint_scope.rs"]
mod paint_scope;
#[path = "procedure_render_recovery.rs"]
pub(super) mod render_recovery;
#[path = "procedure_software_present.rs"]
mod software_present;
#[path = "procedure_tasks.rs"]
mod tasks;

pub unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match std::panic::catch_unwind(|| {
        let result = dispatch(hwnd, msg, wp, lp);
        if super::host_lifetime::owns(hwnd)
            && !(*(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State))
                .session_end
                .paused()
        {
            super::tool_panel::refresh(hwnd);
        }
        result
    }) {
        Ok(value) => value,
        Err(_) => {
            PostQuitMessage(1);
            LRESULT(0)
        }
    }
}

unsafe fn dispatch(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        if !super::host_lifetime::attach(hwnd, lp) {
            return LRESULT(0);
        }
        // 默认创建过程保存初始窗口标题；绑定成功不等于可以跳过系统初始化。
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State;
    if pointer.is_null() || !(*pointer).lifetime.owns(hwnd) {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let state = &*pointer;
    super::application::chrome::geometry(&state.chrome, hwnd, msg);
    if let Some(result) = super::session_end::dispatch(hwnd, state, msg, wp, lp) {
        return result;
    }
    if matches!(msg, WM_DESTROY | WM_NCDESTROY) {
        render_recovery::close(hwnd);
    }
    if let Some(result) = super::window_geometry_wake::dispatch(hwnd, &state.geometry_wake, msg, lp)
    {
        return result;
    }
    if let Some(result) = super::screen_color::dispatch(hwnd, state, msg, wp) {
        return result;
    }
    if let Some(result) = super::global_capture::dispatch(hwnd, state, msg, wp, lp) {
        return result;
    }
    if let Some(result) = super::tray::dispatch(hwnd, state, msg, wp, lp) {
        return result;
    }
    if let Some(result) = super::group_lock::dispatch(state, msg, wp) {
        return result;
    }
    if let Some(result) = super::drag_input::dispatch(hwnd, state, msg, wp, lp) {
        return result;
    }
    if let Some(result) = super::pin_window::dispatch_minified_input(state, msg, wp) {
        return result;
    }
    if let Some(result) = super::tool_panel::dispatch(hwnd, state, msg, wp, lp) {
        return result;
    }
    if let Some(result) =
        super::document_window::parameter_panel::dispatch(hwnd, state, msg, wp, lp)
    {
        return result;
    }
    if let Some(result) = super::clipboard_input::dispatch(hwnd, state, msg, wp) {
        return result;
    }
    if let Some(result) = super::node_menu::art_picker::dispatch(hwnd, state, msg, wp, lp) {
        return result;
    }
    if let Some(result) = super::host_lifetime::dispatch(hwnd, state, msg, wp, lp) {
        return result;
    }
    if let Some(result) = super::pin_window::dispatch_minified(hwnd, state, msg, lp) {
        return result;
    }
    match msg {
        WM_DROPFILES => {
            super::file_drop::receive(hwnd, state, wp);
            LRESULT(0)
        }
        super::window_priority::PRIORITY_CHANGED => {
            super::window_priority::acknowledge(hwnd);
            if let Err(error) = super::priority_input::refresh(hwnd, state, false) {
                eprintln!("native-priority: {error}");
            }
            LRESULT(0)
        }
        WM_NCLBUTTONDOWN => {
            let _ = super::priority_input::refresh(hwnd, state, true);
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_NCHITTEST if state.canvas.borrow().pinned => {
            // 平移工具下使用系统窗口拖动；编辑工具仍接收客户区指针。
            let hit = if state.canvas.borrow().tool == Tool::Pan {
                HTCAPTION
            } else {
                HTCLIENT
            };
            LRESULT(hit as isize)
        }
        WM_PAINT => paint::paint(hwnd, state),
        WM_ERASEBKGND => LRESULT(1),
        WM_SIZE => {
            if wp.0 as u32 == SIZE_MINIMIZED {
                state.canvas.borrow_mut().discard_device();
            } else if state.canvas.borrow().pinned {
                super::pin_window::fit(hwnd, state);
            }
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }
        WM_DPICHANGED => {
            state.canvas.borrow_mut().cancel_gesture();
            if GetCapture() == hwnd {
                let _ = ReleaseCapture();
            }
            let rect = &*(lp.0 as *const RECT);
            let _ = SetWindowPos(
                hwnd,
                None,
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            if state.canvas.borrow().tool == Tool::Text {
                super::text_input::insert(hwnd, state, logical_point(hwnd, lp));
                return LRESULT(0);
            }
            if !super::display_edit::prepare(state) {
                return LRESULT(0);
            }
            if state.canvas.borrow().tool == Tool::Select
                && !state.canvas.borrow().crop_mode
                && GetKeyState(VK_SHIFT.0 as i32) < 0
            {
                let changed = state
                    .canvas
                    .borrow_mut()
                    .toggle_selection_at(logical_point(hwnd, lp));
                if changed {
                    if state.canvas.borrow().marquee.is_some() {
                        SetCapture(hwnd);
                    }
                    let _ = InvalidateRect(Some(hwnd), None, false);
                }
                return LRESULT(0);
            }
            let message_time = GetMessageTime() as u32;
            let _ = super::priority_input::refresh(hwnd, state, true);
            let started = {
                let mut canvas = state.canvas.borrow_mut();
                let time = canvas.message_clock.start(message_time);
                canvas.pointer_down_at(logical_point(hwnd, lp), time)
            };
            if started {
                SetCapture(hwnd);
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            LRESULT(0)
        }
        WM_KEYDOWN if wp.0 == VK_F2.0 as usize => {
            super::text_input::edit_selected(hwnd, state);
            LRESULT(0)
        }
        WM_LBUTTONDBLCLK => {
            if state.canvas.borrow().crop_mode {
                return dispatch(hwnd, WM_LBUTTONDOWN, wp, lp);
            }
            let tool = state.canvas.borrow().tool;
            if matches!(tool, Tool::Select | Tool::Text) {
                super::text_input::edit_at(hwnd, state, logical_point(hwnd, lp));
            } else {
                // CS_DBLCLKS 将第二次按下变为双击消息，其他绘图工具仍需正常开始手势。
                return dispatch(hwnd, WM_LBUTTONDOWN, wp, lp);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let point = logical_point(hwnd, lp);
            let changed = {
                let mut canvas = state.canvas.borrow_mut();
                let time = canvas.message_clock.advance(GetMessageTime() as u32);
                canvas.pointer_move_at(
                    point,
                    GetKeyState(VK_SHIFT.0 as i32) < 0,
                    GetKeyState(VK_CONTROL.0 as i32) < 0,
                    time,
                )
            };
            if changed {
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let changed = {
                let mut canvas = state.canvas.borrow_mut();
                let time = canvas.message_clock.advance(GetMessageTime() as u32);
                canvas.pointer_up_at(
                    logical_point(hwnd, lp),
                    GetKeyState(VK_SHIFT.0 as i32) < 0,
                    GetKeyState(VK_CONTROL.0 as i32) < 0,
                    time,
                )
            };
            super::erase_input::submit(hwnd, state);
            super::resize_input::submit_pending(hwnd, state);
            super::crop_apply::submit(hwnd, state);
            if let Err(error) = super::crop_window::sync(hwnd, state) {
                eprintln!("native-crop-window: {error}");
            }
            if GetCapture() == hwnd {
                let _ = ReleaseCapture();
            }
            if changed {
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            LRESULT(0)
        }
        WM_CANCELMODE | WM_KILLFOCUS => {
            let changed = state.canvas.borrow_mut().cancel_gesture();
            if GetCapture() == hwnd {
                let _ = ReleaseCapture();
            }
            if changed {
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            LRESULT(0)
        }
        WM_CAPTURECHANGED => {
            if state.canvas.borrow_mut().cancel_gesture() {
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            LRESULT(0)
        }
        WM_KEYDOWN | WM_KEYUP if matches!(wp.0 as u16, 0x10 | 0x11) => {
            let changed = state.canvas.borrow_mut().refresh_constraints(
                GetKeyState(VK_SHIFT.0 as i32) < 0,
                GetKeyState(VK_CONTROL.0 as i32) < 0,
            );
            if changed {
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            let control = GetKeyState(VK_CONTROL.0 as i32) < 0;
            let key = wp.0 as u32;
            if control && key == 0x48 {
                if lp.0 & (1 << 30) == 0 {
                    super::capture_history_ui::toggle(state);
                }
                return LRESULT(0);
            }
            if !control && key == 0x47 {
                super::blur_input::start(hwnd, state);
                return LRESULT(0);
            }
            if control && key == 0x4F {
                super::open_image::start(hwnd, state);
                return LRESULT(0);
            }
            if !control && matches!(key, 0x21 | 0x22) {
                super::style_input::adjust_sides(state, if key == 0x21 { 1 } else { -1 });
                return LRESULT(0);
            }
            if !control && key == 0x44 {
                super::style_input::cycle_dash(state);
                return LRESULT(0);
            }
            if !control && key == 0x46 {
                super::style_input::toggle_fill(state);
                return LRESULT(0);
            }
            if !control && key == 0x4F {
                super::style_input::cycle_opacity(state);
                return LRESULT(0);
            }
            if !control && key == 0x43 {
                super::style_input::choose_color(hwnd, state, GetKeyState(VK_SHIFT.0 as i32) < 0);
                return LRESULT(0);
            }
            if !control && matches!(key, 0xDB | 0xDD) {
                let increment = if key == 0xDD { 1.0 } else { -1.0 };
                if GetKeyState(VK_SHIFT.0 as i32) < 0 {
                    super::style_input::adjust_radius(hwnd, state, increment);
                } else {
                    super::style_input::adjust_width(state, increment);
                }
                return LRESULT(0);
            }
            if control && key == 0x53 {
                if GetCapture() == hwnd {
                    let _ = ReleaseCapture();
                }
                super::export_job::start(hwnd, state);
                let _ = InvalidateRect(Some(hwnd), None, false);
                return LRESULT(0);
            }
            if !control && matches!(key, 0xBB | 0xBD) {
                if GetCapture() == hwnd {
                    let _ = ReleaseCapture();
                }
                let factor = if key == 0xBB { 1.1 } else { 1.0 / 1.1 };
                super::resize_input::start(
                    hwnd,
                    state,
                    crate::viewport::Point {
                        x: factor,
                        y: factor,
                    },
                );
                return LRESULT(0);
            }
            // 已投递到画布的 Esc 始终取消，不能被仍按住的 Ctrl 屏蔽。
            let handled = key == 0x1B
                || if control {
                    matches!(key, 0x5A | 0x59 | 0x0D | 0x44)
                } else {
                    matches!(
                        key,
                        0x48 | 0xBC
                            | 0xBE
                            | 0xBB
                            | 0xBD
                            | 0x1B
                            | 0x2E
                            | 0x41
                            | 0x42
                            | 0x4C
                            | 0x50
                            | 0x51
                            | 0x52
                            | 0x53
                            | 0x54
                            | 0x4B
                            | 0x55
                            | 0x4A
                            | 0x58
                            | 0x4E
                            | 0x4D
                            | 0x45
                            | 0x56
                            | 0x57
                    )
                };
            if !handled {
                return DefWindowProcW(hwnd, msg, wp, lp);
            }
            let changed = {
                let mut canvas = state.canvas.borrow_mut();
                let changed = match (control, wp.0 as u32) {
                    (true, 0x5A) => canvas.cancel_gesture() || canvas.editor.undo(),
                    (true, 0x59) => canvas.cancel_gesture() || canvas.editor.redo(),
                    (true, 0x44) => canvas.duplicate_selected(),
                    (true, 0x0D) => canvas.restyle_target(),
                    (_, 0x1B) => {
                        super::copy_job::cancel(state);
                        state.blur_job.borrow_mut().cancel();
                        state.erase_job.borrow_mut().cancel();
                        super::rasterize_input::cancel(state);
                        state.resize_job.borrow().cancel();
                        state.flip_job.borrow().cancel();
                        super::crop_apply::cancel(state);
                        canvas.crop_pending = None;
                        canvas.clear_scale_display();
                        super::open_job::cancel(state);
                        canvas.escape_selection()
                    }
                    (false, 0x2E) => canvas.delete_target(),
                    (false, 0xBC) => canvas.rotate_selected(-15.0),
                    (false, 0xBE) => canvas.rotate_selected(15.0),
                    (false, key) => super::tool_commands::tool_for_command(key as usize)
                        .is_some_and(|tool| canvas.select_tool(tool)),
                    _ => false,
                };
                canvas.sync_raster_source();
                changed
            };
            if GetCapture() == hwnd {
                let _ = ReleaseCapture();
            }
            if changed {
                // 空命中区域可能没有 WM_PAINT；工具切换必须先恢复可交互区域。
                if let Err(error) = super::crop_window::sync(hwnd, state) {
                    eprintln!("native-crop-window: {error}");
                }
                if let Err(error) = super::hit_region::update(hwnd, state) {
                    *state.error.borrow_mut() = Some(error);
                    let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
                }
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            LRESULT(0)
        }
        WM_NCRBUTTONUP => {
            // 无边框钉图用 HTCAPTION 实现拖动；DefWindowProc 不会为它生成 WM_CONTEXTMENU。
            let pinned = state.canvas.borrow().pinned;
            if pinned && wp.0 == HTCAPTION as usize {
                // 释放 Canvas 借用后重入原菜单入口，保留模态/会话/拖出围栏及屏幕坐标。
                SendMessageW(
                    hwnd,
                    WM_CONTEXTMENU,
                    Some(WPARAM(hwnd.0 as usize)),
                    Some(lp),
                )
            } else {
                DefWindowProcW(hwnd, msg, wp, lp)
            }
        }
        WM_CONTEXTMENU => {
            if let Err(error) = super::context_menu::show(hwnd, state, lp) {
                eprintln!("native-menu: {error}");
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            if state.canvas.borrow().blocks_wheel() {
                return LRESULT(0);
            }
            if state.canvas.borrow().pinned {
                if GetKeyState(VK_CONTROL.0 as i32) < 0 {
                    super::pin_window::wheel(hwnd, state, wp, lp);
                }
                return LRESULT(0);
            }
            let mut point = POINT {
                x: lp.0 as i16 as i32,
                y: (lp.0 >> 16) as i16 as i32,
            };
            if ScreenToClient(hwnd, &mut point).as_bool() {
                let dpi = GetDpiForWindow(hwnd).max(1) as f32 / 96.0;
                let steps = (wp.0 >> 16) as i16 as f32 / 120.0;
                if state.canvas.borrow_mut().viewport.zoom_at(
                    Point {
                        x: point.x as f32 / dpi,
                        y: point.y as f32 / dpi,
                    },
                    steps,
                ) {
                    let _ = InvalidateRect(Some(hwnd), None, false);
                }
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_TIMER if tasks::complete(hwnd, state, wp.0) => LRESULT(0),
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe fn logical_point(hwnd: HWND, lp: LPARAM) -> Point {
    let scale = 96.0 / GetDpiForWindow(hwnd).max(1) as f32;
    Point {
        x: lp.0 as i16 as f32 * scale,
        y: (lp.0 >> 16) as i16 as f32 * scale,
    }
}
