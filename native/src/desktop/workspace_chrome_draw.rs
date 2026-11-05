//! 只画公共端口名/外框/Bezier 连线，不读取或拷贝图片；GDI 对象逐次 RAII 回收。
use super::*;
use windows::Win32::Graphics::Gdi::*;
const BLUE: COLORREF = COLORREF(0xd4b606);
const YELLOW: COLORREF = COLORREF(0x38ffd9);
const TEXT: COLORREF = COLORREF(0xeff8f7);
pub(super) fn ports(ui: &Ui) -> Vec<(Port, Point, i32)> {
    let mut ports = Vec::new();
    for node in ui
        .nodes
        .borrow()
        .iter()
        .filter(|node| node.ports(ui.mode.get(), false))
    {
        for (side, count) in [(Side::Input, node.inputs), (Side::Output, node.outputs)] {
            for index in 0..count {
                if let Some(point) = node.point(side, index) {
                    ports.push((
                        Port {
                            node: node.endpoint,
                            side,
                            index,
                        },
                        point,
                        node.radius(),
                    ));
                }
            }
        }
    }
    ports
}
pub(super) fn port_rect(ui: &Ui, port: Port, p: Point, radius: i32) -> RECT {
    let expanded = ui.node(port.node).is_some_and(|node| node.expanded)
        || ui.disclosed.borrow().contains(&port)
        || ui.keyboard.focused() == Some(port);
    let extra = if expanded { radius * 7 } else { 0 };
    RECT {
        left: p
            .x
            .saturating_sub(radius)
            .saturating_sub(if port.side == Side::Input { extra } else { 0 }),
        top: p.y.saturating_sub(radius),
        right: p
            .x
            .saturating_add(radius)
            .saturating_add(if port.side == Side::Output { extra } else { 0 }),
        bottom: p.y.saturating_add(radius),
    }
}
struct Object(HGDIOBJ);
impl Drop for Object {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.0);
        }
    }
}
unsafe fn line(dc: HDC, origin: POINT, a: Point, b: Point, color: COLORREF, dashed: bool) {
    let pen = CreatePen(
        if dashed { PS_DASH } else { PS_SOLID },
        if dashed { 1 } else { 2 },
        color,
    );
    if pen.is_invalid() {
        return;
    }
    let _pen = Object(HGDIOBJ(pen.0));
    let old = SelectObject(dc, HGDIOBJ(pen.0));
    let shift = ((i64::from(b.x) - i64::from(a.x)).abs() / 2).clamp(40, 160) as i32;
    let p = |x: i32, y: i32| POINT {
        x: x.saturating_sub(origin.x),
        y: y.saturating_sub(origin.y),
    };
    let points = [
        p(a.x, a.y),
        p(a.x.saturating_add(shift), a.y),
        p(b.x.saturating_sub(shift), b.y),
        p(b.x, b.y),
    ];
    let _ = PolyBezier(dc, &points);
    SelectObject(dc, old);
}
pub(super) unsafe fn paint(hwnd: HWND, ui: &Ui) {
    let mut paint = PAINTSTRUCT::default();
    let dc = BeginPaint(hwnd, &mut paint);
    if dc.is_invalid() {
        return;
    }
    let _scope = Paint(hwnd, paint);
    let mut bounds = RECT::default();
    if GetWindowRect(hwnd, &mut bounds).is_err() {
        return;
    }
    let origin = POINT {
        x: bounds.left,
        y: bounds.top,
    };
    SetDCBrushColor(dc, COLORREF(0));
    FillRect(dc, &paint.rcPaint, HBRUSH(GetStockObject(DC_BRUSH).0));
    if ui.blocked() {
        return;
    }
    SetBkMode(dc, TRANSPARENT);
    if hwnd == ui.drawing.get() {
        for node in ui.nodes.borrow().iter().filter(|n| n.frame(ui.mode.get())) {
            let rect = RECT {
                left: node.rect.left.saturating_sub(origin.x),
                top: node.rect.top.saturating_sub(origin.y),
                right: node.rect.right.saturating_sub(origin.x),
                bottom: node.rect.bottom.saturating_sub(origin.y),
            };
            SetDCBrushColor(dc, COLORREF(0x9f9a92));
            FrameRect(dc, &rect, HBRUSH(GetStockObject(DC_BRUSH).0));
        }
        if let Some(scene) = ui.scene.borrow().clone() {
            for edge in &scene.edges {
                let (Some(from), Some(to)) = (ui.node(edge.from.node), ui.node(edge.to.node))
                else {
                    continue;
                };
                if !ui.visible(edge.from) || !ui.visible(edge.to) {
                    continue;
                }
                let (Some(a), Some(b)) = (ui.point(edge.from), ui.point(edge.to)) else {
                    continue;
                };
                let (body, dashed) = model::edge_style(ui.mode.get(), &from, &to);
                if body {
                    line(dc, origin, a, b, BLUE, false);
                }
                if dashed {
                    line(dc, origin, a, b, TEXT, true);
                }
            }
        }
        if let Some(press) = ui.press.borrow().as_ref().filter(|press| press.dragged) {
            if let Some(start) = press.source.and_then(|source| ui.point(source)) {
                line(dc, origin, start, press.current, YELLOW, true);
            }
        }
        if let (Some(source), Some(target)) = (ui.keyboard.source(), ui.keyboard.focused()) {
            if let (Some(a), Some(b)) = (ui.point(source), ui.point(target)) {
                line(dc, origin, a, b, YELLOW, true);
            }
        }
    } else {
        let old_pen = SelectObject(dc, GetStockObject(DC_PEN));
        let old_brush = SelectObject(dc, GetStockObject(DC_BRUSH));
        let old_font = SelectObject(dc, GetStockObject(DEFAULT_GUI_FONT));
        SetDCPenColor(dc, BLUE);
        let binding = ui
            .input_windows
            .borrow()
            .iter()
            .find(|(h, _, _)| *h == hwnd)
            .copied();
        for (port, point, radius) in ports(ui).into_iter().filter(|(p, _, _)| {
            binding.is_some_and(|(_, node, side)| p.node == node && p.side == side)
        }) {
            SetDCPenColor(
                dc,
                if ui.keyboard.focused() == Some(port) {
                    YELLOW
                } else {
                    BLUE
                },
            );
            let mut rect = port_rect(ui, port, point, radius);
            rect.left = rect.left.saturating_sub(origin.x);
            rect.right = rect.right.saturating_sub(origin.x);
            rect.top = rect.top.saturating_sub(origin.y);
            rect.bottom = rect.bottom.saturating_sub(origin.y);
            SetDCBrushColor(dc, COLORREF(0x201711));
            if rect.right - rect.left > 2 * radius {
                let _ = Rectangle(dc, rect.left, rect.top, rect.right, rect.bottom);
            } else {
                let _ = Ellipse(dc, rect.left, rect.top, rect.right, rect.bottom);
            }
            if rect.right - rect.left > 2 * radius {
                if let Some(scene) = ui.scene.borrow().clone() {
                    if let Some(endpoint) = scene.catalog.endpoints.get(port.node) {
                        let names = if port.side == Side::Input {
                            &endpoint.inputs
                        } else {
                            &endpoint.outputs
                        };
                        if let Some(name) = names.get(port.index) {
                            let mut text: Vec<u16> = name
                                .chars()
                                .take(64)
                                .collect::<String>()
                                .encode_utf16()
                                .collect();
                            SetTextColor(dc, TEXT);
                            DrawTextW(
                                dc,
                                &mut text,
                                &mut rect,
                                DT_CENTER
                                    | DT_VCENTER
                                    | DT_SINGLELINE
                                    | DT_NOPREFIX
                                    | DT_END_ELLIPSIS,
                            );
                        }
                    }
                }
            }
        }
        SelectObject(dc, old_font);
        SelectObject(dc, old_brush);
        SelectObject(dc, old_pen);
    }
}
struct Paint(HWND, PAINTSTRUCT);
impl Drop for Paint {
    fn drop(&mut self) {
        unsafe {
            let _ = EndPaint(self.0, &self.1);
        }
    }
}
