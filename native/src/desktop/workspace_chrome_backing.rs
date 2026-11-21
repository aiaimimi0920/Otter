//! 每钉图的穿透 Tab 底板；与所属端点同层，复用原完整帧表面、像素预算及截图围栏。
use super::*;
use draw::backdrops::Backdrops;
use windows::Win32::{Graphics::Gdi::*, System::LibraryLoader::GetModuleHandleW};

pub(super) struct Backing {
    pub hwnd: HWND,
    host: isize,
    marker: isize,
    ui_marker: isize,
    pub(super) surface: RefCell<Option<draw::Surface>>,
    content: RefCell<Option<(Rect, Backdrops)>>,
}
impl Backing {
    pub(super) fn matches(&self, node: &Node) -> bool {
        self.host == node.hwnd
            && self.marker == node.marker
            && unsafe { IsWindow(Some(self.hwnd)).as_bool() }
    }
    unsafe fn new(ui: &Ui, node: &Node) -> Result<Rc<Self>> {
        let pointer = ui as *const Ui;
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT,
            window::CLASS,
            w!("Otter · 参数底板"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(HINSTANCE(GetModuleHandleW(None)?.0)),
            Some(pointer.cast()),
        )?;
        let result = Rc::new(Self {
            hwnd,
            host: node.hwnd,
            marker: node.marker,
            ui_marker: pointer as isize,
            surface: RefCell::default(),
            content: RefCell::default(),
        });
        SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)?;
        Ok(result)
    }
    unsafe fn refresh(&self, ui: &Ui, node: &Node, content: Backdrops) -> Result<()> {
        let rect = content.bounds().and_then(|rect| screen_bounds(rect));
        let Some(rect) = rect.filter(|_| node.rect.valid() && !ui.blocked()) else {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
            self.content.borrow_mut().take();
            self.surface.borrow_mut().take();
            return Ok(());
        };
        window::affinity_if_changed(
            self.hwnd,
            if node.protected {
                WDA_EXCLUDEFROMCAPTURE
            } else {
                WDA_NONE
            },
        )?;
        // refresh_drawing 已在本轮任何像素/首显之前统一排好所有节点的层序。
        let content = (rect, content);
        if self.content.borrow().as_ref() != Some(&content) {
            let size = SIZE {
                cx: rect.right - rect.left,
                cy: rect.bottom - rect.top,
            };
            let cached = self.surface.borrow_mut().take().filter(|s| s.fits(size));
            let surface = match cached {
                Some(s) => s,
                None => draw::Surface::new(size)?,
            };
            let dc = surface.dc();
            SetDCBrushColor(dc, COLORREF(0));
            FillRect(
                dc,
                &RECT {
                    left: 0,
                    top: 0,
                    right: size.cx,
                    bottom: size.cy,
                },
                HBRUSH(GetStockObject(DC_BRUSH).0),
            );
            content.1.paint(
                ui,
                dc,
                POINT {
                    x: rect.left,
                    y: rect.top,
                },
            )?;
            surface.present(self.hwnd, rect)?;
            *self.surface.borrow_mut() = Some(surface);
            *self.content.borrow_mut() = Some(content);
        }
        if !IsWindowVisible(self.hwnd).as_bool() {
            SetWindowPos(
                self.hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_SHOWWINDOW
                    | SWP_NOMOVE
                    | SWP_NOSIZE
                    | SWP_NOACTIVATE
                    | SWP_NOZORDER
                    | SWP_NOOWNERZORDER,
            )?;
        }
        ValidateRect(Some(self.hwnd), None).ok()
    }
}
// 窗口无业务 owner/线程/计时器；Ui 的集合退出时同步销毁 HWND 和有界 DIB。
impl Drop for Backing {
    fn drop(&mut self) {
        unsafe {
            if GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) == self.ui_marker {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}
unsafe fn screen_bounds(rect: Rect) -> Option<Rect> {
    let (x, y) = (
        GetSystemMetrics(SM_XVIRTUALSCREEN),
        GetSystemMetrics(SM_YVIRTUALSCREEN),
    );
    let result = Rect {
        left: rect.left.max(x),
        top: rect.top.max(y),
        right: rect
            .right
            .min(x.saturating_add(GetSystemMetrics(SM_CXVIRTUALSCREEN))),
        bottom: rect
            .bottom
            .min(y.saturating_add(GetSystemMetrics(SM_CYVIRTUALSCREEN))),
    };
    result.valid().then_some(result)
}
pub(super) unsafe fn sync(ui: &Ui, nodes: &[Node]) -> Result<()> {
    let mut backings = ui.backings.borrow().clone();
    backings.retain(|b| nodes.iter().any(|n| b.matches(n)));
    for node in nodes {
        if !backings.iter().any(|b| b.matches(node)) {
            backings.push(Backing::new(ui, node)?);
        }
    }
    drop(ui.backings.replace(backings));
    Ok(())
}
pub(super) unsafe fn refresh(ui: &Ui) -> Result<()> {
    let backings = ui.backings.borrow().clone();
    let nodes = ui.nodes.borrow().clone();
    let content = draw::backdrops::collect(ui);
    for backing in backings {
        if let Some(node) = nodes.iter().find(|n| backing.matches(n)) {
            backing.refresh(ui, node, content.for_node(node.endpoint))?;
        } else {
            let _ = ShowWindow(backing.hwnd, SW_HIDE);
        }
    }
    Ok(())
}
pub(super) fn owns(ui: &Ui, hwnd: HWND) -> bool {
    ui.backings.borrow().iter().any(|b| b.hwnd == hwnd)
}
pub(super) fn moving(ui: &Ui, host: HWND) -> Vec<HWND> {
    ui.backings
        .borrow()
        .iter()
        .filter(|b| b.host == host.0 as isize && unsafe { IsWindow(Some(b.hwnd)).as_bool() })
        .map(|b| b.hwnd)
        .collect()
}
