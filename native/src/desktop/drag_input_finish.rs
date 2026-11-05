//! 暂存完成后复核来源并进入 OLE；发布后的缓存文件不因拖动取消或窗口关闭而删除。
use super::*;
use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};

pub(super) unsafe fn poll(hwnd: HWND, state: &State, held: bool) {
    poll_preview(hwnd, state, held, |path, cancel, preview| {
        // 文件发布/句柄准备也可能跨过松键时刻；不得在用户已经松键后新开 OLE 手势。
        if GetAsyncKeyState(VK_LBUTTON.0 as i32) >= 0 {
            return Ok(());
        }
        crate::desktop::drag_transport::drag_files_with_preview(&[path], cancel, Some(&preview))
            .map(|_| ())
            .map_err(|error| error.to_string())
    });
}
struct Active<'a>(&'a Session);
impl Drop for Active<'_> {
    fn drop(&mut self) {
        if let Some(signal) = self.0.active.borrow_mut().take() {
            signal.store(true, Ordering::Release);
        }
    }
}
#[cfg(test)]
pub(super) unsafe fn poll_using(
    hwnd: HWND,
    state: &State,
    held: bool,
    publish: impl FnOnce(PathBuf, Arc<AtomicBool>) -> Result<(), String>,
) {
    poll_preview(hwnd, state, held, |path, cancel, _| publish(path, cancel));
}
unsafe fn poll_preview(
    hwnd: HWND,
    state: &State,
    held: bool,
    publish: impl FnOnce(
        PathBuf,
        Arc<AtomicBool>,
        crate::desktop::drag_preview::Preview,
    ) -> Result<(), String>,
) {
    if !IsWindow(Some(hwnd)).as_bool() {
        dispose(hwnd, state);
        return;
    }
    if !held
        || state
            .drag
            .pending
            .borrow()
            .as_ref()
            .is_some_and(|pending| pending.job.is_cancelled())
    {
        cancel(hwnd, state);
        return;
    }
    if !state
        .drag
        .pending
        .borrow()
        .as_ref()
        .is_some_and(|pending| pending.job.is_finished())
    {
        return;
    }
    let mut pending = state.drag.pending.borrow_mut().take().unwrap();
    let _ = KillTimer(Some(hwnd), TIMER);
    state.drag.origin.set(None);
    // 清除准备阶段鼠标捕获可重入窗口过程；任务已移出槽位，不会被二次回收。
    if GetCapture() == hwnd {
        let _ = ReleaseCapture();
    }
    if !IsWindow(Some(hwnd)).as_bool()
        || state.modals.blocked()
        || state.file_dialog.get()
        || state.group_locked.get()
        || !pending.stamp.matches(
            &state.canvas.borrow(),
            state.resolved_display.borrow().as_ref(),
        )
    {
        return;
    }
    let result = (|| -> Result<(), String> {
        let Some(staged) = pending
            .job
            .take_ready()
            .ok_or("Missing completed drag task")??
        else {
            return Ok(());
        };
        let signal = Arc::new(AtomicBool::new(false));
        let (path, preview) = staged.publish(&signal).map_err(|error| error.to_string())?;
        // OLE 同步调用期间禁止缓存清理替换/删除来源；完成后保留文件供目标继续读取。
        let _file = OpenOptions::new()
            .read(true)
            .share_mode(windows::Win32::Storage::FileSystem::FILE_SHARE_READ.0)
            .open(&path)
            .map_err(|error| error.to_string())?;
        let _modal = crate::desktop::file_dialog::Guard::begin(&state.file_dialog)
            .ok_or("Drag modal already active")?;
        *state.drag.active.borrow_mut() = Some(signal.clone());
        let _active = Active(&state.drag);
        publish(path, signal, preview)
    })();
    if let Err(error) = result {
        eprintln!("native-drag: {error}");
    }
}
