//! 长截图控件投影与任务等待；禁用按钮不替代命令执行时的准入复核。
use super::*;
pub(super) unsafe fn status(hwnd: HWND, text: &str) {
    let _ = SetDlgItemTextW(hwnd, 103, &HSTRING::from(text));
}
pub(super) unsafe fn frame_status(hwnd: HWND, input: &Input, count: usize) {
    if input.recording.enabled() {
        let snapshot = input.job.borrow().snapshot();
        let direction = match snapshot.direction {
            Some(crate::long_capture::Direction::Up) => "向上",
            Some(crate::long_capture::Direction::Down) => "向下",
            Some(crate::long_capture::Direction::Left) => "向左",
            Some(crate::long_capture::Direction::Right) => "向右",
            None => "方向待识别",
        };
        status(
            hwnd,
            &format!(
                "{} · 已采集 {count} 帧 · {direction}",
                if input.recording.running() {
                    "录制中"
                } else {
                    "已停止"
                }
            ),
        );
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
    if input.recording.running() {
        frame_status(hwnd, input, input.job.borrow().snapshot().frame_count);
    } else if input.automatic.borrow().running() {
        let count = input.job.borrow().snapshot().frame_count;
        status(hwnd, &format!("自动采集中：已采集 {count} 帧"));
    } else {
        status(hwnd, "处理中…");
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
