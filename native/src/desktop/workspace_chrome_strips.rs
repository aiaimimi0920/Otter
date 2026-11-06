//! 端点条与曲线统一受连接组层序管理；不绑定原生 owner，节点身份与回收归 Strip。
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
            None,
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
        affinity_if_changed(
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
    let images: Vec<_> = ui
        .nodes
        .borrow()
        .iter()
        .map(|node| {
            let padding = if node.frame(ui.mode.get()) {
                desktop::pin_border_style::width(node.dpi) as i32
            } else {
                0
            };
            Rect {
                left: node.rect.left.saturating_sub(padding),
                top: node.rect.top.saturating_sub(padding),
                right: node.rect.right.saturating_add(padding),
                bottom: node.rect.bottom.saturating_add(padding),
            }
        })
        .collect();
    let mut pixels = 0i64;
    for (hwnd, endpoint, side) in handles {
        let Some(node) = ui
            .node(endpoint)
            .filter(|node| node.ports(ui.mode.get(), false))
        else {
            let _ = ShowWindow(hwnd, SW_HIDE);
            continue;
        };
        let areas: Vec<_> = ports
            .iter()
            .filter(|(p, _, _)| p.node == endpoint && p.side == side)
            .map(|(p, point, radius)| draw::port_rect(ui, *p, *point, *radius))
            .collect();
        let capsule = capsule::area(ui, hwnd);
        let mut all = areas.iter().chain(capsule.iter());
        let Some(first) = all.next() else {
            let _ = ShowWindow(hwnd, SW_HIDE);
            continue;
        };
        let mut bounds = *first;
        for area in all {
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
        affinity_if_changed(
            hwnd,
            if node.protected {
                WDA_EXCLUDEFROMCAPTURE
            } else {
                WDA_NONE
            },
        )?;
        let mut current = RECT::default();
        GetWindowRect(hwnd, &mut current)?;
        if current != bounds {
            SetWindowPos(
                hwnd,
                None,
                bounds.left,
                bounds.top,
                width as i32,
                height as i32,
                SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOOWNERZORDER,
            )?;
        }
        let region = CreateRectRgn(0, 0, 0, 0);
        if region.is_invalid() {
            return Err(Error::from_win32());
        }
        let result = (|| -> Result<bool> {
            for area in &areas {
                combine(region, bounds, *area, RGN_OR)?;
            }
            // region 同时扣除图片及可见白框；不依赖 owned 窗口层级去压住半透明边框。
            for image in &images {
                combine(
                    region,
                    bounds,
                    RECT {
                        left: image.left,
                        top: image.top,
                        right: image.right,
                        bottom: image.bottom,
                    },
                    RGN_DIFF,
                )?;
            }
            // 单字段 child 有独立的外侧布局，不随端口的遮挡裁剪丢失输入区域。
            if let Some(area) = capsule {
                combine(region, bounds, area, RGN_OR)?;
            }
            // 相同 region 也会生成 WM_WINDOWPOSCHANGED；静止端点不应随其他钉图反复重置。
            let previous = CreateRectRgn(0, 0, 0, 0);
            if previous.is_invalid() {
                return Err(Error::from_win32());
            }
            let unchanged =
                GetWindowRgn(hwnd, previous) != RGN_ERROR && EqualRgn(previous, region).as_bool();
            let _ = DeleteObject(HGDIOBJ(previous.0));
            if unchanged {
                return Ok(false);
            }
            if SetWindowRgn(hwnd, Some(region), true) == 0 {
                return Err(Error::from_win32());
            }
            Ok(true)
        })();
        if !matches!(result, Ok(true)) {
            let _ = DeleteObject(HGDIOBJ(region.0));
        }
        result?;
        capsule::place(ui, hwnd, bounds)?;
        if !ui.blocked() && !IsWindowVisible(hwnd).as_bool() {
            SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_SHOWWINDOW
                    | SWP_NOZORDER
                    | SWP_NOACTIVATE
                    | SWP_NOMOVE
                    | SWP_NOSIZE
                    | SWP_NOOWNERZORDER,
            )?;
        }
        InvalidateRect(Some(hwnd), None, false).ok()?;
    }
    Ok(())
}

unsafe fn combine(region: HRGN, bounds: RECT, area: RECT, mode: RGN_COMBINE_MODE) -> Result<()> {
    // 先求屏幕交集，既跳过无关图片，也保证转换到窗口坐标时差值受像素预算约束。
    let (left, top, right, bottom) = (
        area.left.max(bounds.left),
        area.top.max(bounds.top),
        area.right.min(bounds.right),
        area.bottom.min(bounds.bottom),
    );
    if left >= right || top >= bottom {
        return Ok(());
    }
    let part = CreateRectRgn(
        left - bounds.left,
        top - bounds.top,
        right - bounds.left,
        bottom - bounds.top,
    );
    if part.is_invalid() {
        return Err(Error::from_win32());
    }
    let combined = CombineRgn(Some(region), Some(region), Some(part), mode);
    let _ = DeleteObject(HGDIOBJ(part.0));
    if combined == RGN_ERROR {
        return Err(Error::from_win32());
    }
    Ok(())
}
