//! STA 等待仅泵本线程消息；取消异步操作，绝不在 UI 线程轮询。
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
};
pub(super) fn pump() {
    unsafe {
        let mut message = MSG::default();
        // 限制单轮消息量，避免消息洪泛推迟取消检查。
        for _ in 0..32 {
            if !PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                break;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(2));
}
macro_rules! operation {
    ($operation:expr, $cancel:expr, $deadline:expr) => {{
        let cancel = $cancel;
        let deadline = $deadline;
        let operation = $operation;
        let mut stopped = None;
        loop {
            if stopped.is_none() {
                if let Err(error) = crate::clipboard_history::check(cancel, deadline) {
                    let _ = operation.Cancel();
                    stopped = Some(error);
                }
            }
            // Cancel 只是请求。直到操作离开 Started 才释放读取许可，避免超时后
            // 重开菜单累积仍在执行的提供方操作；UI owner 不得阻塞 join 本线程。
            // windows-future 0.1.1 的 AsyncStatus::Started ABI 值为 0。
            if operation.Status()?.0 != 0 {
                break match stopped { Some(error) => Err(error), None => operation.GetResults() };
            }
            crate::clipboard_history::wait::pump();
        }
    }};
}
pub(super) use operation;
#[cfg(test)]
#[path = "clipboard_history_wait_tests.rs"]
mod tests;
