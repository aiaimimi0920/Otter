//! Shared link layer, independent per-node borders, and at most two endpoint strips; no HWND per port.
use super::*;
use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};
use windows::Win32::{Graphics::Gdi::*, System::LibraryLoader::GetModuleHandleW};
pub(super) const CLASS: windows::core::PCWSTR = w!("Otter.Native.WorkspaceChrome");
pub(super) const MAX_PIXELS: i64 = 33_554_432;
#[path = "workspace_chrome_order.rs"]
mod order;
#[path = "workspace_chrome_strips.rs"]
mod strips;
pub(super) use order::apply as order_drawing;
pub(super) struct Windows {
    ui: Rc<Ui>,
    drawing: HWND,
    inputs: Vec<strips::Strip>,
    language: crate::ui_text::DisplayLanguage,
    _paint_session: draw::PaintSession,
}
impl Windows {
    pub unsafe fn new(ui: Rc<Ui>) -> Result<Self> {
        register()?;
        let mut owner = Self {
            ui,
            drawing: HWND::default(),
            inputs: Vec::new(),
            language: crate::ui_text::current_language(),
            _paint_session: draw::PaintSession::new()?,
        };
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST | WS_EX_TRANSPARENT,
            CLASS,
            &HSTRING::from(crate::ui_text::text("workspace-title")),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(HINSTANCE(GetModuleHandleW(None)?.0)),
            Some(Rc::as_ptr(&owner.ui).cast()),
        )?;
        owner.drawing = hwnd;
        owner.ui.drawing.set(hwnd);
        SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)?;
        Ok(owner)
    }
    pub unsafe fn sync(&mut self) -> Result<()> {
        let nodes = self.ui.nodes.borrow().clone();
        if nodes.len() > crate::workspace::MAX_UNITS {
            return Err(Error::from(E_OUTOFMEMORY));
        }
        border::sync(&self.ui, &nodes)?;
        backing::sync(&self.ui, &nodes)?;
        let by_endpoint: HashMap<_, _> = nodes.iter().map(|node| (node.endpoint, node)).collect();
        let mut present = HashSet::with_capacity(self.inputs.len());
        self.inputs.retain(|strip| {
            by_endpoint
                .get(&strip.endpoint)
                .is_some_and(|node| strip.matches(node))
                && present.insert((strip.endpoint, strip.side))
        });
        for node in &nodes {
            for (side, count) in [(Side::Input, node.inputs), (Side::Output, node.outputs)] {
                if (count > 0
                    || side == Side::Output
                        && candidate_auxiliary::layout(&self.ui, node).is_some())
                    && present.insert((node.endpoint, side))
                {
                    self.inputs
                        .push(strips::Strip::new(self.ui.clone(), node, side)?);
                }
            }
        }
        *self.ui.input_windows.borrow_mut() = self
            .inputs
            .iter()
            .map(|strip| (strip.hwnd, strip.endpoint, strip.side))
            .collect();
        self.refresh_language()?;
        refresh(&self.ui)
    }
    pub unsafe fn refresh(&self) -> Result<()> {
        refresh(&self.ui)
    }
    pub fn language_changed(&self) -> bool {
        self.language != crate::ui_text::current_language()
    }
    unsafe fn refresh_language(&mut self) -> Result<()> {
        if !self.language_changed() {
            return Ok(());
        }
        let titles = [
            (vec![self.drawing], "workspace-title"),
            (
                self.inputs.iter().map(|strip| strip.hwnd).collect(),
                "workspace-ports-title",
            ),
            (
                self.ui
                    .backings
                    .borrow()
                    .iter()
                    .map(|backing| backing.hwnd)
                    .collect(),
                "workspace-parameter-backing-title",
            ),
            (
                self.ui
                    .borders
                    .borrow()
                    .iter()
                    .map(|border| border.hwnd)
                    .collect(),
                "workspace-pin-border-title",
            ),
        ];
        for (handles, id) in titles {
            for hwnd in handles {
                SetWindowTextW(hwnd, &HSTRING::from(crate::ui_text::text(id)))?;
            }
        }
        self.language = crate::ui_text::current_language();
        Ok(())
    }
}
impl Drop for Windows {
    fn drop(&mut self) {
        unsafe {
            self.ui.cancel();
            self.inputs.clear();
            drop(self.ui.borders.replace(Vec::new()));
            drop(self.ui.backings.replace(Vec::new()));
            self.ui.input_windows.borrow_mut().clear();
            self.ui.drawing.set(HWND::default());
            self.ui.surface.borrow_mut().take();
            self.ui.port_painter.borrow_mut().take();
            self.ui.drawers.borrow_mut().clear();
            for hwnd in [self.drawing] {
                if !hwnd.is_invalid()
                    && GetWindowLongPtrW(hwnd, GWLP_USERDATA) == Rc::as_ptr(&self.ui) as isize
                {
                    let _ = DestroyWindow(hwnd);
                }
            }
        }
    }
}
unsafe fn register() -> Result<()> {
    static REGISTERED: OnceLock<std::result::Result<(), windows::core::HRESULT>> = OnceLock::new();
    REGISTERED
        .get_or_init(|| {
            let result = (|| -> Result<()> {
                let class = WNDCLASSW {
                    lpfnWndProc: Some(procedure),
                    hInstance: HINSTANCE(GetModuleHandleW(None)?.0),
                    lpszClassName: CLASS,
                    // Endpoint strips also host labels, eyes, and fields; no lingering crosshair after link capture ends.
                    hCursor: LoadCursorW(None, IDC_ARROW)?,
                    ..Default::default()
                };
                if RegisterClassW(&class) == 0 {
                    return Err(Error::from_win32());
                }
                Ok(())
            })();
            result.map_err(|error| error.code())
        })
        .map_err(Error::from)
}
pub(super) unsafe fn hide(ui: &Ui) {
    drawer::cancel(ui);
    candidate_auxiliary::cancel(ui);
    border::restore_stationary(ui);
    ui.keyboard.retire();
    ui.cancel();
    ui.pending.borrow_mut().take();
    for hwnd in ui.handles().into_iter().filter(|h| !h.is_invalid()) {
        if !border::has_content(ui, hwnd) {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
}
fn padded(rect: Rect, x: i32, y: i32) -> Rect {
    Rect {
        left: rect.left.saturating_sub(x),
        top: rect.top.saturating_sub(y),
        right: rect.right.saturating_add(x),
        bottom: rect.bottom.saturating_add(y),
    }
}
pub(super) unsafe fn refresh(ui: &Ui) -> Result<()> {
    refresh_at(ui, false)
}
pub(super) unsafe fn refresh_at(ui: &Ui, translated: bool) -> Result<()> {
    keyboard::reconcile(ui);
    if ui.capturing() {
        ui.redraw_after_capture.set(true);
        ui.cancel();
        return Ok(());
    }
    refresh_drawing(ui)?;
    if ui.blocked() {
        return Ok(());
    }
    border::refresh(ui)?;
    strips::refresh(ui, translated)?;
    backing::refresh(ui)?;
    drawer::geometry(ui)
}
pub(super) unsafe fn refresh_drawing(ui: &Ui) -> Result<()> {
    if ui.nodes.borrow().is_empty() {
        ui.surface.borrow_mut().take();
        hide(ui);
        return Ok(());
    }
    if ui.blocked() {
        hide(ui);
        return Ok(());
    }
    let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
    let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
    let right = vx.saturating_add(GetSystemMetrics(SM_CXVIRTUALSCREEN));
    let bottom = vy.saturating_add(GetSystemMetrics(SM_CYVIRTUALSCREEN));
    let clip = |r: Rect| Rect {
        left: r.left.max(vx),
        top: r.top.max(vy),
        right: r.right.min(right),
        bottom: r.bottom.min(bottom),
    };
    let mut lines = draw::lines(ui);
    // Independent offscreen links must not inflate the shared DIB or hide visible port strips.
    lines.retain(|line| {
        let r = clip(draw::curve_bounds(line));
        r.left < r.right && r.top < r.bottom
    });
    let rect = if let Some(needed) = draw::line_bounds(&lines).map(clip) {
        let mut rect = clip(padded(needed, 128, 64));
        // Retain HWND/DIB size when active curves fit within spare margin and have not shrunk far below an old surface.
        let mut existing = RECT::default();
        if ui.surface.borrow().is_some() && GetWindowRect(ui.drawing.get(), &mut existing).is_ok() {
            let area = |left, top, right, bottom| {
                (i64::from(right) - i64::from(left))
                    .max(0)
                    .saturating_mul((i64::from(bottom) - i64::from(top)).max(0))
            };
            let existing_pixels =
                area(existing.left, existing.top, existing.right, existing.bottom);
            let target_pixels = area(rect.left, rect.top, rect.right, rect.bottom);
            if existing.left <= needed.left
                && existing.top <= needed.top
                && existing.right >= needed.right
                && existing.bottom >= needed.bottom
                && existing.left >= vx
                && existing.top >= vy
                && existing.right <= right
                && existing.bottom <= bottom
                && existing_pixels <= target_pixels.saturating_mul(2).max(262_144)
            {
                rect = Rect {
                    left: existing.left,
                    top: existing.top,
                    right: existing.right,
                    bottom: existing.bottom,
                };
            }
        }
        rect
    } else {
        // Unlinked pins need no desktop-spanning DIB; retain a 1x1 anchor so visibility and Z-order invariants hold at 4-byte cost.
        clip(Rect {
            left: vx,
            top: vy,
            right: vx.saturating_add(1),
            bottom: vy.saturating_add(1),
        })
    };
    let width = i64::from(rect.right) - i64::from(rect.left);
    let height = i64::from(rect.bottom) - i64::from(rect.top);
    if width <= 0 || height <= 0 {
        hide(ui);
        return Ok(());
    }
    if width > 32768 || height > 32768 || width * height > MAX_PIXELS {
        hide(ui);
        return Err(Error::new(
            E_OUTOFMEMORY,
            crate::ui_text::text("workspace-pixel-budget"),
        ));
    }
    let drawing = ui.drawing.get();
    if drawing.is_invalid() {
        return Ok(());
    }
    // One overlay may contain multiple owners; apply the strictest affinity to avoid exposing protected-node ports.
    let affinity = if ui.nodes.borrow().iter().any(|node| node.protected) {
        WDA_EXCLUDEFROMCAPTURE
    } else {
        WDA_NONE
    };
    affinity_if_changed(drawing, affinity)?;
    if ui.blocked() {
        hide(ui);
        return Ok(());
    }
    // Establish Z-order before first display/new-node publication; never draw new links over images then lower them.
    order_drawing(ui)?;
    draw::present(ui, rect, lines)?;
    if !IsWindowVisible(drawing).as_bool() {
        SetWindowPos(
            drawing,
            None,
            0,
            0,
            0,
            0,
            SWP_SHOWWINDOW | SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
        )?;
    }
    // A complete frame is committed; idle WM_PAINT must not copy the shared line layer again.
    ValidateRect(Some(drawing), None).ok()?;
    Ok(())
}
pub(super) unsafe fn affinity_if_changed(
    hwnd: HWND,
    desired: WINDOW_DISPLAY_AFFINITY,
) -> Result<()> {
    let mut current = 0;
    GetWindowDisplayAffinity(hwnd, &mut current)?;
    if current != desired.0 {
        SetWindowDisplayAffinity(hwnd, desired)?;
    }
    Ok(())
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    std::panic::catch_unwind(|| dispatch(hwnd, msg, wp, lp)).unwrap_or_else(|_| {
        let _ = ShowWindow(hwnd, SW_HIDE);
        LRESULT(0)
    })
}
unsafe fn dispatch(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (*(lp.0 as *const CREATESTRUCTW)).lpCreateParams as isize,
        );
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Ui;
    if pointer.is_null() {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let ui = &*pointer;
    match msg {
        WM_PAINT => {
            draw::paint(hwnd, ui);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_WINDOWPOSCHANGED => {
            // Focus/child activation may raise layers again after WM_ACTIVATE; follow actual Z-order notifications without stealing focus.
            let result = DefWindowProcW(hwnd, msg, wp, lp);
            if !(*(lp.0 as *const WINDOWPOS)).flags.contains(SWP_NOZORDER) {
                if let Err(error) = order_drawing(ui) {
                    ui.fail(error);
                }
            }
            result
        }
        WM_GETDLGCODE => LRESULT((DLGC_WANTARROWS | DLGC_WANTTAB | DLGC_WANTCHARS) as isize),
        WM_KILLFOCUS => {
            keyboard::lost(ui, HWND(wp.0 as *mut _));
            LRESULT(0)
        }
        WM_LBUTTONDOWN | WM_MOUSEMOVE | WM_LBUTTONUP => {
            if !parameter_scroll::message(hwnd, ui, msg, wp, lp) {
                input::mouse(hwnd, ui, msg, lp);
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            parameter_scroll::message(hwnd, ui, msg, wp, lp);
            LRESULT(0)
        }
        WM_CONTEXTMENU => {
            if parameter_rail::context(ui, hwnd, lp) {
                LRESULT(0)
            } else {
                DefWindowProcW(hwnd, msg, wp, lp)
            }
        }
        WM_CANCELMODE | WM_CAPTURECHANGED => {
            ui.cancel();
            ui.repaint();
            LRESULT(0)
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
