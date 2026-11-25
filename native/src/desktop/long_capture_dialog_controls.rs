//! Scrolling-capture control projection/task waits; disabled buttons do not replace admission checks at command execution.
use super::*;
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
                *error = Some("长截图窗口发生内部错误，录制已关闭。".into());
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
                status(hwnd, "请继续滚动并采集一帧，以确认固定边缘");
            }
        }
        CaptureError::ChangedEdges => status(hwnd, "固定边缘已变化，请取消并重新选择区域"),
        CaptureError::CrossAxis if input.recording.enabled() => {
            status(hwnd, "滚动轴已变化，录制已停止；可完成已有内容或取消")
        }
        CaptureError::DocumentLimit => {
            if snapshot.frame_count == 1
                || snapshot.confirmed_edges.is_some()
                || snapshot.spatial_position.is_some()
            {
                status(hwnd, "已达钉图尺寸上限，可完成已有内容或取消");
            } else {
                status(hwnd, "已达钉图尺寸上限，请缩小选区后重新录制");
            }
        }
        other => status(hwnd, &format!("无法完成：{other:?}")),
    }
}
pub(super) unsafe fn frame_status(hwnd: HWND, input: &Input, count: usize) {
    if input.recording.enabled() {
        // Per-frame replies refresh previews only; start, completion, stop, and error boundaries update headings.
        overlay::refresh(hwnd);
    } else {
        let mode = if input.job.borrow().can_scroll() {
            ""
        } else {
            " · 手动滚动"
        };
        status(hwnd, &format!("已采集 {count} 帧{mode}"));
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
        (Axis::Vertical, true) => "方向：向下",
        (Axis::Vertical, false) => "方向：向上",
        (Axis::Horizontal, true) => "方向：向右",
        (Axis::Horizontal, false) => "方向：向左",
    };
    let _ = SetDlgItemTextW(
        hwnd,
        107,
        &HSTRING::from(if input.recording.enabled() {
            "方向：自动识别"
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
            "停止自动"
        } else {
            "自动采集"
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
            status(hwnd, &format!("自动采集中：已采集 {count} 帧"));
        } else {
            status(hwnd, "处理中…");
        }
    }
    if SetTimer(Some(hwnd), TIMER, 50, None) == 0 {
        *input.error.borrow_mut() = Some("无法等待长截图后台任务。".into());
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
        status(hwnd, "自动采集已达到次数或时间上限");
        return;
    }
    let result = input.job.borrow_mut().scroll_capture(input.forward.get());
    if result.is_ok() {
        waiting(hwnd, input);
    } else {
        input.automatic.borrow_mut().stop();
        controls(hwnd, input, false);
        status(hwnd, "无法继续自动采集");
    }
}
