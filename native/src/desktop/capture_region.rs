//! Frozen-frame selection window; blocking message loop and event-driven drawing, with frame/state released only after window destruction.
use super::{capture_crop, image::Image, Window};
use std::cell::Cell;
#[path = "capture_region_paint.rs"]
mod drawing;
#[path = "capture_region_release.rs"]
mod release;
pub(super) fn select_realtime(windows: Vec<RECT>) -> Result<Option<RECT>> {
    release::select(windows)
}
use windows::{
    core::{w, Error, Result},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Quick,
    Adjust,
    Live,
}
struct Selection<'a> {
    image: &'a Image,
    model: Cell<super::capture_selection::Selection>,
    pointer: Cell<Option<POINT>>,
    done: Cell<bool>,
    accepted: Cell<bool>,
    paint_failed: Cell<bool>,
    windows: Vec<RECT>,
    hover: Cell<Option<RECT>>,
    mode: Mode,
    whole_window: Cell<bool>,
    press: Cell<Option<POINT>>,
}
impl Selection<'_> {
    fn preview_rect(&self) -> Option<RECT> {
        let model = self.model.get();
        model.rect.or_else(|| {
            if model.is_dragging() {
                None
            } else {
                self.hover.get()
            }
        })
    }
}
struct Class(HINSTANCE);
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Native.CaptureRegion"), Some(self.0));
        }
    }
}

pub(super) fn select(image: Image, origin: POINT, windows: Vec<RECT>) -> Result<Option<Image>> {
    let windows =
        super::capture_windows::local_rectangles(windows, origin, image.width, image.height);
    select_with(image, origin, windows, true, |_| {})
}
fn select_with(
    image: Image,
    origin: POINT,
    windows: Vec<RECT>,
    visible: bool,
    ready: impl FnOnce(HWND),
) -> Result<Option<Image>> {
    select_positioned_with(image, origin, windows, visible, ready)
        .map(|result| result.map(|(image, _)| image))
}
/// Recording's frozen image is for selection only, not video; release frozen pixels before resampling the real target.
pub(super) fn select_recording(
    image: Image,
    origin: POINT,
    windows: Vec<RECT>,
) -> Result<Option<RECT>> {
    let windows =
        super::capture_windows::local_rectangles(windows, origin, image.width, image.height);
    select_bounds_with(&image, origin, windows, true, Mode::Quick, |_| {})?
        .map(|(rect, _)| capture_crop::screen_rectangle(origin, rect))
        .transpose()
}
fn select_positioned_with(
    image: Image,
    origin: POINT,
    windows: Vec<RECT>,
    visible: bool,
    ready: impl FnOnce(HWND),
) -> Result<Option<(Image, POINT)>> {
    select_bounds_with(&image, origin, windows, visible, Mode::Quick, ready)?
        .map(|(rect, _)| capture_crop::positioned(&image, origin, rect))
        .transpose()
}
/// Scrolling capture retains selection confirmation; ordinary capture's release-to-submit must not change this specialized flow.
pub(super) fn select_adjustable(
    image: Image,
    origin: POINT,
    windows: Vec<RECT>,
) -> Result<Option<(Image, POINT)>> {
    let windows =
        super::capture_windows::local_rectangles(windows, origin, image.width, image.height);
    select_bounds_with(&image, origin, windows, true, Mode::Adjust, |_| {})?
        .map(|(rect, _)| capture_crop::positioned(&image, origin, rect))
        .transpose()
}
/// Live does not copy cropped frozen pixels; return selection intent, with window identity owned by the caller's pre-freeze snapshot.
pub(super) fn select_live(
    image: Image,
    origin: POINT,
    windows: Vec<RECT>,
) -> Result<Option<(RECT, bool)>> {
    let windows =
        super::capture_windows::local_rectangles(windows, origin, image.width, image.height);
    select_bounds_with(&image, origin, windows, true, Mode::Live, |_| {})?
        .map(|(rect, whole)| Ok((capture_crop::screen_rectangle(origin, rect)?, whole)))
        .transpose()
}
fn select_bounds_with(
    image: &Image,
    origin: POINT,
    windows: Vec<RECT>,
    visible: bool,
    mode: Mode,
    ready: impl FnOnce(HWND),
) -> Result<Option<(RECT, bool)>> {
    let _paint_session = super::buffered_paint::Session::new()?;
    let state = Selection {
        image,
        windows,
        hover: Cell::new(None),
        pointer: Cell::new(None),
        model: Cell::new(super::capture_selection::Selection::new(
            image.width,
            image.height,
        )),
        done: Cell::new(false),
        accepted: Cell::new(false),
        paint_failed: Cell::new(false),
        mode,
        whole_window: Cell::new(false),
        press: Cell::new(None),
    };
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None)?.into();
        let class = WNDCLASSW {
            style: if mode == Mode::Live {
                CS_DBLCLKS
            } else {
                WNDCLASS_STYLES::default()
            },
            lpfnWndProc: Some(procedure),
            hInstance: instance,
            lpszClassName: w!("Otter.Native.CaptureRegion"),
            hCursor: LoadCursorW(None, IDC_CROSS)?,
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            return Err(Error::from_win32());
        }
        let _class = Class(instance);
        let window = Window(CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class.lpszClassName,
            match mode {
                Mode::Quick => w!("Otter 区域截图：拖动选择，松手截图，Esc 取消"),
                Mode::Adjust => w!("Otter 长截图选区：拖动选择，Enter 确认，Esc 取消"),
                Mode::Live => w!("Otter Live：拖动选择区域，双击整窗，Esc 取消"),
            },
            WS_POPUP,
            origin.x,
            origin.y,
            image.width as i32,
            image.height as i32,
            None,
            None,
            Some(instance),
            Some((&state as *const Selection).cast()),
        )?);
        if visible {
            let _ = ShowWindow(window.0, SW_SHOW);
            let _ = SetForegroundWindow(window.0);
        }
        ready(window.0);
        let mut message = MSG::default();
        while !state.done.get() {
            let result = GetMessageW(&mut message, None, 0, 0).0;
            if result == -1 {
                return Err(Error::from_win32());
            }
            if result == 0 {
                PostQuitMessage(message.wParam.0 as i32);
                break;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        if GetCapture() == window.0 {
            let _ = ReleaseCapture();
        }
        drop(window);
    }
    if state.paint_failed.get() {
        return Err(Error::new(E_FAIL, "Cannot paint frozen capture"));
    }
    if !state.accepted.get() {
        return Ok(None);
    }
    Ok(state
        .model
        .get()
        .rect
        .map(|rect| (rect, state.whole_window.get())))
}

unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let create = &*(lparam.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
    }
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Selection;
    if pointer.is_null() {
        return DefWindowProcW(hwnd, message, wparam, lparam);
    }
    let state = &*pointer;
    // Freeze the result after release submission so reentrant input during capture release/window destruction cannot rewrite selection.
    if state.done.get()
        && matches!(
            message,
            WM_LBUTTONDOWN | WM_LBUTTONUP | WM_LBUTTONDBLCLK | WM_MOUSEMOVE | WM_KEYDOWN
        )
    {
        return LRESULT(0);
    }
    let point = || POINT {
        x: lparam.0 as i16 as i32,
        y: (lparam.0 >> 16) as i16 as i32,
    };
    match message {
        WM_LBUTTONDOWN => {
            state.press.set(Some(point()));
            state.pointer.set(Some(point()));
            state
                .hover
                .set(super::capture_windows::pick(&state.windows, point()));
            let mut model = state.model.get();
            if state.mode == Mode::Quick {
                model =
                    super::capture_selection::Selection::new(state.image.width, state.image.height);
            }
            model.begin(
                point(),
                (6 * windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd).max(96) / 96) as i32,
            );
            state.model.set(model);
            SetCapture(hwnd);
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        WM_MOUSEMOVE => {
            let next = point();
            if state.pointer.replace(Some(next)) == Some(next) {
                return LRESULT(0);
            }
            let before = state.preview_rect();
            let hover = super::capture_windows::pick(&state.windows, next);
            state.hover.set(hover);
            if GetCapture() == hwnd {
                let mut model = state.model.get();
                model.update(next);
                state.model.set(model);
            }
            // Continued dragging beyond bounds may leave the rectangle unchanged; redraw the frozen frame only when visible selection changes.
            if before != state.preview_rect() {
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
        }
        WM_LBUTTONUP if GetCapture() == hwnd => {
            state.pointer.set(Some(point()));
            let mut model = state.model.get();
            model.finish(point());
            let press = state.press.take();
            let dragged = model.rect.is_some() && press.is_some_and(|start| start != point());
            if model.rect.is_none() && (state.mode != Mode::Quick || press == Some(point())) {
                model.rect = state.hover.get();
            }
            state.model.set(model);
            if (state.mode == Mode::Quick && model.rect.is_some())
                || (state.mode == Mode::Live && dragged)
            {
                state.accepted.set(true);
                state.done.set(true);
            }
            let _ = ReleaseCapture();
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        WM_LBUTTONDBLCLK if state.mode == Mode::Live => {
            // Select the hovered window in current Z order on the same selection surface; revalidate identity/bounds after leaving the message loop.
            if let Some(rect) = super::capture_windows::pick(&state.windows, point()) {
                let mut model = state.model.get();
                model.cancel_drag();
                model.rect = Some(rect);
                state.model.set(model);
                state.whole_window.set(true);
                state.accepted.set(true);
                state.done.set(true);
            }
        }
        WM_KEYDOWN if wparam.0 == VK_RETURN.0 as usize && state.mode != Mode::Quick => {
            if state.preview_rect().is_some() && !state.model.get().is_dragging() {
                let mut model = state.model.get();
                model.rect = state.preview_rect();
                state.model.set(model);
                state.accepted.set(true);
                state.done.set(true);
            }
        }
        WM_KEYDOWN if wparam.0 == VK_ESCAPE.0 as usize => state.done.set(true),
        WM_KEYDOWN if wparam.0 == 0x43 && !state.model.get().is_dragging() => {
            let result = if GetKeyState(VK_CONTROL.0 as i32) < 0 {
                state
                    .model
                    .get()
                    .rect
                    .map(|rect| super::clipboard::copy_image(hwnd, state.image, rect))
            } else {
                state
                    .pointer
                    .get()
                    .and_then(|point| super::capture_pixel::rgb(state.image, point))
                    .map(|rgb| super::clipboard::copy_color(hwnd, rgb))
            };
            match result {
                Some(Ok(())) => state.done.set(true),
                Some(Err(error)) => {
                    let text = windows::core::HSTRING::from(format!(
                        "复制失败，请重试。若发布阶段失败，原剪贴板内容可能已被清空。\n{error}"
                    ));
                    MessageBoxW(Some(hwnd), &text, w!("Otter"), MB_OK | MB_ICONERROR);
                }
                None => {}
            }
        }
        WM_KEYDOWN
            if state.mode != Mode::Quick
                && (VK_LEFT.0 as usize..=VK_DOWN.0 as usize).contains(&wparam.0) =>
        {
            let step = if GetKeyState(VK_SHIFT.0 as i32) < 0 {
                10
            } else {
                1
            };
            let (dx, dy) = match wparam.0 as u16 {
                0x25 => (-step, 0),
                0x26 => (0, -step),
                0x27 => (step, 0),
                _ => (0, step),
            };
            let mut model = state.model.get();
            let before = model.rect;
            model.nudge(dx, dy);
            state.model.set(model);
            if before != model.rect {
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
        }
        WM_CAPTURECHANGED | WM_CANCELMODE => {
            let mut model = state.model.get();
            model.cancel_drag();
            state.model.set(model);
            if message == WM_CANCELMODE && GetCapture() == hwnd {
                let _ = ReleaseCapture();
            }
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        WM_RBUTTONDOWN | WM_CLOSE | WM_DESTROY | WM_DISPLAYCHANGE => state.done.set(true),
        WM_ERASEBKGND => return LRESULT(1),
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut paint);
            if let Err(error) = drawing::paint(hwnd, state, dc, &paint.rcPaint) {
                eprintln!("native-capture-paint: {error}");
                state.paint_failed.set(true);
                state.done.set(true);
            }
            let _ = EndPaint(hwnd, &paint);
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            return DefWindowProcW(hwnd, message, wparam, lparam);
        }
        _ => return DefWindowProcW(hwnd, message, wparam, lparam),
    }
    LRESULT(0)
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "capture_region_tests.rs"]
mod tests;
