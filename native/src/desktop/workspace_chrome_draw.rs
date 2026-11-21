//! 只画公共端口名/外框/Bezier 连线，不读取或拷贝图片；GDI 对象逐次 RAII 回收。
use super::*;
use crate::desktop::buffered_paint as buffered;
pub(super) use buffered::Session as PaintSession;
use windows::Win32::Graphics::Gdi::*;
#[path = "workspace_chrome_backdrops.rs"]
pub(super) mod backdrops;
#[path = "workspace_chrome_surface.rs"]
mod surface;
pub(super) use surface::Surface;
const BLUE: COLORREF = COLORREF(0xd4b606);
const YELLOW: COLORREF = COLORREF(0x38ffd9);
const TEXT: COLORREF = COLORREF(0xeff8f7);
type Lines = Vec<(Point, Point, COLORREF, bool)>;
#[derive(PartialEq, Eq)]
pub(super) struct Content {
    rect: Rect,
    lines: Lines,
    backdrops: backdrops::Backdrops,
}
#[cfg(test)]
#[path = "workspace_chrome_frame_tests.rs"]
mod frame_tests;
pub(super) fn ports(ui: &Ui) -> Vec<(Port, Point, i32)> {
    let mut ports = Vec::new();
    for node in ui
        .nodes
        .borrow()
        .iter()
        .filter(|node| node.ports(ui.mode.get(), node.hide_collapsed_endpoints))
    {
        for (side, count) in [(Side::Input, node.inputs), (Side::Output, node.outputs)] {
            for index in 0..count {
                let port = Port {
                    node: node.endpoint,
                    side,
                    index,
                };
                if let Some(point) = ui.point(port) {
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
pub(super) fn port_rect(ui: &Ui, port: Port, _: Point, _: i32) -> RECT {
    let frame = port_visual::layout(ui, port)
        .expect("rendered port has current geometry")
        .frame;
    let mut rect = RECT {
        left: frame.x,
        top: frame.y,
        right: frame.x.saturating_add(frame.width),
        bottom: frame.y.saturating_add(frame.height),
    };
    if let Some(clip) = parameter_rail::clip(ui, port) {
        rect.top = rect.top.max(clip.top);
        rect.bottom = rect.bottom.min(clip.bottom);
    }
    rect
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
        if backing::owns(ui, hwnd) {
            return Ok(()); // ULW 已提交完整底板，不能混用 GDI 重定向绘制。
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
pub(super) unsafe fn present(ui: &Ui, rect: Rect, backdrops: backdrops::Backdrops) -> Result<()> {
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
    let content = Content {
        rect,
        lines: lines(ui),
        backdrops,
    };
    let cached = ui.surface.borrow_mut().take().filter(|s| s.fits(size));
    // 节点位移不等于连线像素变化。保留已提交表面，避免每个拖动步都清空/ULW 整张公共层。
    if cached
        .as_ref()
        .is_some_and(|s| s.content.as_ref() == Some(&content))
    {
        *ui.surface.borrow_mut() = cached;
        return Ok(());
    }
    let mut surface = match cached {
        Some(surface) => surface,
        None => Surface::new(size)?,
    };
    let area = RECT {
        left: 0,
        top: 0,
        right: size.cx,
        bottom: size.cy,
    };
    clear(surface.dc(), &area);
    let origin = POINT {
        x: rect.left,
        y: rect.top,
    };
    content.backdrops.paint(ui, surface.dc(), origin)?;
    for &(a, b, color, dashed) in &content.lines {
        line(surface.dc(), origin, a, b, color, dashed);
    }
    let result = surface.present(ui.drawing.get(), rect);
    if result.is_ok() {
        surface.content = Some(content);
    }
    if !ui.closed.get() {
        *ui.surface.borrow_mut() = Some(surface);
    }
    result
}
unsafe fn clear(dc: HDC, area: &RECT) {
    SetDCBrushColor(dc, COLORREF(0));
    FillRect(dc, area, HBRUSH(GetStockObject(DC_BRUSH).0));
    SetBkMode(dc, TRANSPARENT);
}
unsafe fn contents(hwnd: HWND, ui: &Ui, dc: HDC, area: &RECT, origin: POINT) -> Result<()> {
    clear(dc, area);
    if ui.blocked() {
        return Ok(());
    }
    port_visual::paint(hwnd, ui, dc, origin)
}
fn lines(ui: &Ui) -> Lines {
    let mut lines = Vec::new();
    if !ui.blocked() {
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
                    lines.push((a, b, BLUE, false));
                }
                if dashed {
                    lines.push((a, b, TEXT, true));
                }
            }
        }
        if let Some(press) = ui.press.borrow().as_ref().filter(|press| press.dragged) {
            if let Some(start) = press.source.and_then(|source| ui.point(source)) {
                lines.push((start, press.current, YELLOW, true));
            }
        }
        if let (Some(source), Some(target)) = (ui.keyboard.source(), ui.keyboard.focused()) {
            if let (Some(a), Some(b)) = (ui.point(source), ui.point(target)) {
                lines.push((a, b, YELLOW, true));
            }
        }
    }
    lines
}
struct Paint(HWND, PAINTSTRUCT);
impl Drop for Paint {
    fn drop(&mut self) {
        unsafe {
            let _ = EndPaint(self.0, &self.1);
        }
    }
}
