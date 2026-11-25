//! Draw only public port names/borders/Bezier links, without image reads/copies; GDI objects are reclaimed through RAII each time.
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
pub(super) fn ports(ui: &Ui) -> Vec<Port> {
    let mut ports = Vec::new();
    for node in ui
        .nodes
        .borrow()
        .iter()
        .filter(|node| node.ports(ui.mode.get(), node.hide_collapsed_endpoints))
    {
        for side in [Side::Input, Side::Output] {
            let geometry = port_visual::SideGeometry::new(ui, node, side);
            for index in geometry.indices() {
                if geometry.layout(index, true).is_some() {
                    ports.push(geometry.port(index));
                }
            }
        }
    }
    ports
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
    // Retain mainline pure-white outer strokes at two logical pixels; never cover image content or move endpoint anchors.
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
    // Four color bands have fixed cost without brush creation; inner rectangles stay unchanged, and DPI adds no GDI calls.
    fill(outer_left, outer_top, outer_right, top);
    fill(outer_left, bottom, outer_right, outer_bottom);
    fill(outer_left, top, left, bottom);
    fill(right, top, outer_right, bottom);
}
unsafe fn line(
    dc: HDC,
    origin: POINT,
    a: Point,
    b: Point,
    color: COLORREF,
    dashed: bool,
) -> Result<()> {
    let pen = CreatePen(
        if dashed { PS_DASH } else { PS_SOLID },
        if dashed { 1 } else { 2 },
        color,
    );
    if pen.is_invalid() {
        return Err(Error::new(E_FAIL, "Workspace chrome CreatePen failed"));
    }
    let _pen = Object(HGDIOBJ(pen.0));
    let old = SelectObject(dc, HGDIOBJ(pen.0));
    if old.is_invalid() {
        return Err(Error::new(E_FAIL, "Workspace chrome pen selection failed"));
    }
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
    let drawn = PolyBezier(dc, &points);
    let restored = SelectObject(dc, old);
    if !drawn.as_bool() || restored.is_invalid() {
        return Err(Error::new(E_FAIL, "Workspace chrome curve draw failed"));
    }
    Ok(())
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
        return; // Retain committed frames during screenshots and disable only input; the application supplies a frame afterward.
    }
    let result = (|| -> Result<()> {
        if hwnd == ui.drawing.get() {
            return window::refresh_drawing(ui);
        }
        if backing::owns(ui, hwnd) {
            return Ok(()); // ULW already committed the complete backdrop; do not mix GDI redirected drawing.
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
    present_with(ui, rect, backdrops, |dc, origin, (a, b, color, dashed)| {
        line(dc, origin, a, b, color, dashed)
    })
}
pub(super) unsafe fn present_with(
    ui: &Ui,
    rect: Rect,
    backdrops: backdrops::Backdrops,
    mut draw_line: impl FnMut(HDC, POINT, (Point, Point, COLORREF, bool)) -> Result<()>,
) -> Result<()> {
    if ui.presenting.replace(true) {
        return Ok(()); // Synchronous ULW size messages must not modify the DIB still being submitted.
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
    // Node movement does not imply changed link pixels. Retain committed surfaces instead of clearing/ULW-submitting the entire shared layer per drag step.
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
    // The reused DIB now becomes scratch. No old or new identity survives a failed frame.
    surface.content = None;
    clear(surface.dc(), &area)?;
    let origin = POINT {
        x: rect.left,
        y: rect.top,
    };
    content.backdrops.paint(ui, surface.dc(), origin)?;
    for &curve in &content.lines {
        draw_line(surface.dc(), origin, curve)?;
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
unsafe fn clear(dc: HDC, area: &RECT) -> Result<()> {
    if SetDCBrushColor(dc, COLORREF(0)) == COLORREF(u32::MAX)
        || FillRect(dc, area, HBRUSH(GetStockObject(DC_BRUSH).0)) == 0
        || SetBkMode(dc, TRANSPARENT) == 0
    {
        return Err(Error::new(
            E_FAIL,
            "Workspace chrome background draw failed",
        ));
    }
    Ok(())
}
unsafe fn contents(hwnd: HWND, ui: &Ui, dc: HDC, area: &RECT, origin: POINT) -> Result<()> {
    clear(dc, area)?;
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
                // point already determines visibility from clipped port layout; do not calculate port positions again.
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
