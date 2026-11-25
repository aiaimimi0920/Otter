//! 左键5 CSS px阈值；旧边保留到有效drop提交，空白/取消/失去capture不删拓扑。
use super::*;
use windows::Win32::{Graphics::Gdi::ClientToScreen, UI::Input::KeyboardAndMouse::*};
pub(super) struct Press {
    port: Port,
    start: Point,
    pub current: Point,
    pub source: Option<Port>,
    pub dragged: bool,
    pub(super) action: Option<Port>,
    dpi: u32,
    catalog: Rc<Catalog>,
}
pub(super) fn hit(ui: &Ui, hwnd: HWND, point: Point) -> Option<Port> {
    let (_, endpoint, side) = ui
        .input_windows
        .borrow()
        .iter()
        .find(|(h, _, _)| *h == hwnd)
        .copied()?;
    let node = ui
        .node(endpoint)
        .filter(|node| node.ports(ui.mode.get(), node.hide_collapsed_endpoints))?;
    let geometry = port_visual::SideGeometry::new(ui, &node, side);
    geometry
        .indices()
        .filter_map(|index| {
            let layout = geometry.layout(index, true)?;
            Rect::from(geometry.rect(index, &layout))
                .contains(point)
                .then(|| (index, layout.connection().1))
        })
        .min_by_key(|(_, y)| (i64::from(*y) - i64::from(point.y)).abs())
        .map(|(index, _)| geometry.port(index))
}
impl From<RECT> for Rect {
    fn from(value: RECT) -> Self {
        Self {
            left: value.left,
            top: value.top,
            right: value.right,
            bottom: value.bottom,
        }
    }
}
unsafe fn current(ui: &Ui, port: Port) -> bool {
    ui.node(port.node).is_some_and(|node| {
        node.interactive && {
            let hwnd = HWND(node.hwnd as *mut _);
            IsWindow(Some(hwnd)).as_bool()
                && IsWindowVisible(hwnd).as_bool()
                && GetWindowLongPtrW(hwnd, GWLP_USERDATA) == node.marker
        }
    })
}
pub(super) unsafe fn mouse(hwnd: HWND, ui: &Ui, msg: u32, lp: LPARAM) {
    if ui.blocked() {
        ui.cancel();
        return;
    }
    let mut point = POINT {
        x: lp.0 as u16 as i16 as i32,
        y: (lp.0 >> 16) as u16 as i16 as i32,
    };
    if !ClientToScreen(hwnd, &mut point).as_bool() {
        ui.cancel();
        return;
    }
    let point = Point {
        x: point.x,
        y: point.y,
    };
    if msg == WM_LBUTTONDOWN {
        if ui.press.borrow().is_some() || ui.pending.borrow().is_some() {
            return;
        }
        let Some(port) = hit(ui, hwnd, point).filter(|port| current(ui, *port)) else {
            return;
        };
        let action = port_visual::action_hit(ui, port, point).then_some(port);
        if action.is_none() && !port_visual::anchor_hit(ui, port, point) {
            return;
        }
        let Some(scene) = ui.scene.borrow().clone() else {
            return;
        };
        ui.keyboard.retire();
        let source = if action.is_some() {
            None
        } else if port.side == Side::Output {
            Some(port)
        } else {
            scene
                .edges
                .iter()
                .find(|edge| edge.to == port)
                .map(|edge| edge.from)
        };
        *ui.press.borrow_mut() = Some(Press {
            port,
            source,
            start: point,
            current: point,
            dragged: false,
            action,
            dpi: ui.node(port.node).map_or(96, |node| node.dpi),
            catalog: scene.catalog.clone(),
        });
        SetCapture(hwnd);
        if GetCapture() != hwnd {
            ui.cancel();
        } else if action.is_some() {
            let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), None, false);
        }
    } else if msg == WM_MOUSEMOVE {
        if GetCapture() != hwnd {
            ui.cancel();
            return;
        }
        if let Some(press) = ui.press.borrow_mut().as_mut() {
            press.current = point;
            press.dragged |= model::moved(press.start, point, press.dpi);
        }
        if ui
            .press
            .borrow()
            .as_ref()
            .is_some_and(|p| p.action.is_some())
        {
            let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), None, false);
            return;
        }
        // 仅扩线层范围；端点 region 不随每个鼠标事件重建。
        if window::refresh_drawing(ui).is_err() {
            window::hide(ui);
        }
    } else if msg == WM_LBUTTONUP {
        let press = ui.press.borrow_mut().take();
        if GetCapture() == hwnd {
            let _ = ReleaseCapture();
        }
        let Some(mut press) = press else {
            return;
        };
        press.dragged |= model::moved(press.start, point, press.dpi);
        if let Some(port) = press.action {
            if !press.dragged && current(ui, port) && port_visual::action_hit(ui, port, point) {
                ports::visibility::request(ui, port);
            }
            let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), None, false);
            return;
        }
        if !press.dragged {
            if current(ui, press.port)
                && hit(ui, hwnd, point) == Some(press.port)
                && port_visual::anchor_hit(ui, press.port, point)
            {
                let expanded = disclosure::toggle(ui, press.port);
                if expanded {
                    parameter_rail::request(ui, press.port, false);
                }
            }
            if window::refresh(ui).is_err() {
                window::hide(ui);
            }
            return;
        }
        let Some(source) = press.source.filter(|source| current(ui, *source)) else {
            ui.repaint();
            return;
        };
        // 捕获后的坐标可以在任意窗口外；只接收当前自有端点条实际命中的输入，不点击/注入目标。
        let target_hwnd = WindowFromPoint(POINT {
            x: point.x,
            y: point.y,
        });
        let Some(target) = hit(ui, target_hwnd, point).filter(|p| {
            p.side == Side::Input && current(ui, *p) && port_visual::anchor_hit(ui, *p, point)
        }) else {
            ui.repaint();
            return;
        };
        if source.node == target.node {
            ui.repaint();
            return;
        }
        let Some(scene) = ui
            .scene
            .borrow()
            .clone()
            .filter(|scene| Rc::ptr_eq(&scene.catalog, &press.catalog))
        else {
            ui.repaint();
            return;
        };
        match scene
            .catalog
            .make_link(source.node, target.node, source.index, target.index)
        {
            Ok(Some(link))
                if ui.pending.borrow().is_none()
                    && PostMessageW(Some(scene.wake), WM_NULL, WPARAM(0), LPARAM(0)).is_ok() =>
            {
                *ui.pending.borrow_mut() = Some(desktop::link_catalog::Creation {
                    catalog: scene.catalog.clone(),
                    link,
                });
            }
            Err(error) => eprintln!("native-workspace-link: {error}"),
            _ => (),
        }
        ui.repaint();
    }
}
