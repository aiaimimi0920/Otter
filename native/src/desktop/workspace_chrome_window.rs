//! 一个穿透线层及每节点至多两个 owned 端点条；不为每端口建立 HWND。
use super::*;
use std::sync::OnceLock;
use windows::Win32::{Graphics::Gdi::*, System::LibraryLoader::GetModuleHandleW};
const CLASS: windows::core::PCWSTR = w!("Otter.Native.WorkspaceChrome");
const MAX_PIXELS: i64 = 33_554_432;
#[path = "workspace_chrome_strips.rs"]
mod strips;
pub(super) struct Windows {
    ui: Rc<Ui>,
    drawing: HWND,
    inputs: Vec<strips::Strip>,
}
impl Windows {
    pub unsafe fn new(ui: Rc<Ui>) -> Result<Self> {
        register()?;
        let mut owner = Self {
            ui,
            drawing: HWND::default(),
            inputs: Vec::new(),
        };
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST | WS_EX_TRANSPARENT,
            CLASS,
            w!("Otter · 工作区"),
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
        SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_COLORKEY)?;
        Ok(owner)
    }
    pub unsafe fn sync(&mut self) -> Result<()> {
        let nodes = self.ui.nodes.borrow().clone();
        if nodes.len() > crate::workspace::MAX_UNITS {
            return Err(Error::from(E_OUTOFMEMORY));
        }
        self.inputs
            .retain(|strip| nodes.iter().any(|node| strip.matches(node)));
        for node in &nodes {
            for (side, count) in [(Side::Input, node.inputs), (Side::Output, node.outputs)] {
                if count > 0
                    && !self
                        .inputs
                        .iter()
                        .any(|strip| strip.matches(node) && strip.side == side)
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
        refresh(&self.ui)
    }
    pub unsafe fn refresh(&self) -> Result<()> {
        refresh(&self.ui)
    }
}
impl Drop for Windows {
    fn drop(&mut self) {
        unsafe {
            self.ui.cancel();
            self.inputs.clear();
            self.ui.input_windows.borrow_mut().clear();
            self.ui.drawing.set(HWND::default());
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
                    hCursor: LoadCursorW(None, IDC_CROSS)?,
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
    capsule::cancel(ui);
    ui.keyboard.retire();
    ui.cancel();
    ui.pending.borrow_mut().take();
    for hwnd in ui.handles().into_iter().filter(|h| !h.is_invalid()) {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
}
fn bounds(nodes: &[Node]) -> Option<Rect> {
    let first = nodes.first()?;
    let mut rect = first.rect;
    for node in nodes {
        rect.left = rect.left.min(node.rect.left);
        rect.top = rect.top.min(node.rect.top);
        rect.right = rect.right.max(node.rect.right);
        rect.bottom = rect.bottom.max(node.rect.bottom);
    }
    Some(Rect {
        left: rect.left.saturating_sub(800),
        top: rect.top.saturating_sub(100),
        right: rect.right.saturating_add(800),
        bottom: rect.bottom.saturating_add(100),
    })
}
pub(super) unsafe fn refresh(ui: &Ui) -> Result<()> {
    keyboard::reconcile(ui);
    capsule::reconcile(ui);
    refresh_drawing(ui)?;
    if ui.blocked() {
        return Ok(());
    }
    strips::refresh(ui)
}
pub(super) unsafe fn refresh_drawing(ui: &Ui) -> Result<()> {
    let nodes = ui.nodes.borrow().clone();
    let Some(mut rect) = bounds(&nodes).filter(|_| !ui.blocked()) else {
        hide(ui);
        return Ok(());
    };
    if let Some(press) = ui.press.borrow().as_ref() {
        rect.left = rect.left.min(press.current.x.saturating_sub(160));
        rect.right = rect.right.max(press.current.x.saturating_add(160));
        rect.top = rect.top.min(press.current.y.saturating_sub(16));
        rect.bottom = rect.bottom.max(press.current.y.saturating_add(16));
    }
    let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
    let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
    rect.left = rect.left.max(vx);
    rect.top = rect.top.max(vy);
    rect.right = rect
        .right
        .min(vx.saturating_add(GetSystemMetrics(SM_CXVIRTUALSCREEN)));
    rect.bottom = rect
        .bottom
        .min(vy.saturating_add(GetSystemMetrics(SM_CYVIRTUALSCREEN)));
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
            "工作区叠层超过显示像素预算，文档保留",
        ));
    }
    let drawing = ui.drawing.get();
    if drawing.is_invalid() {
        return Ok(());
    }
    // 一张叠层可能包含多个 owner；采用最严格 affinity，不能泄漏受保护节点的端口。
    let affinity = if nodes.iter().any(|node| node.protected) {
        WDA_EXCLUDEFROMCAPTURE
    } else {
        WDA_NONE
    };
    SetWindowDisplayAffinity(drawing, affinity)?;
    SetWindowPos(
        drawing,
        Some(HWND_TOPMOST),
        rect.left,
        rect.top,
        width as i32,
        height as i32,
        SWP_NOACTIVATE,
    )?;
    if ui.blocked() {
        hide(ui);
        return Ok(());
    }
    let _ = ShowWindow(drawing, SW_SHOWNOACTIVATE);
    ui.repaint();
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
        WM_GETDLGCODE => LRESULT((DLGC_WANTARROWS | DLGC_WANTTAB | DLGC_WANTCHARS) as isize),
        WM_KILLFOCUS => {
            keyboard::lost(ui, HWND(wp.0 as *mut _));
            LRESULT(0)
        }
        WM_LBUTTONDOWN | WM_MOUSEMOVE | WM_LBUTTONUP => {
            input::mouse(hwnd, ui, msg, lp);
            LRESULT(0)
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
