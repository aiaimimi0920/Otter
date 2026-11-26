//! Tab left/right strips and bottom bars remain adjacent to their pins; shared curves stay below all pins without changing image order/focus.
use super::*;

struct Guard<'a>(&'a Cell<bool>);
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
const FLAGS: SET_WINDOW_POS_FLAGS = SET_WINDOW_POS_FLAGS(
    SWP_NOMOVE.0 | SWP_NOSIZE.0 | SWP_NOACTIVATE.0 | SWP_NOREDRAW.0 | SWP_NOOWNERZORDER.0,
);
unsafe fn topmost(hwnd: HWND) -> bool {
    GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0
}

pub(in super::super) unsafe fn apply(ui: &Ui) -> Result<()> {
    if ui.drawing.get().is_invalid() || ui.blocked() || ui.ordering.replace(true) {
        return Ok(());
    }
    // Win32 synchronous callbacks must not nest another Z-order transaction; no RefCell borrow across system calls.
    let _guard = Guard(&ui.ordering);
    let nodes: Vec<_> = ui
        .nodes
        .borrow()
        .iter()
        .filter(|node| {
            let host = HWND(node.hwnd as *mut _);
            IsWindowVisible(host).as_bool() && GetWindowLongPtrW(host, GWLP_USERDATA) == node.marker
        })
        .cloned()
        .collect();
    let strips = ui.input_windows.borrow().clone();
    let backings = ui.backings.borrow().clone();
    let mut tails = HashSet::new();
    for node in &nodes {
        let host = HWND(node.hwnd as *mut _);
        let layers: Vec<_> = strips
            .iter()
            .filter(|(hwnd, endpoint, _)| {
                *endpoint == node.endpoint && IsWindow(Some(*hwnd)).as_bool()
            })
            .map(|(hwnd, _, _)| *hwnd)
            .chain(drawer::moving(ui, host))
            .chain(backings.iter().filter(|b| b.matches(node)).map(|b| b.hwnd))
            .collect();
        adjacent(host, &layers)?;
        tails.insert(layers.last().copied().unwrap_or(host).0 as isize);
    }
    if tails.is_empty() {
        return Ok(());
    }
    let drawing = ui.drawing.get();
    // Performance audit optimization (Tier 2.2):
    // Skip the up-to-4,096-HWND desktop `GetTopWindow` Z-order scan when the shared link/curve layer is hidden
    // (ordinary pin workspaces without links/ports) or already positioned immediately behind the sole pin tail.
    if !IsWindowVisible(drawing).as_bool() {
        return Ok(());
    }
    if tails.len() == 1 {
        let only = *tails.iter().next().unwrap();
        if GetWindow(HWND(only as *mut _), GW_HWNDNEXT).ok() == Some(drawing) {
            return Ok(());
        }
    }
    // Only shared curves retain the below-all-pins policy; bottom bars belong to their own pins, not the shared line layer.
    let layers = [drawing];
    let members: HashSet<_> = layers.iter().map(|hwnd| hwnd.0 as isize).collect();
    let mut cursor = GetTopWindow(None)?;
    let mut above = false;
    for _ in 0..4096 {
        if cursor.is_invalid() {
            break;
        }
        above |= members.contains(&(cursor.0 as isize));
        if tails.remove(&(cursor.0 as isize)) && tails.is_empty() {
            // Correct only shared layers crossing nodes/Tab; correctly lowered line layers must not rise above external applications on refresh.
            return if above {
                adjacent(cursor, &layers)
            } else {
                Ok(())
            };
        }
        cursor = GetWindow(cursor, GW_HWNDNEXT).unwrap_or_default();
    }
    Ok(())
}

unsafe fn adjacent(anchor: HWND, layers: &[HWND]) -> Result<()> {
    let band = topmost(anchor);
    let mut previous = anchor;
    let mut changed = false;
    for &hwnd in layers {
        changed |= GetWindow(previous, GW_HWNDNEXT).ok() != Some(hwnd) || topmost(hwnd) != band;
        previous = hwnd;
    }
    if !changed || layers.is_empty() {
        return Ok(());
    }
    // Insertion after the last TOPMOST window does not automatically become TOPMOST; match the band first.
    // The lack of native owners is deliberate: focus on edit children must not also raise images.
    for &hwnd in layers {
        if topmost(hwnd) != band {
            SetWindowPos(
                hwnd,
                Some(if band { HWND_TOPMOST } else { HWND_NOTOPMOST }),
                0,
                0,
                0,
                0,
                FLAGS,
            )?;
        }
    }
    let mut batch = BeginDeferWindowPos(layers.len() as i32)?;
    // Insert in reverse order against fixed anchors, preventing batches from reading prior members' old bands and raising later members again.
    for &hwnd in layers.iter().rev() {
        batch = DeferWindowPos(batch, hwnd, Some(anchor), 0, 0, 0, 0, FLAGS)?;
    }
    EndDeferWindowPos(batch)
}
