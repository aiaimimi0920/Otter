//! 完成按钮和全局快捷键共用停止、排空、拼接状态；失败返回可重试状态，不静默交付部分结果。
use super::super::finish_route::Registration;
use super::*;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Ready,
    Draining,
    Composing,
    Closed,
}
pub(super) struct State {
    capture: Rc<Cell<bool>>,
    route: RefCell<Option<Registration>>,
    phase: Cell<Phase>,
}
impl State {
    pub fn new(capture: Rc<Cell<bool>>) -> Self {
        Self {
            capture,
            route: RefCell::new(None),
            phase: Cell::new(Phase::Ready),
        }
    }
    pub fn pending(&self) -> bool {
        self.phase.get() != Phase::Ready
    }
    pub fn retry(&self) {
        self.phase.set(Phase::Ready);
    }
    pub unsafe fn install(&self, hwnd: HWND) -> Result<()> {
        *self.route.borrow_mut() = Some(Registration::new(hwnd, self.capture.clone())?);
        Ok(())
    }
    pub unsafe fn accepts(&self, hwnd: HWND, wp: WPARAM, lp: LPARAM) -> bool {
        self.route
            .borrow()
            .as_ref()
            .is_some_and(|route| route.accept(hwnd, wp, lp))
    }
    pub fn close(&self) {
        self.phase.set(Phase::Closed);
        self.route.borrow_mut().take();
    }
}
pub(super) unsafe fn request(hwnd: HWND, input: &Input) {
    if input.finish.pending() {
        return;
    }
    input.finish.phase.set(Phase::Draining);
    input.automatic.borrow_mut().stop();
    if input.recording.enabled() {
        super::recording::finish(hwnd, input);
        return;
    }
    if input.job.borrow().is_pending() {
        waiting(hwnd, input);
        status(hwnd, "已停止后续滚动，等待当前任务后完成");
    } else {
        compose(hwnd, input);
    }
}
pub(super) unsafe fn compose(hwnd: HWND, input: &Input) {
    input.finish.phase.set(Phase::Composing);
    let result = input.job.borrow_mut().compose();
    if result.is_ok() {
        waiting(hwnd, input);
        status(hwnd, "正在拼接长截图…");
    } else {
        input.finish.retry();
        controls(hwnd, input, false);
        status(hwnd, "无法启动长截图拼接，请重试或取消");
    }
}
pub(super) fn completed(input: &Input, completion: &Completion) -> bool {
    match input.finish.phase.get() {
        Phase::Draining => {
            let accepted = matches!(
                completion,
                Completion::Append(Ok(Append::Added { .. } | Append::Duplicate))
                    | Completion::SpatialAppend(Ok(
                        crate::long_capture::SpatialAppend::Added { .. }
                            | crate::long_capture::SpatialAppend::Duplicate
                    ))
                    | Completion::Preview(Ok(_))
                    | Completion::SpatialPreview(Ok(_))
            );
            if !accepted {
                input.finish.retry();
                if input.recording.enabled() {
                    input.recording.stop();
                }
            }
            if input.recording.enabled() {
                return false;
            }
            accepted
        }
        Phase::Composing => {
            // 成功由输出提交边界退休路由；失败保留会话，必须显式重试。
            if matches!(
                completion,
                Completion::Compose(Err(_)) | Completion::SpatialCompose(Err(_))
            ) {
                input.finish.retry();
            }
            false
        }
        _ => false,
    }
}
