//! UI owner for user drag/scroll recording; original SessionJob retains exclusive pixels, periodic capture, and stitching.
use super::*;
use std::time::Instant;
#[path = "long_capture_recording_hook.rs"]
mod hook;
#[path = "long_capture_recording_schedule.rs"]
mod schedule;
use schedule::{Action, Phase, Schedule};

pub(super) struct State {
    enabled: bool,
    target: Option<long_capture_source::scroll::Target>,
    schedule: RefCell<Schedule>,
    hook: RefCell<Option<hook::Hook>>,
    preview_due: Cell<bool>,
    covered: Cell<bool>,
    pub(super) overlay_layout: Cell<Option<(i32, i32, u32)>>,
}
impl State {
    pub fn new(target: Option<long_capture_source::scroll::Target>) -> Self {
        Self {
            enabled: target.is_some(),
            target,
            schedule: RefCell::new(Schedule::default()),
            hook: RefCell::new(None),
            preview_due: Cell::new(true),
            covered: Cell::new(false),
            overlay_layout: Cell::new(None),
        }
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn running(&self) -> bool {
        self.schedule.borrow().phase() == Phase::Recording
    }
    pub fn cancel(&self) {
        self.covered.set(false);
        self.overlay_layout.set(None);
        self.hook.borrow_mut().take();
        self.schedule.borrow_mut().cancel();
        self.preview_due.set(false);
    }
    pub fn stop(&self) {
        self.covered.set(false);
        self.hook.borrow_mut().take();
        self.schedule.borrow_mut().stop();
    }
}
pub(super) unsafe fn start(hwnd: HWND, input: &Input) {
    let state = &input.recording;
    let Some(target) = &state.target else {
        return;
    };
    if input.finish.pending() || input.job.borrow().is_pending() || state.running() {
        return;
    }
    let result = target
        .validate()
        .map_err(|error| Error::new(E_FAIL, error))
        .and_then(|_| hook::Hook::install(hwnd, target.root()));
    match result {
        Ok(hook) if state.schedule.borrow_mut().start(Instant::now()) => {
            *state.hook.borrow_mut() = Some(hook);
            status(hwnd, crate::ui_text::text("long-capture-recording"));
        }
        Ok(_) => status(hwnd, crate::ui_text::text("long-capture-record-time-limit")),
        Err(error) => status(
            hwnd,
            &crate::ui_text::format(
                "long-capture-record-start-failed",
                &[("error", error.to_string().into())],
            ),
        ),
    }
    controls(hwnd, input, false);
    drive(hwnd, input);
}
pub(super) unsafe fn command(hwnd: HWND, input: &Input) {
    if !input.recording.enabled() || input.finish.pending() {
        return;
    }
    if input.recording.running() {
        input.recording.stop();
        let busy = input.job.borrow().is_pending();
        controls(hwnd, input, busy);
        status(hwnd, crate::ui_text::text("long-capture-record-stopped"));
        drive(hwnd, input);
    } else {
        start(hwnd, input);
    }
}
pub(super) unsafe fn notify(hwnd: HWND, input: &Input, wp: WPARAM) {
    let wheel = input
        .recording
        .hook
        .borrow()
        .as_ref()
        .and_then(|hook| hook.take(wp));
    if let Some(wheel) = wheel {
        receive(hwnd, input, wheel);
    }
    drive(hwnd, input);
}
unsafe fn receive(hwnd: HWND, input: &Input, wheel: hook::Wheel) {
    let state = &input.recording;
    if !state.running()
        || state.covered.get()
        || windows::Win32::System::SystemInformation::GetTickCount64().saturating_sub(wheel.time_ms)
            > 220
    {
        return;
    }
    let Some(target) = &state.target else {
        return;
    };
    match target.accepts_point(wheel.point) {
        Ok(true) => state.schedule.borrow_mut().notify(Instant::now()),
        Ok(false) => {}
        Err(error) => {
            state.stop();
            status(
                hwnd,
                &crate::ui_text::format(
                    "long-capture-record-error",
                    &[("error", error.to_string().into())],
                ),
            );
            controls(hwnd, input, input.job.borrow().is_pending());
        }
    }
}
pub(super) unsafe fn finish(hwnd: HWND, input: &Input) {
    let pending = input
        .recording
        .hook
        .borrow()
        .as_ref()
        .and_then(|hook| hook.pending());
    if let Some(wheel) = pending {
        receive(hwnd, input, wheel);
    }
    input.recording.hook.borrow_mut().take();
    input.recording.preview_due.set(false);
    if input.recording.schedule.borrow().phase() == Phase::Finished {
        super::finish::compose(hwnd, input);
        return;
    }
    input.recording.schedule.borrow_mut().finish(Instant::now());
    status(hwnd, crate::ui_text::text("long-capture-last-frame"));
    controls(hwnd, input, input.job.borrow().is_pending());
    drive(hwnd, input);
}
/// Return whether the occlusion-pause branch already projected this failure into UI.
pub(super) unsafe fn completed(hwnd: HWND, input: &Input, completion: &Completion) -> bool {
    let state = &input.recording;
    if !state.enabled() {
        return false;
    }
    match completion {
        Completion::CaptureFailed(error)
            if error == long_capture_source::scroll::TARGET_COVERED
                && matches!(
                    state.schedule.borrow().phase(),
                    Phase::Recording | Phase::Draining
                ) =>
        {
            state.schedule.borrow_mut().covered(Instant::now());
            if state.running() && !state.covered.replace(true) {
                status(hwnd, crate::ui_text::text("long-capture-record-obscured"));
            }
            return true;
        }
        Completion::Append(Ok(_)) | Completion::SpatialAppend(Ok(_)) => {
            if state.covered.replace(false) && state.running() {
                status(hwnd, crate::ui_text::text("long-capture-recording"));
            }
        }
        _ => {}
    }
    match completion {
        Completion::Append(Ok(Append::Added { .. }))
        | Completion::SpatialAppend(Ok(crate::long_capture::SpatialAppend::Added { .. })) => {
            state.preview_due.set(true);
            state.schedule.borrow_mut().completed(Instant::now());
        }
        Completion::Append(Ok(Append::Duplicate))
        | Completion::SpatialAppend(Ok(crate::long_capture::SpatialAppend::Duplicate)) => {
            state.schedule.borrow_mut().completed(Instant::now());
        }
        Completion::Append(Ok(Append::Rejected(_)))
        | Completion::SpatialAppend(Ok(crate::long_capture::SpatialAppend::Rejected(_))) => {
            // Dragging may briefly repaint or lack reliable overlap; skip the frame and continue sampling from the last trusted baseline.
            state.schedule.borrow_mut().completed(Instant::now());
        }
        Completion::Preview(Err(crate::long_capture::Error::UnconfirmedEdges)) => {}
        Completion::Compose(Ok(_)) | Completion::SpatialCompose(Ok(_)) => state.cancel(),
        Completion::Preview(Ok(_)) | Completion::SpatialPreview(Ok(_)) => {}
        _ => state.stop(),
    }
    false
}
pub(super) unsafe fn drive(hwnd: HWND, input: &Input) {
    drive_with_timer(hwnd, input, |delay| {
        SetTimer(Some(hwnd), TIMER, delay, None) != 0
    });
}
unsafe fn drive_with_timer(hwnd: HWND, input: &Input, arm: impl FnOnce(u32) -> bool) {
    if !input.recording.enabled() {
        return;
    }
    movement::drive(hwnd, input);
    let busy = input.job.borrow().is_pending();
    let action = input
        .recording
        .schedule
        .borrow_mut()
        .action(Instant::now(), busy);
    match action {
        Action::Capture => {
            let result = input.job.borrow_mut().capture();
            if result.is_ok() {
                waiting(hwnd, input);
            } else {
                input.recording.stop();
                input.finish.retry();
                status(hwnd, crate::ui_text::text("long-capture-sample-failed"));
            }
        }
        Action::Compose => {
            super::finish::compose(hwnd, input);
        }
        Action::Limit => {
            input.recording.stop();
            input.finish.retry();
            status(hwnd, crate::ui_text::text("long-capture-record-time-limit"));
            controls(hwnd, input, false);
        }
        Action::Idle => {
            let can_preview = input.job.borrow().snapshot().confirmed_edges.is_some()
                || input.job.borrow().snapshot().frame_count == 1
                || input.job.borrow().snapshot().spatial_position.is_some();
            if !busy
                && !input.finish.pending()
                && input.recording.preview_due.get()
                && can_preview
                && !input.recording.schedule.borrow().needs_timer(false)
            {
                input.recording.preview_due.set(false);
                let result = input.job.borrow_mut().preview();
                if result.is_ok() {
                    waiting(hwnd, input);
                }
            }
        }
    }
    let busy = input.job.borrow().is_pending();
    let delay = input
        .recording
        .schedule
        .borrow()
        .wake_after(Instant::now(), busy);
    if let Some(delay) = delay {
        if !arm(delay.as_millis().clamp(1, 300_000) as u32) {
            // The timer is also the sole background-reply wakeup; merely stopping recording leaves in-flight tasks and disabled controls.
            *input.error.borrow_mut() =
                Some(crate::ui_text::text("long-capture-wait-failed").into());
            close(hwnd, input);
        }
    } else {
        let _ = KillTimer(Some(hwnd), TIMER);
    }
}
pub(super) unsafe fn update_control(hwnd: HWND, input: &Input, busy: bool) {
    let state = &input.recording;
    if let Ok(control) = GetDlgItem(Some(hwnd), 112) {
        let _ = ShowWindow(control, if state.enabled() { SW_SHOW } else { SW_HIDE });
        let _ = EnableWindow(
            control,
            state.enabled() && !input.finish.pending() && (state.running() || !busy),
        );
    }
    let _ = SetDlgItemTextW(
        hwnd,
        112,
        &HSTRING::from(if state.running() {
            crate::ui_text::text("long-capture-record-stop")
        } else {
            crate::ui_text::text("long-capture-record-resume")
        }),
    );
}
pub(super) const MESSAGE: u32 = hook::MESSAGE;

#[cfg(test)]
#[path = "long_capture_recording_runtime_tests.rs"]
mod runtime_tests;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "long_capture_recording_tests.rs"]
mod tests;
