//! Tile 的独立 HWND；不注册托盘/全局 hook，不激活其他窗口，六秒 watchdog 不等待 HTTP。
#[path = "tile_surface_action_window.rs"]
mod actions;
#[path = "tile_surface_keyboard_window.rs"]
mod keyboard;
#[path = "tile_surface_local_window.rs"]
mod local;
#[path = "tile_window_render.rs"]
mod render;
#[path = "tile_surface_scroll_window.rs"]
mod scroll;
#[path = "tile_surface_scrollbar_window.rs"]
mod scrollbar;
#[cfg(test)]
#[path = "tile_window_test_access.rs"]
mod test_access;
#[path = "tile_surface_text_window.rs"]
mod text;
use crate::tile_shared::diagnostics::Stage;
use crate::{
    tile_outputs::TileOutput,
    tile_shared::{Frame, Shared},
    tile_wire::Size,
};
use std::{
    cell::{Cell, RefCell},
    sync::{Arc, OnceLock},
    time::Instant,
};
use windows::{
    core::{w, Error, Result, HRESULT},
    Win32::{
        Foundation::*, Graphics::Gdi::*, System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    },
};
struct State {
    shared: Arc<Shared>,
    renderer: RefCell<Option<render::Renderer>>,
    closed: Cell<bool>,
    changed: Cell<bool>,
    frame: RefCell<Option<Arc<Frame>>>,
    painted: Cell<bool>,
    error: RefCell<Option<Error>>,
    marker: RefCell<Option<(Arc<crate::tile_identification::Marker>, u64)>>,
    displayed_marker: RefCell<Option<Arc<crate::tile_identification::Marker>>>,
    pressed: RefCell<actions::Gesture>,
    slider_preview: RefCell<Option<crate::tile_surface_slider::Drag>>,
    text: RefCell<Option<text::Owner>>,
    text_preview: RefCell<std::rc::Rc<Vec<crate::tile_surface_text::Preview>>>,
    text_click: RefCell<
        Option<(
            crate::tile_surface_wire::View,
            Arc<crate::tile_surface_edit::Edit>,
        )>,
    >,
    text_retired: Cell<bool>,
    text_dirty: Cell<bool>,
    keyboard: RefCell<keyboard::Keyboard>,
    keyboard_value: RefCell<Option<keyboard::Owner>>,
    keyboard_retired: Cell<bool>,
    local: RefCell<crate::tile_surface_local::Cache>,
    scrollbar: RefCell<scrollbar::Owner>,
}
pub(crate) struct Window {
    hwnd: HWND,
    state: Box<State>,
}
impl Window {
    pub fn new(shared: Arc<Shared>, output: &TileOutput) -> Result<Self> {
        unsafe {
            register()?;
            let state = Box::new(State {
                shared,
                renderer: RefCell::new(None),
                closed: Cell::new(false),
                changed: Cell::new(false),
                frame: RefCell::new(None),
                painted: Cell::new(false),
                error: RefCell::new(None),
                marker: RefCell::new(None),
                displayed_marker: RefCell::new(None),
                pressed: RefCell::new(Default::default()),
                slider_preview: RefCell::new(None),
                text: RefCell::new(None),
                text_preview: RefCell::new(Default::default()),
                text_click: RefCell::new(None),
                text_retired: Cell::new(false),
                text_dirty: Cell::new(false),
                keyboard: RefCell::new(Default::default()),
                keyboard_value: RefCell::new(None),
                keyboard_retired: Cell::new(false),
                local: RefCell::new(Default::default()),
                scrollbar: RefCell::new(Default::default()),
            });
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("Otter.Native.Tile"),
                w!("Otter Tile"),
                WS_POPUP,
                output.x,
                output.y,
                output.width as i32,
                output.height as i32,
                None,
                None,
                Some(HINSTANCE(GetModuleHandleW(None)?.0)),
                Some((&*state as *const State).cast()),
            )?;
            let window = Self { hwnd, state };
            *window.state.renderer.borrow_mut() = Some(render::Renderer::new(
                hwnd,
                Size {
                    width: output.width,
                    height: output.height,
                },
            )?);
            Ok(window)
        }
    }
    pub fn alive(&self) -> bool {
        !self.state.closed.get()
    }
    pub fn output_changed(&self) -> bool {
        self.state.changed.get()
    }
    pub fn tick(&self) -> Result<()> {
        if !self.alive() || self.output_changed() {
            return Ok(());
        }
        let (frame, marker, visibility) = {
            let mut s = self.state.shared.lock();
            s.expire(Instant::now());
            let marker = (!s.stopped && s.until.is_some_and(|v| Instant::now() < v))
                .then(|| s.identification.marker(Instant::now()))
                .flatten();
            (
                s.visible(Instant::now()).then(|| s.frame.clone()).flatten(),
                marker,
                s.visibility(Instant::now()),
            )
        };
        self.state.shared.trace.observe(visibility);
        let marker = marker.map(|m| {
            let second = m
                .until
                .saturating_duration_since(Instant::now())
                .as_millis()
                .div_ceil(1000) as u64;
            (m, second)
        });
        let marker_changed = match (self.state.marker.borrow().as_ref(), marker.as_ref()) {
            (Some((a, x)), Some((b, y))) => !Arc::ptr_eq(a, b) || x != y,
            (None, None) => false,
            _ => true,
        };
        *self.state.marker.borrow_mut() = marker;
        let changed = match (self.state.frame.borrow().as_ref(), frame.as_ref()) {
            (Some(a), Some(b)) => !Arc::ptr_eq(a, b),
            (None, None) => false,
            _ => true,
        };
        *self.state.frame.borrow_mut() = frame;
        local::tick(&self.state);
        let frozen = self.state.shared.lock().frozen;
        if frozen || self.state.frame.borrow().is_none() && !actions::slider::waiting(&self.state) {
            actions::slider::cancel(self.hwnd, &self.state);
        }
        if actions::slider::settle(&self.state) || changed || marker_changed {
            self.state.painted.set(false);
        }
        unsafe {
            if changed
                || marker_changed
                || !self.state.painted.get()
                || self.state.text_dirty.replace(false)
            {
                let _ = InvalidateRect(Some(self.hwnd), None, false);
                let _ = UpdateWindow(self.hwnd);
                if !self.alive() || self.output_changed() {
                    return Ok(());
                }
            }
            if !IsWindowVisible(self.hwnd).as_bool() {
                let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            }
        }
        if self.state.painted.get()
            && self.state.frame.borrow().is_none()
            && self.state.marker.borrow().is_none()
        {
            self.state.shared.confirm_black(Instant::now());
        }
        if let Some(error) = self.state.error.borrow_mut().take() {
            return Err(error);
        }
        text::tick(&self.state);
        keyboard::tick(&self.state);
        scrollbar::tick(self.hwnd, &self.state);
        let stale = match &*self.state.pressed.borrow() {
            actions::Gesture::Slider(drag) => !actions::slider::retained(&self.state, drag),
            _ => false,
        };
        if stale {
            actions::slider::cancel(self.hwnd, &self.state);
        }
        Ok(())
    }
    pub fn hide(&self) {
        if self.alive() {
            unsafe {
                let _ = ShowWindow(self.hwnd, SW_HIDE);
            }
        }
        self.state.shared.stop();
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        self.state.shared.stop();
        if !self.state.closed.get() {
            unsafe {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}
unsafe fn register() -> Result<()> {
    static ONCE: OnceLock<std::result::Result<(), HRESULT>> = OnceLock::new();
    ONCE.get_or_init(|| {
        let result = (|| -> Result<()> {
            if RegisterClassW(&WNDCLASSW {
                lpfnWndProc: Some(procedure),
                hInstance: HINSTANCE(GetModuleHandleW(None)?.0),
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                lpszClassName: w!("Otter.Native.Tile"),
                ..Default::default()
            }) == 0
            {
                return Err(Error::from_win32());
            }
            Ok(())
        })();
        result.map_err(|e| e.code())
    })
    .map_err(Error::from_hresult)
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (*(lp.0 as *const CREATESTRUCTW)).lpCreateParams as isize,
        );
    }
    let Some(state) = (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State).as_ref() else {
        return DefWindowProcW(hwnd, msg, wp, lp);
    };
    if scrollbar::message(hwnd, state, msg, wp, lp) {
        return LRESULT(0);
    }
    let identifying = msg == WM_KEYDOWN && state.shared.lock().identification.blocked();
    if let Some(result) = text::message(hwnd, state, msg, wp, lp) {
        return result;
    }
    if !identifying {
        if let Some(result) = keyboard::message(hwnd, state, msg, wp, lp) {
            return result;
        }
    }
    match msg {
        WM_KEYDOWN if identifying => {
            if matches!(wp.0, 13 | 27 | 32) {
                let displayed = state.displayed_marker.borrow().clone();
                if let Some(displayed) = displayed {
                    state
                        .shared
                        .lock()
                        .identification
                        .dismiss(&displayed, Instant::now());
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            if IsWindowVisible(hwnd).as_bool() {
                let pixel = [
                    (lp.0 as u16 as i16) as i32,
                    ((lp.0 >> 16) as u16 as i16) as i32,
                ];
                actions::pointer_down(hwnd, state, pixel);
            }
            LRESULT(0)
        }
        WM_CAPTURECHANGED | WM_KILLFOCUS | WM_CANCELMODE => {
            actions::slider::cancel(hwnd, state);
            if msg == WM_CANCELMODE
                || msg == WM_KILLFOCUS && GetAncestor(HWND(wp.0 as *mut _), GA_ROOTOWNER) != hwnd
            {
                text::retire(state);
                keyboard::retire(state);
            }
            keyboard::cancel_press(state);
            LRESULT(0)
        }
        WM_SHOWWINDOW if wp.0 == 0 => {
            local::clear(state);
            actions::slider::cancel(hwnd, state);
            text::retire(state);
            keyboard::retire(state);
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_LBUTTONUP => {
            let displayed = state.displayed_marker.borrow().clone();
            if let Some(displayed) = displayed {
                let size = displayed.endpoint.pixel_size;
                let rect =
                    render::identification::close_rect([size.width as f32, size.height as f32]);
                let x = (lp.0 as u16 as i16) as f32;
                let y = ((lp.0 >> 16) as u16 as i16) as f32;
                if x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom {
                    state
                        .shared
                        .lock()
                        .identification
                        .dismiss(&displayed, Instant::now());
                }
            } else if IsWindowVisible(hwnd).as_bool() {
                actions::click(
                    hwnd,
                    state,
                    [
                        (lp.0 as u16 as i16) as i32,
                        ((lp.0 >> 16) as u16 as i16) as i32,
                    ],
                );
            } else {
                actions::slider::cancel(hwnd, state);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            actions::slider::update(
                hwnd,
                state,
                [
                    (lp.0 as u16 as i16) as i32,
                    ((lp.0 >> 16) as u16 as i16) as i32,
                ],
            );
            LRESULT(0)
        }
        WM_MOUSEWHEEL | WM_MOUSEHWHEEL if scroll::message(hwnd, state, msg, wp, lp) => LRESULT(0),
        WM_CONTEXTMENU => {
            let mut point = POINT {
                x: (lp.0 as u16 as i16) as i32,
                y: ((lp.0 >> 16) as u16 as i16) as i32,
            };
            if point.x == -1 && point.y == -1 {
                let _ = GetCursorPos(&mut point);
            }
            actions::menu(hwnd, state, point);
            LRESULT(0)
        }
        WM_DISPLAYCHANGE | WM_DPICHANGED => {
            // 不采用建议矩形迁移旧图；先隐藏并冻结旧代际，再由唯一输出owner重新枚举。
            state.changed.set(true);
            state.shared.stop();
            let _ = ShowWindow(hwnd, SW_HIDE);
            LRESULT(0)
        }
        WM_KEYDOWN if wp.0 == 27 => {
            // 编辑层取消后可把焦点还给Tile；同一次Esc的长按重复不能顺便关闭整个输出。
            if lp.0 as usize & (1 << 30) == 0 {
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut paint);
            // 隐藏窗口的WM_PAINT只验证更新区；软件提交可能没有有效DC，且不能确认已呈现。
            if !IsWindowVisible(hwnd).as_bool() {
                state.shared.trace.observe(Stage::HiddenPaint);
                state.painted.set(false);
                if state.closed.get() || state.changed.get() {
                    state.renderer.borrow_mut().take();
                }
                let _ = EndPaint(hwnd, &paint);
                return LRESULT(0);
            }
            let mut renderer = state.renderer.borrow_mut().take();
            let (frame, marker) = {
                let s = state.shared.lock();
                let marker = (!s.stopped && s.until.is_some_and(|v| Instant::now() < v))
                    .then(|| s.identification.marker(Instant::now()))
                    .flatten();
                (
                    s.visible(Instant::now()).then(|| s.frame.clone()).flatten(),
                    marker,
                )
            };
            let slider = match &*state.pressed.borrow() {
                actions::Gesture::Slider(drag) => Some((**drag).clone()),
                _ => state.slider_preview.borrow().clone(),
            };
            let result = if let Some(r) = &mut renderer {
                r.synchronize_scroll(&state.shared.lock());
                r.text_previews(state.text_preview.borrow().clone());
                r.local_values(state.local.borrow().snapshot());
                r.focus(keyboard::visual(state));
                r.paint(frame.as_ref(), marker.as_deref(), hwnd, slider.as_ref())
            } else {
                state.shared.trace.observe(Stage::RendererMissing);
                Ok(false)
            };
            // 绘制/提交可能重入关闭；原指针仍由外层 Window owner 持有，不能重新挂回资源。
            if !state.closed.get() && !state.changed.get() {
                *state.renderer.borrow_mut() = renderer;
            }
            if result.is_err() {
                state.shared.trace.observe(Stage::PaintFailed);
            } else if matches!(&result, Ok(false)) {
                state
                    .shared
                    .trace
                    .observe(if frame.is_some() && marker.is_none() {
                        Stage::FramePaintUnconfirmed
                    } else {
                        Stage::PaintUnconfirmed
                    });
            } else if state.closed.get() || state.changed.get() {
                state.shared.trace.observe(Stage::WindowRetiredDuringPaint);
            } else if !IsWindowVisible(hwnd).as_bool() {
                state.shared.trace.observe(Stage::HiddenAfterPaint);
            }
            if matches!(&result, Ok(true))
                && !state.closed.get()
                && !state.changed.get()
                && IsWindowVisible(hwnd).as_bool()
            {
                state.painted.set(true);
                *state.displayed_marker.borrow_mut() = marker.clone();
                if let Some(marker) = &marker {
                    let mut s = state.shared.lock();
                    if !s.stopped && s.until.is_some_and(|v| Instant::now() < v) {
                        s.identification.confirm(marker, Instant::now());
                    }
                } else if let Some(f) = &frame {
                    state.shared.confirm(f, Instant::now());
                } else {
                    state.shared.confirm_black(Instant::now());
                }
            }
            if let Err(error) = result {
                *state.error.borrow_mut() = Some(error);
                state.shared.stop();
                if !state.closed.get() {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                }
            }
            if dc.0.is_null() {
                state.shared.stop();
            }
            let _ = EndPaint(hwnd, &paint);
            LRESULT(0)
        }
        WM_DESTROY => {
            local::clear(state);
            state.closed.set(true);
            state.shared.stop();
            state.frame.borrow_mut().take();
            state.marker.borrow_mut().take();
            state.displayed_marker.borrow_mut().take();
            actions::slider::cancel(hwnd, state);
            text::retire(state);
            keyboard::retire(state);
            state.renderer.borrow_mut().take();
            LRESULT(0)
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

#[cfg(test)]
#[path = "tile_window_tests.rs"]
mod tests;
