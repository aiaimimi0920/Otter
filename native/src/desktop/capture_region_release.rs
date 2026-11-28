//! Ordinary capture selects rectangles using only a nearly transparent input layer and click-through decoration layer; no desktop freezing or pixel reads.
use super::*;
#[path = "capture_region_release_paint.rs"]
mod paint;

const NON_RUDE: windows::core::PCWSTR = w!("NonRudeHWND");

struct Selection {
    last: Option<crate::capture_last_region::Last>,
    adjusting: Cell<bool>,
    hud: crate::desktop::capture_loupe_window::Selector,
    origin: POINT,
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
    unsafe fn redraw_change(&self, before: Option<RECT>, after: Option<RECT>) {
        let outline = self.outline.get();
        let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(outline).clamp(96, 768);
        super::drawing::invalidate_selection_change(outline, before, after, dpi);
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

pub(super) fn select(
    windows: Vec<RECT>,
    last: Option<crate::capture_last_region::Last>,
) -> Result<Option<RECT>> {
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
    select_region(screen, windows, last, true, |_| {})
}
#[cfg(test)]
fn select_with(
    screen: RECT,
    windows: Vec<RECT>,
    visible: bool,
    ready: impl FnOnce(HWND),
) -> Result<Option<RECT>> {
    select_region(screen, windows, None, visible, ready)
}
fn select_region(
    screen: RECT,
    windows: Vec<RECT>,
    last: Option<crate::capture_last_region::Last>,
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
        last,
        adjusting: Cell::new(false),
        hud: Default::default(),
        origin,
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
                &HSTRING::from(crate::ui_text::text("capture-region-quick-title")),
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
        // alpha=1 preserves hits over the entire input surface while the live desktop remains visible; do not punch transparent-color holes that let presses escape.
        SetLayeredWindowAttributes(input.0, COLORREF(0), 1, LWA_ALPHA)?;
        let outline = create(WS_EX_TRANSPARENT | WS_EX_NOACTIVATE, Some(input.0))?;
        state.outline.set(outline.0);
        SetLayeredWindowAttributes(outline.0, paint::KEY, 255, LWA_COLORKEY)?;
        for window in [input.0, outline.0] {
            // These monitor-sized popups are selectors, not fullscreen apps. Opt out before
            // either is shown so Explorer doesn't lower the taskbar and expose windows behind it.
            SetPropW(window, NON_RUDE, Some(HANDLE(1usize as *mut _)))?;
            SetWindowDisplayAffinity(window, WDA_EXCLUDEFROMCAPTURE)?;
            let _ = windows::Win32::UI::Input::Ime::ImmAssociateContext(
                window,
                windows::Win32::UI::Input::Ime::HIMC::default(),
            );
        }
        if visible {
            state.hud.install(input.0)?;
            let _ = ShowWindow(input.0, SW_SHOW);
            let _ = ShowWindow(outline.0, SW_SHOWNOACTIVATE);
            let _ = SetForegroundWindow(input.0);
            state.hud.prime(input.0, &state.pointer, origin);
            state.hover.set(
                state
                    .pointer
                    .get()
                    .and_then(|p| super::super::capture_windows::pick(&state.windows, p)),
            );
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
            if !(message.hwnd == input.0 && (WM_KEYFIRST..=WM_KEYLAST).contains(&message.message)) {
                let _ = TranslateMessage(&message);
            }
            DispatchMessageW(&message);
        }
        if GetCapture() == input.0 {
            let _ = ReleaseCapture();
        }
        if let Some(h) = state.hud.get() {
            h.hide();
        }
        if let Some(point) = state.color.get() {
            let _ = ShowWindow(outline.0, SW_HIDE);
            let _ = ShowWindow(input.0, SW_HIDE);
            windows::Win32::Graphics::Dwm::DwmFlush()?;
            let sample =
                super::super::capture::region(origin.x + point.x, origin.y + point.y, 1, 1)?;
            let rgb = super::super::capture_pixel::rgb(&sample, POINT::default()).unwrap();
            super::super::clipboard::copy_color(
                input.0,
                rgb,
                state.hud.get().map(|h| h.format()).unwrap_or_default(),
            )?;
        }
        drop(outline);
        drop(input);
        super::drawing::clear_cached_resources();
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
        let _ = RemovePropW(hwnd, NON_RUDE);
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
    if state.hud.dispatch(
        hwnd,
        message,
        wp,
        lp,
        state.pointer.get(),
        state.model.get().rect,
        state.origin,
        None,
    ) {
        return LRESULT(0);
    }
    match message {
        WM_KEYDOWN if region_keys(state, wp) => {}
        WM_LBUTTONDOWN => {
            state.pointer.set(Some(point));
            let mut model = state.model.get();
            model.cancel_drag();
            let tolerance = if state.adjusting.get() {
                (6 * windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd).max(96) / 96) as i32
            } else {
                model.rect = None;
                0
            };
            model.begin(point, tolerance);
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
            let after = state.preview();
            state.hud.schedule(hwnd);
            if before != after {
                state.redraw_change(before, after);
            }
        }
        WM_LBUTTONUP if GetCapture() == hwnd => {
            let mut model = state.model.get();
            model.finish(point);
            if state.press.take() == Some(point) && model.rect.is_none() {
                model.rect = state.hover.get();
            }
            state.model.set(model);
            if model.rect.is_some() && !state.adjusting.get() {
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
            let mut screen = POINT::default();
            if GetCursorPos(&mut screen).is_ok() && state.hud.get().is_some_and(|h| h.valid(screen))
            {
                state.color.set(Some(POINT {
                    x: screen.x - state.origin.x,
                    y: screen.y - state.origin.y,
                }));
                state.done.set(true);
            }
        }
        WM_RBUTTONDOWN | WM_CLOSE | WM_DESTROY | WM_DISPLAYCHANGE => state.done.set(true),
        _ => return DefWindowProcW(hwnd, message, wp, lp),
    }
    LRESULT(0)
}
unsafe fn region_keys(state: &Selection, wp: WPARAM) -> bool {
    let mut model = state.model.get();
    if model.is_dragging() {
        return false;
    }
    let Some(hud) = state.hud.get() else {
        return false;
    };
    if wp.0 == b'R' as usize && !state.adjusting.get() {
        if let Some([l, t, r, b]) = state
            .last
            .as_ref()
            .and_then(|last| last.restore(hud.displays()))
        {
            let (x, y) = (state.origin.x, state.origin.y);
            model.rect = Some(RECT {
                left: l - x,
                top: t - y,
                right: r - x,
                bottom: b - y,
            });
            state.model.set(model);
            state.adjusting.set(true);
            state.redraw();
        }
        return true;
    }
    if !state.adjusting.get() {
        return false;
    }
    let step = if GetKeyState(VK_SHIFT.0 as i32) < 0 {
        10
    } else {
        1
    };
    let before = model.rect;
    match wp.0 as u16 {
        0x25 => model.nudge(-step, 0),
        0x26 => model.nudge(0, -step),
        0x27 => model.nudge(step, 0),
        0x28 => model.nudge(0, step),
        0x0d => {
            if model
                .rect
                .and_then(|r| super::super::capture_crop::screen_rectangle(state.origin, r).ok())
                .is_some_and(|r| {
                    crate::capture_last_region::covered(
                        [r.left, r.top, r.right, r.bottom],
                        hud.displays(),
                    )
                })
            {
                state.accepted.set(true);
                state.done.set(true);
            }
            return true;
        }
        _ => return false,
    }
    state.model.set(model);
    state.redraw_change(before, model.rect);
    true
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "capture_region_release_tests.rs"]
mod tests;
