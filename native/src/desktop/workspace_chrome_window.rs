//! 共享连线层、每节点独立外框及至多两个端点条；不为每端口建立 HWND。
use super::*;
use std::sync::OnceLock;
use windows::Win32::{Graphics::Gdi::*, System::LibraryLoader::GetModuleHandleW};
const CLASS: windows::core::PCWSTR = w!("Otter.Native.WorkspaceChrome");
pub(super) const MAX_PIXELS: i64 = 33_554_432;
#[path = "workspace_chrome_strips.rs"]
mod strips;
pub(super) struct Windows {
    ui: Rc<Ui>,
    drawing: HWND,
    inputs: Vec<strips::Strip>,
    _paint_session: draw::PaintSession,
}
impl Windows {
    pub unsafe fn new(ui: Rc<Ui>) -> Result<Self> {
        register()?;
        let mut owner = Self {
            ui,
            drawing: HWND::default(),
            inputs: Vec::new(),
            _paint_session: draw::PaintSession::new()?,
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
        Ok(owner)
    }
    pub unsafe fn sync(&mut self) -> Result<()> {
        let nodes = self.ui.nodes.borrow().clone();
        if nodes.len() > crate::workspace::MAX_UNITS {
            return Err(Error::from(E_OUTOFMEMORY));
        }
        border::sync(&self.ui, &nodes)?;
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
            drop(self.ui.borders.replace(Vec::new()));
            self.ui.input_windows.borrow_mut().clear();
            self.ui.drawing.set(HWND::default());
            self.ui.surface.borrow_mut().take();
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
    border::restore_stationary(ui);
    capsule::cancel(ui);
    ui.keyboard.retire();
    ui.cancel();
    ui.pending.borrow_mut().take();
    for hwnd in ui.handles().into_iter().filter(|h| !h.is_invalid()) {
        if !border::has_content(ui, hwnd) {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
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
    // 线层已有富余空间时保持 HWND/DIB 尺寸，只提交像素；连续滚轮不能每帧重建全局外框表面。
    let mut existing = RECT::default();
    if ui.surface.borrow().is_some() && GetWindowRect(ui.drawing.get(), &mut existing).is_ok() {
        let needed = Rect {
            left: nodes
                .iter()
                .map(|n| n.rect.left.saturating_sub(160))
                .min()
                .unwrap_or(rect.left)
                .max(vx),
            top: nodes
                .iter()
                .map(|n| n.rect.top.saturating_sub(16))
                .min()
                .unwrap_or(rect.top)
                .max(vy),
            right: nodes
                .iter()
                .map(|n| n.rect.right.saturating_add(160))
                .max()
                .unwrap_or(rect.right)
                .min(vx.saturating_add(GetSystemMetrics(SM_CXVIRTUALSCREEN))),
            bottom: nodes
                .iter()
                .map(|n| n.rect.bottom.saturating_add(16))
                .max()
                .unwrap_or(rect.bottom)
                .min(vy.saturating_add(GetSystemMetrics(SM_CYVIRTUALSCREEN))),
        };
        if ui.press.borrow().is_none()
            && existing.left <= needed.left
            && existing.top <= needed.top
            && existing.right >= needed.right
            && existing.bottom >= needed.bottom
            && existing.left >= vx
            && existing.top >= vy
            && existing.right <= vx.saturating_add(GetSystemMetrics(SM_CXVIRTUALSCREEN))
            && existing.bottom <= vy.saturating_add(GetSystemMetrics(SM_CYVIRTUALSCREEN))
        {
            rect = Rect {
                left: existing.left,
                top: existing.top,
                right: existing.right,
                bottom: existing.bottom,
            };
        }
    }
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
    affinity_if_changed(drawing, affinity)?;
    if ui.blocked() {
        hide(ui);
        return Ok(());
    }
    // 首次显示/新节点发布前先确定层序，不能先把新连线画到图片上再下移。
    order_drawing(ui)?;
    draw::present(ui, rect)?;
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
    // 已提交完整帧；不要让空闲 WM_PAINT 再复制一遍共享线层。
    ValidateRect(Some(drawing), None).ok()?;
    Ok(())
}
struct OrderGuard<'a>(&'a Cell<bool>);
impl Drop for OrderGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
pub(super) unsafe fn order_drawing(ui: &Ui) -> Result<()> {
    let drawing = ui.drawing.get();
    if drawing.is_invalid() || ui.blocked() || ui.ordering.get() {
        return Ok(());
    }
    // EndDeferWindowPos 同步回调组内每个 HWND，不能在尚未完成的同一层序事务中再开一批。
    ui.ordering.set(true);
    let _guard = OrderGuard(&ui.ordering);
    let mut hosts: HashSet<_> = ui
        .nodes
        .borrow()
        .iter()
        .filter(|node| {
            let hwnd = HWND(node.hwnd as *mut _);
            IsWindowVisible(hwnd).as_bool() && GetWindowLongPtrW(hwnd, GWLP_USERDATA) == node.marker
        })
        .map(|node| node.hwnd)
        .collect();
    if hosts.is_empty() {
        return Ok(());
    }
    // 端点不绑定 HWND owner，避免系统自动升层；连接组统一位于所有图片下。
    let layers: Vec<_> = ui
        .input_windows
        .borrow()
        .iter()
        .map(|(hwnd, _, _)| *hwnd)
        .filter(|hwnd| IsWindow(Some(*hwnd)).as_bool())
        .chain([drawing])
        .collect();
    let members: HashSet<_> = layers.iter().map(|hwnd| hwnd.0 as isize).collect();
    let mut cursor = GetTopWindow(None)?;
    let mut connection_above = false;
    for _ in 0..4096 {
        if cursor.is_invalid() {
            break;
        }
        connection_above |= members.contains(&(cursor.0 as isize));
        if hosts.remove(&(cursor.0 as isize)) && hosts.is_empty() {
            if connection_above {
                // 仅违约时批量下移连接组，不提升/重排钉图与独立白框，不重绘静止像素。
                let mut batch = BeginDeferWindowPos(layers.len() as i32)?;
                for hwnd in layers {
                    batch = DeferWindowPos(
                        batch,
                        hwnd,
                        Some(cursor),
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOREDRAW | SWP_NOOWNERZORDER,
                    )?;
                    cursor = hwnd;
                }
                EndDeferWindowPos(batch)?;
            }
            return Ok(());
        }
        cursor = GetWindow(cursor, GW_HWNDNEXT).unwrap_or_default();
    }
    Ok(())
}
unsafe fn affinity_if_changed(hwnd: HWND, desired: WINDOW_DISPLAY_AFFINITY) -> Result<()> {
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
            // 焦点/child 激活可能在 WM_ACTIVATE 后再次升层；以真实层序通知为准，不夺走焦点。
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
