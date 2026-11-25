//! Manage left/right endpoint-strip Z-order by owning pin; no native-owner binding, with Strip owning node identity/reclamation.
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
            &HSTRING::from(crate::ui_text::text("workspace-ports-title")),
            WS_POPUP | WS_CLIPCHILDREN,
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
pub(super) unsafe fn refresh(ui: &Ui, translated: bool) -> Result<()> {
    let handles = ui.input_windows.borrow().clone();
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
            .filter(|node| node.ports(ui.mode.get(), node.hide_collapsed_endpoints))
        else {
            let _ = ShowWindow(hwnd, SW_HIDE);
            continue;
        };
        let geometry = port_visual::SideGeometry::new(ui, &node, side);
        let mut areas: Vec<_> = geometry
            .indices()
            .filter_map(|index| {
                let layout = geometry.layout(index, true)?;
                Some(geometry.rect(index, &layout))
            })
            .collect();
        if side == Side::Output {
            if let Some(auxiliary) = candidate_auxiliary::layout(ui, &node) {
                areas.push(auxiliary);
            }
        }
        if side == Side::Input && parameter_rail::active(ui, &node) {
            if let Some(track) = geometry.rail.as_ref().and_then(|r| r.track()) {
                areas.push(RECT {
                    left: track.x,
                    top: track.y,
                    right: track.x + track.width,
                    bottom: track.y + track.height,
                });
            }
        }
        let mut all = areas.iter();
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
        let redraw = !translated
            || !IsWindowVisible(hwnd).as_bool()
            || i64::from(current.right - current.left) != width
            || i64::from(current.bottom - current.top) != height;
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
            // Regions subtract both images and visible white borders, rather than relying on owned-window hierarchy to cover translucent borders.
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
            // Identical regions still generate WM_WINDOWPOSCHANGED; stationary endpoints must not repeatedly reset as other pins move.
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
        let region_changed = result?;
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
        // Pure translation already moved cached ports; redraw only on clip/size changes, not every endpoint when one pin drags.
        if redraw || region_changed {
            InvalidateRect(Some(hwnd), None, false).ok()?;
        }
    }
    parameter_rail::place(ui)
}

unsafe fn combine(region: HRGN, bounds: RECT, area: RECT, mode: RGN_COMBINE_MODE) -> Result<()> {
    // Find screen intersections first, skipping unrelated images and bounding window-coordinate differences by pixel budgets.
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
