//! 取景框四向命令与 UI 参考坐标；候选坐标只在 worker 接受后提交。
use super::*;
pub(super) struct State {
    region: Cell<Region>,
    pending: Cell<Option<Region>>,
}
impl State {
    pub fn region(&self) -> Region {
        self.region.get()
    }
    pub fn new(region: Region) -> Self {
        Self {
            region: Cell::new(region),
            pending: Cell::new(None),
        }
    }
    pub fn completed(&self, completion: &Completion) -> bool {
        let Some(candidate) = self.pending.take() else {
            return false;
        };
        let accepted = matches!(
            completion,
            Completion::Append(Ok(Append::Added { .. } | Append::Duplicate))
                | Completion::SpatialAppend(Ok(crate::long_capture::SpatialAppend::Added { .. }
                    | crate::long_capture::SpatialAppend::Duplicate))
        );
        if accepted {
            self.region.set(candidate);
        }
        accepted
    }
}
pub(super) unsafe fn controls(hwnd: HWND, input: &Input, busy: bool) {
    let allowed = !input.finish.pending()
        && !busy
        && !input.automatic.borrow().running()
        && !input.recording.running()
        && (!input.recording.enabled() || input.job.borrow().snapshot().axis.is_some())
        && input.job.borrow().can_move();
    for id in 108..=111 {
        if let Ok(control) = GetDlgItem(Some(hwnd), id) {
            let _ = EnableWindow(control, allowed);
        }
    }
}
pub(super) unsafe fn dispatch(hwnd: HWND, input: &Input, command: usize) {
    let desktop = Region {
        left: GetSystemMetrics(SM_XVIRTUALSCREEN),
        top: GetSystemMetrics(SM_YVIRTUALSCREEN),
        width: GetSystemMetrics(SM_CXVIRTUALSCREEN),
        height: GetSystemMetrics(SM_CYVIRTUALSCREEN),
    };
    dispatch_with(hwnd, input, command, desktop);
}
unsafe fn dispatch_with(hwnd: HWND, input: &Input, command: usize, desktop: Region) {
    let allowed = {
        let job = input.job.borrow();
        !input.finish.pending()
            && job.can_move()
            && !job.is_pending()
            && !input.automatic.borrow().running()
            && !input.recording.running()
            && (!input.recording.enabled() || job.snapshot().axis.is_some())
    };
    if !allowed {
        return;
    }
    let region = input.movement.region.get();
    let Some((candidate, dx, dy)) = candidate(region, command, desktop) else {
        return;
    };
    let dispatched = input.job.borrow_mut().move_capture(dx, dy);
    if dispatched.is_ok() {
        input.movement.pending.set(Some(candidate));
        waiting(hwnd, input);
    }
}
fn candidate(region: Region, command: usize, desktop: Region) -> Option<(Region, i32, i32)> {
    let (horizontal, sign) = match command {
        108 => (false, -1),
        109 => (true, -1),
        110 => (true, 1),
        111 => (false, 1),
        _ => return None,
    };
    let (length, position, minimum, extent) = if horizontal {
        (region.width, region.left, desktop.left, desktop.width)
    } else {
        (region.height, region.top, desktop.top, desktop.height)
    };
    if length <= 1 || extent < length {
        return None;
    }
    let maximum = i64::from(minimum) + i64::from(extent) - i64::from(length);
    if i64::from(position) < i64::from(minimum) || i64::from(position) > maximum {
        return None;
    }
    let step = (length / 5).clamp(1, 64);
    let destination =
        (i64::from(position) + i64::from(sign * step)).clamp(i64::from(minimum), maximum);
    let delta = i32::try_from(destination - i64::from(position)).ok()?;
    if delta == 0 {
        return None;
    }
    let destination = i32::try_from(destination).ok()?;
    let candidate = if horizontal {
        Region {
            left: destination,
            ..region
        }
    } else {
        Region {
            top: destination,
            ..region
        }
    };
    Some((
        candidate,
        if horizontal { delta } else { 0 },
        if horizontal { 0 } else { delta },
    ))
}
#[cfg(test)]
#[path = "long_capture_dialog_movement_tests.rs"]
mod tests;
