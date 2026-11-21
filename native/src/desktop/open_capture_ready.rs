//! 普通截图借原准备线程句柄即时结算；仍泵消息并由 OpenJob 取消/join，不新增线程或事件。
use super::*;
use std::os::windows::io::AsRawHandle;
use windows::Win32::System::Threading::INFINITE;

pub(super) unsafe fn finish(hwnd: HWND, state: &State) -> Result<()> {
    let result = pump(hwnd, state);
    if result.is_err() {
        retire(hwnd, state);
    }
    result
}
unsafe fn retire(hwnd: HWND, state: &State) {
    let _ = KillTimer(Some(hwnd), OPEN_TIMER);
    let job = state.open_job.borrow_mut().take();
    // 先释放 RefCell 借用再取消/join；不分离任务，不交付取消后的像素。
    drop(job);
}
unsafe fn pump(hwnd: HWND, state: &State) -> Result<()> {
    loop {
        if !IsWindow(Some(hwnd)).as_bool() {
            retire(hwnd, state);
            return Ok(());
        }
        let mut message = MSG::default();
        if PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            match message.message {
                WM_QUIT => {
                    retire(hwnd, state);
                    PostQuitMessage(message.wParam.0 as i32);
                    return Ok(());
                }
                WM_KEYDOWN if message.wParam.0 == 0x1b => cancel(state),
                _ => {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
        }
        // 每轮最多分派一条消息就观察完成，持续鼠标消息不能饿死已完成的截图。
        if !IsWindow(Some(hwnd)).as_bool() {
            retire(hwnd, state);
            return Ok(());
        }
        super::finish(hwnd, state, false);
        let handle = state.open_job.borrow().as_ref().and_then(|job| {
            job.worker
                .as_ref()
                .map(|worker| HANDLE(worker.as_raw_handle()))
        });
        let Some(handle) = handle else { return Ok(()) };
        // Peek/Dispatch 前重新取句柄，OS 等待期间 owner 不变且不持模型借用。
        if MsgWaitForMultipleObjectsEx(Some(&[handle]), INFINITE, QS_ALLINPUT, MWMO_INPUTAVAILABLE)
            == WAIT_FAILED
        {
            return Err(Error::from_win32());
        }
    }
}

#[cfg(test)]
#[path = "open_capture_ready_tests.rs"]
mod tests;
