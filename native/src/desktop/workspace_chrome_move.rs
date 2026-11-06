//! 缓存图框和所属端口在同一 DeferWindowPos 批次移动，批次之间不绘共享连线或复制图片。
use super::*;

pub(super) unsafe fn cached(ui: &Ui, host: HWND, frame: HWND, x: i32, y: i32) -> Result<()> {
    let mut current = RECT::default();
    GetWindowRect(frame, &mut current)?;
    let (dx, dy) = (x - current.left, y - current.top);
    if dx == 0 && dy == 0 {
        return Ok(());
    }
    let node = ui
        .nodes
        .borrow()
        .iter()
        .find(|n| n.hwnd == host.0 as isize)
        .cloned();
    let handles: Vec<_> = node.map_or_else(Vec::new, |node| {
        ui.input_windows
            .borrow()
            .iter()
            .filter(|(_, endpoint, _)| *endpoint == node.endpoint)
            .map(|(hwnd, _, _)| *hwnd)
            .collect()
    });
    let mut positions = vec![(frame, x, y)];
    for hwnd in handles {
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
