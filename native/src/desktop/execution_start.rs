//! User Start commands capture only configuration identity and wake the application; no image encoding/Loom connections inside Win32 callbacks.
use super::{configuration_stamp::Stamp, State};
#[cfg(test)]
use windows::core::w;
use windows::{
    core::Result,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
pub(super) const COMMAND: usize = 0x1083;
#[cfg(test)]
#[path = "execution_start_restore_tests.rs"]
mod restore_tests;
pub(super) struct Request {
    pub stamp: Stamp,
    pub trigger: crate::graph_execution_config::Trigger,
}
/// After parameter transactions, coalesce to latest configuration; cancellation queues retire old execution first, without workers starting in edit callbacks.
pub(super) fn parameter_changed(state: &State) {
    automatic_changed(
        state,
        crate::graph_execution_config::Trigger::Parameter { final_change: true },
    );
}
pub(super) fn upstream_changed(state: &State) {
    automatic_changed(state, crate::graph_execution_config::Trigger::Upstream);
}
fn automatic_changed(state: &State, automatic: crate::graph_execution_config::Trigger) {
    if state.group_locked.get() || state.selected_art.borrow().is_none() {
        return;
    }
    // Actual parameter/upstream changes release restored-preview retention even if automatic-execution settings later reject backend requests.
    state
        .execution_results
        .borrow_mut()
        .invalidate_restored_preview();
    let mut pending = state.pending_start.borrow_mut();
    let trigger = if pending
        .as_ref()
        .is_some_and(|request| request.trigger == crate::graph_execution_config::Trigger::Manual)
    {
        crate::graph_execution_config::Trigger::Manual
    } else {
        automatic
    };
    *pending = Some(Request {
        stamp: Stamp::capture(state),
        trigger,
    });
}
pub(super) fn available(state: &State) -> bool {
    ready(state) && state.pending_start.borrow().is_none()
}
/// Scheduler checks exclude the pending-start slot itself, preventing queued requests from permanently declaring themselves busy.
pub(super) fn ready(state: &State) -> bool {
    state.selected_art.borrow().is_some()
        && !state.group_locked.get()
        && state.pending_preparation.borrow().is_none()
        && state.pending_execution.borrow().is_none()
        && state.execution_results.borrow().active().is_none()
}
pub(super) unsafe fn request(hwnd: HWND, state: &State) -> Result<()> {
    if !IsWindow(Some(hwnd)).as_bool() {
        return Err(windows::core::Error::from_win32());
    }
    if !available(state) {
        return Ok(());
    }
    *state.pending_start.borrow_mut() = Some(Request {
        stamp: Stamp::capture(state),
        trigger: crate::graph_execution_config::Trigger::Manual,
    });
    if let Err(error) = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) {
        state.pending_start.borrow_mut().take();
        return Err(error);
    }
    state
        .execution_results
        .borrow_mut()
        .invalidate_restored_preview();
    Ok(())
}
/// Same explicit run after modeless draft commit: upgrade only the current parameter trigger, without dropping/overwriting other start requests.
pub(super) unsafe fn request_after_parameter(hwnd: HWND, state: &State) -> Result<()> {
    if !IsWindow(Some(hwnd)).as_bool() || !ready(state) {
        return Ok(());
    }
    let promote = state
        .pending_start
        .borrow()
        .as_ref()
        .is_some_and(|request| {
            request.stamp.matches(state)
                && matches!(
                    request.trigger,
                    crate::graph_execution_config::Trigger::Parameter { .. }
                )
        });
    if !promote {
        return request(hwnd, state);
    }
    PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0))?;
    // PostMessage does not reenter synchronously; upgrade after successful enqueue, preserving the original automatic trigger on failure.
    state.pending_start.borrow_mut().as_mut().unwrap().trigger =
        crate::graph_execution_config::Trigger::Manual;
    Ok(())
}

#[cfg(test)]
pub(super) unsafe fn append(menu: HMENU, state: &State) -> Result<()> {
    AppendMenuW(
        menu,
        MF_STRING
            | if available(state) {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
        COMMAND,
        w!("运行所选能力"),
    )
}
