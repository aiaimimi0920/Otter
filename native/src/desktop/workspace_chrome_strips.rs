//! 每个端点条按宿主 owned Z-order 跟随；稀疏 region 以外不占用任何应用输入区域。
use super::*;
pub(super) struct Strip {
    pub hwnd: HWND,
    pub endpoint: usize,
    pub side: Side,
    host: isize,
    marker: isize,
    ui: Rc<Ui>,
}
impl Strip {
    pub unsafe fn new(ui: Rc<Ui>, node: &Node, side: Side) -> Result<Self> {
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            CLASS,
            w!("Otter · 端口"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            Some(HWND(node.hwnd as *mut _)),
            None,
            Some(HINSTANCE(GetModuleHandleW(None)?.0)),
            Some(Rc::as_ptr(&ui).cast()),
        )?;
        let owner = Self {
            hwnd,
            endpoint: node.endpoint,
            side,
            host: node.hwnd,
            marker: node.marker,
            ui,
        };
        SetWindowDisplayAffinity(
            hwnd,
            if node.protected {
                WDA_EXCLUDEFROMCAPTURE
            } else {
                WDA_NONE
            },
        )?;
        SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_COLORKEY)?;
        Ok(owner)
    }
    pub fn matches(&self, node: &Node) -> bool {
        self.endpoint == node.endpoint
            && self.host == node.hwnd
            && self.marker == node.marker
            && unsafe { IsWindow(Some(self.hwnd)).as_bool() }
    }
}
impl Drop for Strip {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(Some(self.hwnd)).as_bool()
                && GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) == Rc::as_ptr(&self.ui) as isize
            {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}
pub(super) unsafe fn refresh(ui: &Ui) -> Result<()> {
    let handles = ui.input_windows.borrow().clone();
    let ports = draw::ports(ui);
    let mut pixels = 0i64;
    for (hwnd, endpoint, side) in handles {
        let Some(node) = ui
            .node(endpoint)
            .filter(|node| node.ports(ui.mode.get(), false))
        else {
            let _ = ShowWindow(hwnd, SW_HIDE);
            continue;
        };
        let mut areas: Vec<_> = ports
            .iter()
            .filter(|(p, _, _)| p.node == endpoint && p.side == side)
            .map(|(p, point, radius)| draw::port_rect(ui, *p, *point, *radius))
            .collect();
        if let Some(area) = capsule::area(ui, hwnd) {
            areas.push(area);
        }
        let Some(first) = areas.first() else {
            let _ = ShowWindow(hwnd, SW_HIDE);
            continue;
        };
        let mut bounds = *first;
        for area in &areas {
            bounds.left = bounds.left.min(area.left);
            bounds.top = bounds.top.min(area.top);
            bounds.right = bounds.right.max(area.right);
            bounds.bottom = bounds.bottom.max(area.bottom);
        }
        let (width, height) = (
            i64::from(bounds.right) - i64::from(bounds.left),
            i64::from(bounds.bottom) - i64::from(bounds.top),
        );
        pixels = pixels
            .checked_add(width * height)
            .ok_or_else(|| Error::from(E_OUTOFMEMORY))?;
        if width <= 0 || height <= 0 || width > 32768 || height > 32768 || pixels > MAX_PIXELS {
            return Err(Error::from(E_OUTOFMEMORY));
        }
        SetWindowDisplayAffinity(
            hwnd,
            if node.protected {
                WDA_EXCLUDEFROMCAPTURE
            } else {
                WDA_NONE
            },
        )?;
        SetWindowPos(
            hwnd,
            None,
            bounds.left,
            bounds.top,
            width as i32,
            height as i32,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )?;
        let region = CreateRectRgn(0, 0, 0, 0);
        if region.is_invalid() {
            return Err(Error::from_win32());
        }
        let result = (|| -> Result<()> {
            for area in &areas {
                let part = CreateRectRgn(
                    area.left - bounds.left,
                    area.top - bounds.top,
                    area.right - bounds.left,
                    area.bottom - bounds.top,
                );
                if part.is_invalid() {
                    return Err(Error::from_win32());
                }
                let combined = CombineRgn(Some(region), Some(region), Some(part), RGN_OR);
                let _ = DeleteObject(HGDIOBJ(part.0));
                if combined == RGN_ERROR {
                    return Err(Error::from_win32());
                }
            }
            if SetWindowRgn(hwnd, Some(region), true) == 0 {
                return Err(Error::from_win32());
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = DeleteObject(HGDIOBJ(region.0));
        }
        result?;
        capsule::place(ui, hwnd, bounds)?;
        if !ui.blocked() {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
    }
    Ok(())
}
