//! Scrolling-capture control projection/task waits; disabled buttons do not replace admission checks at command execution.
use super::*;
pub(super) unsafe fn abort_error(
    hwnd: HWND,
    input: &Input,
    id: &str,
    error: impl std::fmt::Display,
) {
    *input.error.borrow_mut() = Some(crate::ui_text::format(
        id,
        &[("error", error.to_string().into())],
    ));
    close(hwnd, input);
}
pub(super) unsafe fn status_error(hwnd: HWND, id: &str, error: impl std::fmt::Display) {
    status(
        hwnd,
        &crate::ui_text::format(id, &[("error", error.to_string().into())]),
    );
}
use crate::long_capture::{Error as CaptureError, Snapshot};
/// Modals and recording overlays share message boundaries; errors exit through actual window types without leaving modeless loops.
pub(super) unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> isize {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        handle(hwnd, message, wp, lp)
    }))
    .unwrap_or_else(|_| {
        let pointer = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER)) as *const Input;
        if let Some(input) = pointer.as_ref() {
            if let Ok(mut error) = input.error.try_borrow_mut() {
                *error = Some(crate::ui_text::text("long-capture-internal-error").into());
            }
            overlay::end(hwnd, input, 0);
        }
        1
    })
}
pub(super) unsafe fn status(hwnd: HWND, text: &str) {
    let _ = SetDlgItemTextW(hwnd, 103, &HSTRING::from(text));
    overlay::refresh(hwnd);
}
pub(super) unsafe fn capture_error(
    hwnd: HWND,
    input: &Input,
    snapshot: Snapshot,
    error: CaptureError,
) {
    match error {
        CaptureError::UnconfirmedEdges => {
            if !input.recording.running() {
                status(hwnd, crate::ui_text::text("long-capture-confirm-edges"));
            }
        }
        CaptureError::ChangedEdges => {
            status(hwnd, crate::ui_text::text("long-capture-edges-changed"))
        }
        CaptureError::CrossAxis if input.recording.enabled() => {
            status(hwnd, crate::ui_text::text("long-capture-axis-changed"))
        }
        CaptureError::DocumentLimit => {
            if snapshot.frame_count == 1
                || snapshot.confirmed_edges.is_some()
                || snapshot.spatial_position.is_some()
            {
                status(hwnd, crate::ui_text::text("long-capture-size-limit"));
            } else {
                status(hwnd, crate::ui_text::text("long-capture-selection-limit"));
            }
        }
        other => status(
            hwnd,
            &crate::ui_text::format(
                "long-capture-finish-error",
                &[("error", format!("{other:?}").into())],
            ),
        ),
    }
}
pub(super) unsafe fn frame_status(hwnd: HWND, input: &Input, count: usize) {
    if input.recording.enabled() {
        // Per-frame replies refresh previews only; start, completion, stop, and error boundaries update headings.
        overlay::refresh(hwnd);
    } else {
        let message = if input.job.borrow().can_scroll() {
            "long-capture-frame-count"
        } else {
            "long-capture-frame-count-manual"
        };
        status(
            hwnd,
            &crate::ui_text::format(message, &[("count", count.into())]),
        );
    }
}
pub(super) unsafe fn controls(hwnd: HWND, input: &Input, busy: bool) {
    movement::controls(hwnd, input, busy);
    recording::update_control(hwnd, input, busy);
    let scroll = input.job.borrow().can_scroll();
    let running = input.automatic.borrow().running() || input.recording.running();
    let finishing = input.finish.pending();
    for id in [101, 102, 105, 107] {
        if let Ok(control) = GetDlgItem(Some(hwnd), id) {
            let _ = EnableWindow(
                control,
                !finishing
                    && !busy
                    && (!running || (id == 102 && input.recording.enabled()))
                    && (!matches!(id, 105 | 107) || scroll),
            );
        }
    }
    if let Ok(control) = GetDlgItem(Some(hwnd), 1) {
        let _ = EnableWindow(control, !finishing);
    }
    let direction = match (input.axis, input.forward.get()) {
        (Axis::Vertical, true) => crate::ui_text::text("long-capture-down"),
        (Axis::Vertical, false) => crate::ui_text::text("long-capture-up"),
        (Axis::Horizontal, true) => crate::ui_text::text("long-capture-right"),
        (Axis::Horizontal, false) => crate::ui_text::text("long-capture-left"),
    };
    let _ = SetDlgItemTextW(
        hwnd,
        107,
        &HSTRING::from(if input.recording.enabled() {
            crate::ui_text::text("long-capture-auto-direction")
        } else {
            direction
        }),
    );
    if let Ok(control) = GetDlgItem(Some(hwnd), 106) {
        let _ = EnableWindow(control, !finishing && scroll && (running || !busy));
    }
    let _ = SetDlgItemTextW(
        hwnd,
        106,
        &HSTRING::from(if input.automatic.borrow().running() {
            crate::ui_text::text("long-capture-auto-stop")
        } else {
            crate::ui_text::text("long-capture-auto")
        }),
    );
}
pub(super) unsafe fn clear_preview(hwnd: HWND, input: &Input) {
    input.preview.borrow_mut().take();
    if let Ok(control) = GetDlgItem(Some(hwnd), 104) {
        let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(control), None, true);
    }
}
pub(super) unsafe fn waiting(hwnd: HWND, input: &Input) {
    if !input.automatic.borrow().running() && !input.recording.enabled() {
        clear_preview(hwnd, input);
    }
    controls(hwnd, input, true);
    if !input.recording.enabled() {
        if input.automatic.borrow().running() {
            let count = input.job.borrow().snapshot().frame_count;
            status(
                hwnd,
                &crate::ui_text::format("long-capture-auto-progress", &[("count", count.into())]),
            );
        } else {
            status(hwnd, crate::ui_text::text("long-capture-processing"));
        }
    }
    if SetTimer(Some(hwnd), TIMER, 50, None) == 0 {
        *input.error.borrow_mut() =
            Some(crate::ui_text::text("long-capture-worker-wait-failed").into());
        close(hwnd, input);
    }
}
pub(super) unsafe fn request_auto(hwnd: HWND, input: &Input) {
    if !input
        .automatic
        .borrow_mut()
        .request(std::time::Instant::now())
    {
        controls(hwnd, input, false);
        status(hwnd, crate::ui_text::text("long-capture-auto-limit"));
        return;
    }
    let result = input.job.borrow_mut().scroll_capture(input.forward.get());
    if result.is_ok() {
        waiting(hwnd, input);
    } else {
        input.automatic.borrow_mut().stop();
        controls(hwnd, input, false);
        status(hwnd, crate::ui_text::text("long-capture-auto-failed"));
    }
}
