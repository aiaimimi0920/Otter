//! 只画公共端口名/外框/Bezier 连线，不读取或拷贝图片；GDI 对象逐次 RAII 回收。
use super::*;
use crate::desktop::buffered_paint as buffered;
pub(super) use buffered::Session as PaintSession;
use windows::Win32::Graphics::Gdi::*;
#[path = "workspace_chrome_surface.rs"]
mod surface;
pub(super) use surface::Surface;
const BLUE: COLORREF = COLORREF(0xd4b606);
const YELLOW: COLORREF = COLORREF(0x38ffd9);
const TEXT: COLORREF = COLORREF(0xeff8f7);
#[cfg(test)]
#[path = "workspace_chrome_frame_tests.rs"]
mod frame_tests;
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
#[cfg(test)]
pub(super) unsafe fn frame(dc: HDC, origin: POINT, rect: Rect, dpi: u32) {
    // 沿主干纯白外描边，统一 2 个逻辑像素；不能覆盖图片内容或改变端点锚点。
    let width = desktop::pin_border_style::width(dpi) as i32;
    let (left, top, right, bottom) = (
        rect.left.saturating_sub(origin.x),
        rect.top.saturating_sub(origin.y),
        rect.right.saturating_sub(origin.x),
        rect.bottom.saturating_sub(origin.y),
    );
    let (outer_left, outer_top, outer_right, outer_bottom) = (
        left.saturating_sub(width),
        top.saturating_sub(width),
        right.saturating_add(width),
        bottom.saturating_add(width),
    );
    SetDCBrushColor(dc, COLORREF(0xffffff));
    let brush = HBRUSH(GetStockObject(DC_BRUSH).0);
    let fill = |left, top, right, bottom| {
        FillRect(
            dc,
            &RECT {
                left,
                top,
                right,
                bottom,
            },
            brush,
        );
    };
    // 四条色带固定开销，不创建笔刷；内矩形保持原样，DPI 不增加 GDI 调用次数。
    fill(outer_left, outer_top, outer_right, top);
    fill(outer_left, bottom, outer_right, outer_bottom);
    fill(outer_left, top, left, bottom);
    fill(right, top, outer_right, bottom);
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
    if ui.capturing() {
        ui.redraw_after_capture.set(true);
        return; // 截图期间保留已提交画面，只禁止输入；结束后由应用补一帧。
    }
    let result = (|| -> Result<()> {
        if hwnd == ui.drawing.get() {
            return window::refresh_drawing(ui);
        }
        let Some(buffer) = buffered::Frame::begin(dc, &paint.rcPaint)? else {
            return Ok(());
        };
        let mut bounds = RECT::default();
        GetWindowRect(hwnd, &mut bounds)?;
        contents(
            hwnd,
            ui,
            buffer.dc,
            &paint.rcPaint,
            POINT {
                x: bounds.left,
                y: bounds.top,
            },
        )?;
        buffer.commit()
    })();
    if let Err(error) = result {
        ui.fail(error);
    }
}
pub(super) unsafe fn present(ui: &Ui, rect: Rect) -> Result<()> {
    if ui.presenting.replace(true) {
        return Ok(()); // ULW 同步尺寸消息不能再修改尚在提交的 DIB。
    }
    struct Cycle<'a>(&'a Cell<bool>);
    impl Drop for Cycle<'_> {
        fn drop(&mut self) {
            self.0.set(false);
        }
    }
    let _cycle = Cycle(&ui.presenting);
    let size = SIZE {
        cx: rect.right - rect.left,
        cy: rect.bottom - rect.top,
    };
    let cached = ui.surface.borrow_mut().take().filter(|s| s.fits(size));
    let surface = match cached {
        Some(surface) => surface,
        None => Surface::new(size)?,
    };
    let area = RECT {
        left: 0,
        top: 0,
        right: size.cx,
        bottom: size.cy,
    };
    contents(
        ui.drawing.get(),
        ui,
        surface.dc(),
        &area,
        POINT {
            x: rect.left,
            y: rect.top,
        },
    )?;
    let result = surface.present(ui.drawing.get(), rect);
    if !ui.closed.get() {
        *ui.surface.borrow_mut() = Some(surface);
    }
    result
}
unsafe fn contents(hwnd: HWND, ui: &Ui, dc: HDC, area: &RECT, origin: POINT) -> Result<()> {
    SetDCBrushColor(dc, COLORREF(0));
    FillRect(dc, area, HBRUSH(GetStockObject(DC_BRUSH).0));
    if ui.blocked() {
        return Ok(());
    }
    SetBkMode(dc, TRANSPARENT);
    if hwnd == ui.drawing.get() {
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
    Ok(())
}
struct Paint(HWND, PAINTSTRUCT);
impl Drop for Paint {
    fn drop(&mut self) {
        unsafe {
            let _ = EndPaint(self.0, &self.1);
        }
    }
}
