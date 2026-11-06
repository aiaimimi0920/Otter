//! 每节点的穿透外框：紧邻宿主层序、独立 alpha；共享线层只保留连线和端口。
use super::*;
use std::sync::OnceLock;
use windows::Win32::{Graphics::Gdi::*, System::LibraryLoader::GetModuleHandleW};
const CLASS: windows::core::PCWSTR = w!("Otter.Native.PinBorder");
type PreparedRegion = (HRGN, (i32, i32, u32), u8);

pub(super) struct Border {
    pub hwnd: HWND,
    host: HWND,
    marker: isize,
    token: Box<Cell<u32>>,
    size: Cell<Option<(i32, i32, u32)>>,
    alpha: Cell<u8>,
    leased: Cell<bool>,
    prepared: RefCell<Option<PreparedRegion>>,
}
impl Border {
    #[cfg(test)]
    pub(super) fn alpha(&self) -> u8 {
        self.alpha.get()
    }
    pub fn matches(&self, node: &Node) -> bool {
        self.host.0 as isize == node.hwnd
            && self.marker == node.marker
            && unsafe { IsWindow(Some(self.hwnd)).as_bool() }
    }
    unsafe fn new(node: &Node) -> Result<Rc<Self>> {
        static REGISTERED: OnceLock<std::result::Result<(), windows::core::HRESULT>> =
            OnceLock::new();
        let instance = HINSTANCE(GetModuleHandleW(None)?.0);
        REGISTERED
            .get_or_init(|| {
                if RegisterClassW(&WNDCLASSW {
                    hInstance: instance,
                    lpszClassName: CLASS,
                    lpfnWndProc: Some(procedure),
                    ..Default::default()
                }) == 0
                {
                    Err(Error::from_win32().code())
                } else {
                    Ok(())
                }
            })
            .map_err(Error::from)?;
        let token = Box::new(Cell::new(96u32));
        let host = HWND(node.hwnd as *mut _);
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            CLASS,
            w!("Otter · 钉图外框"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            Some(host),
            None,
            Some(instance),
            Some((&*token as *const Cell<u32>).cast()),
        )?;
        Ok(Rc::new(Self {
            hwnd,
            host,
            marker: node.marker,
            token,
            size: Cell::new(None),
            alpha: Cell::new(0),
            leased: Cell::new(false),
            prepared: RefCell::new(None),
        }))
    }
    pub unsafe fn order(&self) -> Result<()> {
        if GetWindow(self.hwnd, GW_HWNDNEXT).ok() == Some(self.host) {
            return Ok(());
        }
        let above = GetWindow(self.host, GW_HWNDPREV).unwrap_or(HWND_TOP);
        if above != self.hwnd {
            SetWindowPos(
                self.hwnd,
                Some(above),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            )?;
        }
        Ok(())
    }
    unsafe fn refresh(&self, node: &Node, mode: Mode, preparing: bool) -> Result<()> {
        if self.leased.get() {
            return Ok(()); // 同一 HWND 正在显示完整图框；不得隐藏或裁剪其内容。
        }
        if !node.frame(mode) || (node.composite_frame && !preparing) {
            if IsWindowVisible(self.hwnd).as_bool() {
                let _ = ShowWindow(self.hwnd, SW_HIDE);
            }
            return Ok(());
        }
        if GetWindowLongPtrW(self.host, GWLP_USERDATA) != self.marker {
            return Err(Error::from(E_HANDLE));
        }
        let padding = desktop::pin_border_style::width(node.dpi);
        self.token.set(node.dpi);
        let width = node.rect.right - node.rect.left + 2 * padding as i32;
        let height = node.rect.bottom - node.rect.top + 2 * padding as i32;
        let resized = self.size.get() != Some((width, height, padding));
        if resized {
            let outer = CreateRectRgn(0, 0, width, height);
            let inner = CreateRectRgn(
                padding as i32,
                padding as i32,
                width - padding as i32,
                height - padding as i32,
            );
            if outer.is_invalid() || inner.is_invalid() {
                if !outer.is_invalid() {
                    let _ = DeleteObject(outer.into());
                }
                if !inner.is_invalid() {
                    let _ = DeleteObject(inner.into());
                }
                return Err(Error::from_win32());
            }
            let combined = CombineRgn(Some(outer), Some(outer), Some(inner), RGN_DIFF);
            let _ = DeleteObject(inner.into());
            if combined == RGN_ERROR || SetWindowRgn(self.hwnd, Some(outer), false) == 0 {
                let _ = DeleteObject(outer.into());
                return Err(Error::from_win32());
            }
        }
        let repaint = resized || self.alpha.get() != node.border_alpha;
        let mut affinity = 0;
        GetWindowDisplayAffinity(self.hwnd, &mut affinity)?;
        let desired = if node.protected {
            WDA_EXCLUDEFROMCAPTURE
        } else {
            WDA_NONE
        };
        if affinity != desired.0 {
            SetWindowDisplayAffinity(self.hwnd, desired)?;
        }
        if !preparing {
            self.order()?;
        }
        let mut current = RECT::default();
        GetWindowRect(self.hwnd, &mut current)?;
        let x = node.rect.left - padding as i32;
        let y = node.rect.top - padding as i32;
        if repaint {
            let bitmap = frame_bitmap(node)?;
            UpdateLayeredWindow(
                self.hwnd,
                None,
                Some(&POINT { x, y }),
                Some(&SIZE {
                    cx: width,
                    cy: height,
                }),
                Some(bitmap.dc()),
                Some(&POINT::default()),
                COLORREF(0),
                Some(&BLENDFUNCTION {
                    BlendOp: AC_SRC_OVER as u8,
                    BlendFlags: 0,
                    SourceConstantAlpha: 255,
                    AlphaFormat: AC_SRC_ALPHA as u8,
                }),
                ULW_ALPHA,
            )?;
            self.alpha.set(node.border_alpha);
            self.size.set(Some((width, height, padding)));
        } else if current
            != (RECT {
                left: x,
                top: y,
                right: x + width,
                bottom: y + height,
            })
        {
            SetWindowPos(
                self.hwnd,
                None,
                x,
                y,
                width,
                height,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            )?;
        }
        if !preparing && !IsWindowVisible(self.hwnd).as_bool() {
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
                    | SWP_NOZORDER
                    | SWP_NOACTIVATE
                    | SWP_NOOWNERZORDER,
            )?;
        }
        Ok(())
    }
}
pub(super) fn lease(ui: &Ui, host: HWND) -> Option<HWND> {
    let node = ui
        .nodes
        .borrow()
        .iter()
        .find(|n| n.hwnd == host.0 as isize)
        .cloned()?;
    if !node.frame(ui.mode.get()) {
        return None;
    }
    let borders = ui.borders.borrow();
    let border = borders.iter().find(|b| b.matches(&node))?;
    if border.leased.replace(true) {
        return None;
    }
    border.size.set(None);
    Some(border.hwnd)
}
pub(super) fn release(ui: &Ui, hwnd: HWND) {
    if let Some(border) = ui.borders.borrow().iter().find(|b| b.hwnd == hwnd) {
        border.leased.set(false);
    }
}
pub(super) fn has_content(ui: &Ui, hwnd: HWND) -> bool {
    ui.borders
        .borrow()
        .iter()
        .any(|b| b.hwnd == hwnd && b.leased.get())
}
/// 装饰隐藏前先归还静止图像；不能把唯一可见的内容当成纯外框一起藏掉。
pub(super) unsafe fn restore_stationary(ui: &Ui) {
    let owners: Vec<_> = ui
        .borders
        .borrow()
        .iter()
        .filter(|b| b.leased.get())
        .map(|b| (b.host, b.marker))
        .collect();
    for (host, marker) in owners {
        // 只有普通文档的 pin preview 能取得租约；Live 的 userdata 类型不同。
        if marker != 0 && GetWindowLongPtrW(host, GWLP_USERDATA) == marker {
            let state = &*(marker as *const desktop::State);
            if desktop::pin_window::stationary::active(state) {
                desktop::pin_window::animation::stop(host, state);
            }
        }
    }
}
impl Drop for Border {
    fn drop(&mut self) {
        unsafe {
            if let Some((region, _, _)) = self.prepared.get_mut().take() {
                let _ = DeleteObject(region.into());
            }
            if GetWindowLongPtrW(self.hwnd, GWLP_USERDATA)
                == &*self.token as *const Cell<u32> as isize
            {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}
pub(super) unsafe fn sync(ui: &Ui, nodes: &[Node]) -> Result<()> {
    let mut borders = ui.borders.borrow().clone();
    borders.retain(|border| nodes.iter().any(|node| border.matches(node)));
    for node in nodes {
        if !borders.iter().any(|border| border.matches(node)) {
            borders.push(Border::new(node)?);
        }
    }
    // DestroyWindow 可重入；替换后释放旧 owner，绝不持 RefCell 借用跨窗口调用。
    drop(ui.borders.replace(borders));
    Ok(())
}
pub(super) unsafe fn refresh(ui: &Ui) -> Result<()> {
    let borders = ui.borders.borrow().clone();
    let nodes = ui.nodes.borrow().clone();
    for border in borders {
        if let Some(node) = nodes.iter().find(|node| border.matches(node)) {
            border.refresh(node, ui.mode.get(), false)?;
        } else if IsWindowVisible(border.hwnd).as_bool() {
            let _ = ShowWindow(border.hwnd, SW_HIDE);
        }
    }
    Ok(())
}
/// 代理仍独占画面时准备隐藏外框，不在宿主曝光后再创建 region、设置 alpha 或定位。
pub(super) unsafe fn prepare(ui: &Ui, host: HWND) -> Result<()> {
    let node = ui
        .nodes
        .borrow()
        .iter()
        .find(|node| node.hwnd == host.0 as isize)
        .cloned();
    let Some(node) = node.filter(|node| node.composite_frame) else {
        return Ok(());
    };
    let border = ui
        .borders
        .borrow()
        .iter()
        .find(|border| border.matches(&node))
        .cloned();
    if let Some(border) = border {
        if border.leased.get() {
            // 只准备裁剪区域，不改变正在显示的整帧。
            let padding = desktop::pin_border_style::width(node.dpi);
            let width = node.rect.right - node.rect.left + 2 * padding as i32;
            let height = node.rect.bottom - node.rect.top + 2 * padding as i32;
            let outer = CreateRectRgn(0, 0, width, height);
            let inner = CreateRectRgn(
                padding as i32,
                padding as i32,
                width - padding as i32,
                height - padding as i32,
            );
            if outer.is_invalid() || inner.is_invalid() {
                if !outer.is_invalid() {
                    let _ = DeleteObject(outer.into());
                }
                if !inner.is_invalid() {
                    let _ = DeleteObject(inner.into());
                }
                return Err(Error::from_win32());
            }
            let result = CombineRgn(Some(outer), Some(outer), Some(inner), RGN_DIFF);
            let _ = DeleteObject(inner.into());
            if result == RGN_ERROR {
                let _ = DeleteObject(outer.into());
                return Err(Error::from_win32());
            }
            if let Some((old, _, _)) =
                border
                    .prepared
                    .replace(Some((outer, (width, height, padding), node.border_alpha)))
            {
                let _ = DeleteObject(old.into());
            }
        } else {
            border.refresh(&node, ui.mode.get(), true)?;
        }
    }
    Ok(())
}
/// 保留整帧已有白框像素，仅裁掉中心；成功前预览仍拥有窗口和输入。
pub(super) unsafe fn commit(ui: &Ui, host: HWND) -> Result<()> {
    let border = ui
        .borders
        .borrow()
        .iter()
        .find(|b| b.host == host && b.leased.get())
        .cloned();
    let Some(border) = border else {
        return Ok(());
    };
    let prepared = border.prepared.borrow_mut().take();
    let Some((region, size, alpha)) = prepared else {
        return Err(Error::new(E_FAIL, "Missing border handoff region"));
    };
    if SetWindowRgn(border.hwnd, Some(region), false) == 0 {
        *border.prepared.borrow_mut() = Some((region, size, alpha));
        return Err(Error::from_win32());
    }
    border.size.set(Some(size));
    Ok(())
}
fn frame_bitmap(node: &Node) -> Result<crate::software_dib::Bitmap> {
    let padding = desktop::pin_border_style::width(node.dpi);
    crate::software_dib::Bitmap::border(
        windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U {
            width: (node.rect.right - node.rect.left) as u32 + 2 * padding,
            height: (node.rect.bottom - node.rect.top) as u32 + 2 * padding,
        },
        padding,
        node.border_alpha,
    )
}
pub(super) unsafe fn order(ui: &Ui, host: HWND) {
    let borders = ui.borders.borrow().clone();
    for border in borders
        .iter()
        .filter(|b| b.host == host && IsWindowVisible(b.hwnd).as_bool())
    {
        if let Err(error) = border.order() {
            ui.fail(error);
        }
    }
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            SetWindowLongPtrW(
                hwnd,
                GWLP_USERDATA,
                (*(lp.0 as *const CREATESTRUCTW)).lpCreateParams as isize,
            );
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let _ = BeginPaint(hwnd, &mut paint);
            // ULW 的完整预乘帧是唯一像素来源，不能再混用 GDI 重定向表面。
            let _ = EndPaint(hwnd, &paint);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
