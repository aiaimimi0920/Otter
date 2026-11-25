//! Move cached image/border frames, owned ports, and edit strips together; no shared-link drawing, image copies, or unrelated-pin relayout.
use super::*;

pub(super) unsafe fn cached(
    ui: Option<&Ui>,
    host: HWND,
    frame: HWND,
    x: i32,
    y: i32,
    attached: &[(HWND, RECT)],
) -> Result<()> {
    let mut current = RECT::default();
    GetWindowRect(frame, &mut current)?;
    let (dx, dy) = (x - current.left, y - current.top);
    if dx == 0 && dy == 0 && attached.is_empty() {
        return Ok(());
    }
    let node = ui.and_then(|ui| {
        ui.nodes
            .borrow()
            .iter()
            .find(|n| n.hwnd == host.0 as isize)
            .cloned()
    });
    let handles: Vec<_> = node.map_or_else(Vec::new, |node| {
        ui.unwrap()
            .input_windows
            .borrow()
            .iter()
            .filter(|(_, endpoint, _)| *endpoint == node.endpoint)
            .map(|(hwnd, _, _)| *hwnd)
            .collect()
    });
    let mut positions = vec![(frame, x, y)];
    for hwnd in handles
        .into_iter()
        .chain(ui.map_or_else(Vec::new, |ui| backing::moving(ui, host)))
        .chain(ui.map_or_else(Vec::new, |ui| drawer::moving(ui, host)))
    {
        if IsWindowVisible(hwnd).as_bool() {
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect)?;
            positions.push((
                hwnd,
                rect.left.saturating_add(dx),
                rect.top.saturating_add(dy),
            ));
        }
    }
    positions.extend(
        attached
            .iter()
            .map(|(hwnd, rect)| (*hwnd, rect.left, rect.top)),
    );
    let mut batch = BeginDeferWindowPos(positions.len() as i32)?;
    for (hwnd, x, y) in positions {
        batch = DeferWindowPos(
            batch,
            hwnd,
            None,
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_NOREDRAW,
        )?;
    }
    EndDeferWindowPos(batch)
}
