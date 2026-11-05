//! 冻结帧选区窗口；阻塞消息循环、事件触发绘制，窗口销毁后才释放帧及状态。
use super::{capture_crop, image::Image, Window};
use std::cell::Cell;
use windows::{
    core::{w, Error, Result},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};

struct Selection<'a> {
    image: &'a Image,
    model: Cell<super::capture_selection::Selection>,
    pointer: Cell<Option<POINT>>,
    done: Cell<bool>,
    accepted: Cell<bool>,
    paint_failed: Cell<bool>,
    windows: Vec<RECT>,
    hover: Cell<Option<RECT>>,
    live: bool,
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
        super::capture_windows::local_rectangles(&windows, origin, image.width, image.height);
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
pub(super) fn select_positioned(
    image: Image,
    origin: POINT,
    windows: Vec<RECT>,
) -> Result<Option<(Image, POINT)>> {
    let windows =
        super::capture_windows::local_rectangles(&windows, origin, image.width, image.height);
    select_positioned_with(image, origin, windows, true, |_| {})
}
fn select_positioned_with(
    image: Image,
    origin: POINT,
    windows: Vec<RECT>,
    visible: bool,
    ready: impl FnOnce(HWND),
) -> Result<Option<(Image, POINT)>> {
    select_bounds_with(&image, origin, windows, visible, false, ready)?
        .map(|(rect, _)| capture_crop::positioned(&image, origin, rect))
        .transpose()
}
/// Live 不复制冻结帧的裁剪像素；返回选择意图，窗口身份由调用方冻结前的快照拥有。
pub(super) fn select_live(
    image: Image,
    origin: POINT,
    windows: Vec<RECT>,
) -> Result<Option<(RECT, bool)>> {
    let windows =
        super::capture_windows::local_rectangles(&windows, origin, image.width, image.height);
    select_bounds_with(&image, origin, windows, true, true, |_| {})?
        .map(|(rect, whole)| Ok((capture_crop::screen_rectangle(origin, rect)?, whole)))
        .transpose()
}
fn select_bounds_with(
    image: &Image,
    origin: POINT,
    windows: Vec<RECT>,
    visible: bool,
    live: bool,
    ready: impl FnOnce(HWND),
) -> Result<Option<(RECT, bool)>> {
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
        live,
        whole_window: Cell::new(false),
        press: Cell::new(None),
    };
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None)?.into();
        let class = WNDCLASSW {
            style: if live {
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
            if live {
                w!("Otter Live：拖动选择区域，双击整窗，Esc 取消")
            } else {
                w!("Otter 区域截图：拖动选择，Enter 确认，Esc 取消")
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
            let previous = state.pointer.replace(Some(next));
            if previous.is_some_and(|old| old.x == next.x && old.y == next.y) {
                return LRESULT(0);
            }
            let hover = super::capture_windows::pick(&state.windows, next);
            let hover_changed = state.hover.replace(hover) != hover;
            if GetCapture() == hwnd {
                let mut model = state.model.get();
                model.update(next);
                state.model.set(model);
                let _ = InvalidateRect(Some(hwnd), None, false);
            } else {
                if hover_changed && state.model.get().rect.is_none() {
                    let _ = InvalidateRect(Some(hwnd), None, false);
                }
                let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd);
                for position in previous.into_iter().chain(Some(next)) {
                    let rect = super::capture_magnifier::bounds(state.image, position, dpi);
                    let _ = InvalidateRect(Some(hwnd), Some(&rect), false);
                }
            }
        }
        WM_LBUTTONUP if GetCapture() == hwnd => {
            state.pointer.set(Some(point()));
            let mut model = state.model.get();
            model.finish(point());
            let dragged =
                model.rect.is_some() && state.press.take().is_some_and(|start| start != point());
            if model.rect.is_none() {
                model.rect = state.hover.get();
            }
            state.model.set(model);
            let _ = ReleaseCapture();
            if state.live && dragged {
                state.accepted.set(true);
                state.done.set(true);
            }
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        WM_LBUTTONDBLCLK if state.live => {
            // 同一选择表面选取当前 Z 序悬停窗口；身份/边界在退出消息泵后重新校验。
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
        WM_KEYDOWN if wparam.0 == VK_RETURN.0 as usize => {
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
                    .and_then(|point| super::capture_magnifier::sample(state.image, point))
                    .map(|sample| super::clipboard::copy_color(hwnd, sample.rgb))
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
        WM_KEYDOWN if (VK_LEFT.0 as usize..=VK_DOWN.0 as usize).contains(&wparam.0) => {
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
            model.nudge(dx, dy);
            state.model.set(model);
            let _ = InvalidateRect(Some(hwnd), None, false);
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
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: state.image.width as i32,
                    biHeight: -(state.image.height as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    ..Default::default()
                },
                ..Default::default()
            };
            let painted = SetDIBitsToDevice(
                dc,
                0,
                0,
                state.image.width,
                state.image.height,
                0,
                0,
                0,
                state.image.height,
                state.image.pixels.as_ptr().cast(),
                &info,
                DIB_RGB_COLORS,
            );
            if painted == 0 {
                state.paint_failed.set(true);
                state.done.set(true);
            }
            if let Some(rect) = state.preview_rect() {
                let _ = DrawFocusRect(dc, &rect);
                let half =
                    (3 * windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd).max(96) / 96) as i32;
                for point in super::capture_selection::handles(rect) {
                    let handle = RECT {
                        left: point.x - half,
                        top: point.y - half,
                        right: point.x + half + 1,
                        bottom: point.y + half + 1,
                    };
                    let _ = FrameRect(dc, &handle, HBRUSH(GetStockObject(WHITE_BRUSH).0));
                }
                let mut text: Vec<u16> =
                    format!("{} × {} px", rect.right - rect.left, rect.bottom - rect.top)
                        .encode_utf16()
                        .collect();
                let mut label = RECT {
                    left: rect.left,
                    top: if rect.top >= 24 {
                        rect.top - 24
                    } else {
                        rect.top + 4
                    },
                    right: (rect.left + 200).min(state.image.width as i32),
                    bottom: if rect.top >= 24 {
                        rect.top
                    } else {
                        rect.top + 28
                    },
                };
                DrawTextW(
                    dc,
                    &mut text,
                    &mut label,
                    DT_LEFT | DT_SINGLELINE | DT_NOPREFIX,
                );
            }
            if let Some(point) = state.pointer.get() {
                if !super::capture_magnifier::draw(
                    dc,
                    state.image,
                    point,
                    windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd),
                ) {
                    state.paint_failed.set(true);
                    state.done.set(true);
                }
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

#[cfg(test)]
#[path = "capture_region_tests.rs"]
mod tests;
