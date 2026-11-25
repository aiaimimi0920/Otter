//! Finish buttons/global shortcuts share stop, drain, and stitch state; failures return retryable state without silently delivering partial results.
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
        status(hwnd, crate::ui_text::text("long-capture-finish-wait"));
    } else {
        compose(hwnd, input);
    }
}
pub(super) unsafe fn compose(hwnd: HWND, input: &Input) {
    input.finish.phase.set(Phase::Composing);
    let result = input.job.borrow_mut().compose();
    if result.is_ok() {
        waiting(hwnd, input);
        status(hwnd, crate::ui_text::text("long-capture-stitching"));
    } else {
        input.finish.retry();
        controls(hwnd, input, false);
        status(hwnd, crate::ui_text::text("long-capture-stitch-failed"));
    }
}
pub(super) fn completed(input: &Input, completion: &Completion, covered: bool) -> bool {
    match input.finish.phase.get() {
        Phase::Draining => {
            let accepted = covered
                || matches!(
                    completion,
                    Completion::Append(Ok(Append::Added { .. } | Append::Duplicate))
                        | Completion::SpatialAppend(Ok(
                            crate::long_capture::SpatialAppend::Added { .. }
                                | crate::long_capture::SpatialAppend::Duplicate
                        ))
                        | Completion::Preview(Ok(_))
                        | Completion::SpatialPreview(Ok(_))
                )
                || (input.recording.enabled()
                    && matches!(
                        completion,
                        Completion::Append(Ok(Append::Rejected(_)))
                            | Completion::SpatialAppend(Ok(
                                crate::long_capture::SpatialAppend::Rejected(_)
                            ))
                    ));
            // Skip uncommitted occluded/transiently rejected frames; other actual capture/stitch errors still require retries.
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
            // Success retires routing at output-commit boundaries; failure retains sessions and requires explicit retry.
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
