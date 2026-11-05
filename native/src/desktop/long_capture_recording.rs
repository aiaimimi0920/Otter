//! 用户滚轮录制的专属 UI owner；像素、采集与拼接仍由原 SessionJob 独占。
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
    rejected: Cell<u8>,
    revoked: Cell<bool>,
}
impl State {
    pub fn new(target: Option<long_capture_source::scroll::Target>) -> Self {
        Self {
            enabled: target.is_some(),
            target,
            schedule: RefCell::new(Schedule::default()),
            hook: RefCell::new(None),
            preview_due: Cell::new(false),
            rejected: Cell::new(0),
            revoked: Cell::new(false),
        }
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn running(&self) -> bool {
        self.schedule.borrow().phase() == Phase::Recording
    }
    pub fn cancel(&self) {
        self.hook.borrow_mut().take();
        self.schedule.borrow_mut().cancel();
        self.preview_due.set(false);
    }
    pub fn stop(&self) {
        self.hook.borrow_mut().take();
        self.schedule.borrow_mut().stop();
    }
    pub fn revoke(&self) {
        self.stop();
        self.revoked.set(true);
    }
}
pub(super) unsafe fn start(hwnd: HWND, input: &Input) {
    let state = &input.recording;
    let Some(target) = &state.target else {
        return;
    };
    if state.revoked.get()
        || input.finish.pending()
        || input.job.borrow().is_pending()
        || state.running()
    {
        return;
    }
    let result = target
        .validate()
        .map_err(|error| Error::new(E_FAIL, error))
        .and_then(|_| hook::Hook::install(hwnd, target.root()));
    match result {
        Ok(hook) if state.schedule.borrow_mut().start(Instant::now()) => {
            *state.hook.borrow_mut() = Some(hook);
            state.rejected.set(0);
            status(hwnd, "录制中 · 请滚动目标页面 · 方向待识别");
        }
        Ok(_) => status(hwnd, "录制已达到次数或时间上限，可完成或取消"),
        Err(error) => status(hwnd, &format!("无法开始录制：{error}")),
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
        status(
            hwnd,
            if busy {
                "录制已停止，等待当前帧"
            } else {
                "录制已停止"
            },
        );
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
            status(hwnd, &format!("录制已停止：{error}"));
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
    status(hwnd, "正在收取最近滚动后完成…");
    controls(hwnd, input, input.job.borrow().is_pending());
    drive(hwnd, input);
}
pub(super) fn completed(input: &Input, completion: &Completion) {
    let state = &input.recording;
    if !state.enabled() {
        return;
    }
    match completion {
        Completion::Append(Ok(Append::Added { .. }))
        | Completion::SpatialAppend(Ok(crate::long_capture::SpatialAppend::Added { .. })) => {
            state.rejected.set(0);
            state.preview_due.set(true);
            state.schedule.borrow_mut().completed(Instant::now());
        }
        Completion::Append(Ok(Append::Duplicate))
        | Completion::SpatialAppend(Ok(crate::long_capture::SpatialAppend::Duplicate)) => {
            state.rejected.set(0);
            state.schedule.borrow_mut().completed(Instant::now());
        }
        Completion::Append(Ok(Append::Rejected(_)))
        | Completion::SpatialAppend(Ok(crate::long_capture::SpatialAppend::Rejected(_))) => {
            state.rejected.set(state.rejected.get().saturating_add(1));
            if state.rejected.get() >= 3 {
                state.stop();
            } else {
                state.schedule.borrow_mut().completed(Instant::now());
            }
        }
        Completion::Preview(Err(crate::long_capture::Error::UnconfirmedEdges)) => {}
        Completion::Compose(Ok(_)) | Completion::SpatialCompose(Ok(_)) => state.cancel(),
        Completion::Preview(Ok(_)) | Completion::SpatialPreview(Ok(_)) => {}
        _ => state.stop(),
    }
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
                status(hwnd, "无法安排录制采样，已停止");
            }
        }
        Action::Compose => {
            super::finish::compose(hwnd, input);
        }
        Action::Limit => {
            input.recording.stop();
            input.finish.retry();
            status(hwnd, "录制已达到次数或时间上限，可完成或取消");
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
            // timer 也是唯一后台回复唤醒；仅停止录制会遗留在途任务和禁用控件。
            *input.error.borrow_mut() = Some("无法安排长截图录制等待，录制已安全关闭。".into());
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
            state.enabled()
                && !state.revoked.get()
                && !input.finish.pending()
                && (state.running() || !busy),
        );
    }
    let _ = SetDlgItemTextW(
        hwnd,
        112,
        &HSTRING::from(if state.running() {
            "停止录制"
        } else {
            "继续录制"
        }),
    );
}
pub(super) const MESSAGE: u32 = hook::MESSAGE;

#[cfg(test)]
#[path = "long_capture_recording_runtime_tests.rs"]
mod runtime_tests;
#[cfg(test)]
#[path = "long_capture_recording_tests.rs"]
mod tests;
