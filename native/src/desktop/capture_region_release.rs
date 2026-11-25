//! 普通截图只用近透明输入层和穿透装饰层选矩形；不冻结或读取桌面像素。
use super::*;
#[path = "capture_region_release_paint.rs"]
mod paint;

struct Selection {
    size: [i32; 2],
    model: Cell<super::super::capture_selection::Selection>,
    windows: Vec<RECT>,
    hover: Cell<Option<RECT>>,
    press: Cell<Option<POINT>>,
    pointer: Cell<Option<POINT>>,
    color: Cell<Option<POINT>>,
    done: Cell<bool>,
    accepted: Cell<bool>,
    failed: Cell<bool>,
    outline: Cell<HWND>,
}
impl Selection {
    fn preview(&self) -> Option<RECT> {
        let model = self.model.get();
        model
            .rect
            .or_else(|| (!model.is_dragging()).then(|| self.hover.get()).flatten())
    }
    unsafe fn redraw(&self) {
        let _ = InvalidateRect(Some(self.outline.get()), None, false);
    }
}
struct Class(HINSTANCE);
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Native.ReleaseRegion"), Some(self.0));
        }
    }
}

pub(super) fn select(windows: Vec<RECT>) -> Result<Option<RECT>> {
    let screen = unsafe {
        let left = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let top = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let right = left.checked_add(GetSystemMetrics(SM_CXVIRTUALSCREEN));
        let bottom = top.checked_add(GetSystemMetrics(SM_CYVIRTUALSCREEN));
        RECT {
            left,
            top,
            right: right.ok_or_else(|| Error::new(E_INVALIDARG, "Desktop width overflow"))?,
            bottom: bottom.ok_or_else(|| Error::new(E_INVALIDARG, "Desktop height overflow"))?,
        }
    };
    select_with(screen, windows, true, |_| {})
}
fn select_with(
    screen: RECT,
    windows: Vec<RECT>,
    visible: bool,
    ready: impl FnOnce(HWND),
) -> Result<Option<RECT>> {
    let width = screen
        .right
        .checked_sub(screen.left)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid selection width"))?;
    let height = screen
        .bottom
        .checked_sub(screen.top)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid selection height"))?;
    super::super::capture::validate_region(screen.left, screen.top, width, height)?;
    let origin = POINT {
        x: screen.left,
        y: screen.top,
    };
    let _paint = super::super::buffered_paint::Session::new()?;
    let state = Selection {
        size: [width, height],
        model: Cell::new(super::super::capture_selection::Selection::new(
            width as u32,
            height as u32,
        )),
        windows: super::super::capture_windows::local_rectangles(
            windows,
            origin,
            width as u32,
            height as u32,
        ),
        hover: Cell::new(None),
        press: Cell::new(None),
        pointer: Cell::new(None),
        color: Cell::new(None),
        done: Cell::new(false),
        accepted: Cell::new(false),
        failed: Cell::new(false),
        outline: Cell::new(HWND::default()),
    };
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None)?.into();
        let class = WNDCLASSW {
            lpfnWndProc: Some(procedure),
            hInstance: instance,
            lpszClassName: w!("Otter.Native.ReleaseRegion"),
            hCursor: LoadCursorW(None, IDC_CROSS)?,
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            return Err(Error::from_win32());
        }
        let _class = Class(instance);
        let create = |extra, owner| {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | extra,
                class.lpszClassName,
                w!("Otter 区域截图：拖动选择，松手截图，Esc 取消"),
                WS_POPUP,
                screen.left,
                screen.top,
                width,
                height,
                owner,
                None,
                Some(instance),
                Some((&state as *const Selection).cast()),
            )
            .map(Window)
        };
        let input = create(WINDOW_EX_STYLE::default(), None)?;
        // alpha=1 保留整个输入面的命中，底层桌面仍实时可见；不能用透明色挖洞放走按下事件。
        SetLayeredWindowAttributes(input.0, COLORREF(0), 1, LWA_ALPHA)?;
        let outline = create(WS_EX_TRANSPARENT | WS_EX_NOACTIVATE, Some(input.0))?;
        state.outline.set(outline.0);
        SetLayeredWindowAttributes(outline.0, paint::KEY, 255, LWA_COLORKEY)?;
        // 两层都排除，并在最终采集前销毁；不把描边或近透明输入层混入截图。
        for window in [input.0, outline.0] {
            SetWindowDisplayAffinity(window, WDA_EXCLUDEFROMCAPTURE)?;
        }
        if visible {
            let _ = ShowWindow(input.0, SW_SHOW);
            let _ = ShowWindow(outline.0, SW_SHOWNOACTIVATE);
            let _ = SetForegroundWindow(input.0);
        }
        ready(input.0);
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
        if GetCapture() == input.0 {
            let _ = ReleaseCapture();
        }
        if let Some(point) = state.color.get() {
            let _ = ShowWindow(outline.0, SW_HIDE);
            let _ = ShowWindow(input.0, SW_HIDE);
            windows::Win32::Graphics::Dwm::DwmFlush()?;
            let sample =
                super::super::capture::region(origin.x + point.x, origin.y + point.y, 1, 1)?;
            let rgb = super::super::capture_pixel::rgb(&sample, POINT::default()).unwrap();
            super::super::clipboard::copy_color(input.0, rgb)?;
        }
        drop(outline);
        drop(input);
    }
    if state.failed.get() {
        return Err(Error::new(E_FAIL, "Cannot paint realtime selection"));
    }
    if !state.accepted.get() {
        return Ok(None);
    }
    state
        .model
        .get()
        .rect
        .map(|rect| super::super::capture_crop::screen_rectangle(origin, rect))
        .transpose()
}

unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if message == WM_NCCREATE {
        let create = &*(lp.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
    }
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Selection;
    if pointer.is_null() {
        return DefWindowProcW(hwnd, message, wp, lp);
    }
    let state = &*pointer;
    if message == WM_PAINT {
        if let Err(error) = paint::paint(hwnd, state) {
            eprintln!("native-capture-release-paint: {error}");
            state.failed.set(true);
            state.done.set(true);
        }
        return LRESULT(0);
    }
    if message == WM_ERASEBKGND {
        return LRESULT(1);
    }
    if message == WM_NCDESTROY {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        return DefWindowProcW(hwnd, message, wp, lp);
    }
    if hwnd == state.outline.get() {
        return DefWindowProcW(hwnd, message, wp, lp);
    }
    if state.done.get() {
        return DefWindowProcW(hwnd, message, wp, lp);
    }
    let point = POINT {
        x: lp.0 as i16 as i32,
        y: (lp.0 >> 16) as i16 as i32,
    };
    match message {
        WM_LBUTTONDOWN => {
            state.pointer.set(Some(point));
            let mut model = state.model.get();
            model.cancel_drag();
            model.rect = None;
            model.begin(point, 0);
            state.model.set(model);
            state.press.set(Some(point));
            state
                .hover
                .set(super::super::capture_windows::pick(&state.windows, point));
            SetCapture(hwnd);
            state.redraw();
        }
        WM_MOUSEMOVE => {
            if state.pointer.replace(Some(point)) == Some(point) {
                return LRESULT(0);
            }
            let before = state.preview();
            state
                .hover
                .set(super::super::capture_windows::pick(&state.windows, point));
            if GetCapture() == hwnd {
                let mut model = state.model.get();
                model.update(point);
                state.model.set(model);
            }
            if before != state.preview() {
                state.redraw();
            }
        }
        WM_LBUTTONUP if GetCapture() == hwnd => {
            let mut model = state.model.get();
            model.finish(point);
            if state.press.take() == Some(point) && model.rect.is_none() {
                model.rect = state.hover.get();
            }
            state.model.set(model);
            if model.rect.is_some() {
                state.accepted.set(true);
                state.done.set(true);
            }
            let _ = ReleaseCapture();
            state.redraw();
        }
        WM_CAPTURECHANGED | WM_CANCELMODE => {
            let mut model = state.model.get();
            model.cancel_drag();
            state.model.set(model);
            state.press.set(None);
            if message == WM_CANCELMODE && GetCapture() == hwnd {
                let _ = ReleaseCapture();
            }
            state.redraw();
        }
        WM_KEYDOWN if wp.0 == VK_ESCAPE.0 as usize => state.done.set(true),
        WM_KEYDOWN
            if wp.0 == 0x43
                && GetKeyState(VK_CONTROL.0 as i32) >= 0
                && !state.model.get().is_dragging() =>
        {
            if let Some(point) = state.pointer.get() {
                state.color.set(Some(POINT {
                    x: point.x.clamp(0, state.size[0] - 1),
                    y: point.y.clamp(0, state.size[1] - 1),
                }));
                state.done.set(true);
            }
        }
        WM_RBUTTONDOWN | WM_CLOSE | WM_DESTROY | WM_DISPLAYCHANGE => state.done.set(true),
        _ => return DefWindowProcW(hwnd, message, wp, lp),
    }
    LRESULT(0)
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "capture_region_release_tests.rs"]
mod tests;
