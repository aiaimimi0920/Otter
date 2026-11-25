//! Four-way viewfinder commands and UI reference coordinates; commit candidate coordinates only after worker acceptance.
use super::*;
use std::collections::VecDeque;
const QUEUED_MOVES: usize = 16;
pub(super) struct State {
    region: Cell<Region>,
    pending: Cell<Option<Region>>,
    queued: RefCell<VecDeque<(usize, Region)>>,
}
impl State {
    pub fn region(&self) -> Region {
        self.region.get()
    }
    pub fn new(region: Region) -> Self {
        Self {
            region: Cell::new(region),
            pending: Cell::new(None),
            queued: RefCell::new(VecDeque::new()),
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
        } else {
            self.queued.borrow_mut().clear();
        }
        accepted
    }
    pub fn queued(&self) -> bool {
        !self.queued.borrow().is_empty()
    }
    pub fn cancel(&self) {
        self.queued.borrow_mut().clear();
    }
}
pub(super) unsafe fn controls(hwnd: HWND, input: &Input, busy: bool) {
    let allowed = !input.finish.pending()
        && (!busy || input.recording.running())
        && input.movement.queued.borrow().len() < QUEUED_MOVES
        && !input.automatic.borrow().running()
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
            && (!job.is_pending() || input.recording.running())
            && !input.automatic.borrow().running()
    };
    if !allowed {
        return;
    }
    if input.job.borrow().is_pending() || input.movement.queued() {
        let mut queued = input.movement.queued.borrow_mut();
        if (108..=111).contains(&command) && queued.len() < QUEUED_MOVES {
            queued.push_back((command, desktop));
        }
        drop(queued);
        controls(hwnd, input, true);
        return;
    }
    begin(hwnd, input, command, desktop);
}
// Periodic capture and previews share one worker; retain clicks first and prioritize user movement when idle.
pub(super) unsafe fn drive(hwnd: HWND, input: &Input) {
    if !input.recording.running() || input.finish.pending() {
        if input.movement.queued() {
            input.movement.cancel();
            controls(hwnd, input, input.job.borrow().is_pending());
        }
        return;
    }
    while !input.job.borrow().is_pending() {
        let request = input.movement.queued.borrow_mut().pop_front();
        let Some((command, desktop)) = request else {
            break;
        };
        begin(hwnd, input, command, desktop);
    }
    if !input.job.borrow().is_pending() {
        controls(hwnd, input, false);
    }
}
unsafe fn begin(hwnd: HWND, input: &Input, command: usize, desktop: Region) {
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
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "long_capture_dialog_movement_tests.rs"]
mod tests;
