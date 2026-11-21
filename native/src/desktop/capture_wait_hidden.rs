//! 快速区域截图只泵消息，不显示瞬时等待框；取消/退出仍由原任务 owner 统一回收。
use super::*;
use std::os::windows::io::AsRawHandle;
use windows::Win32::System::Threading::INFINITE;

pub(super) unsafe fn run<T>(owner: HWND, input: &Input<T>) -> Result<(), String> {
    let template = layout::template();
    let window = crate::desktop::Window(
        CreateDialogIndirectParamW(
            None,
            template.as_ptr().cast(),
            Some(owner),
            Some(procedure::<T>),
            LPARAM(input as *const _ as isize),
        )
        .map_err(|error| error.to_string())?,
    );
    let mut message = MSG::default();
    while input.result.borrow().is_none() && IsWindow(Some(window.0)).as_bool() {
        // 先领取消息再借线程句柄；Peek/Dispatch 可重入，不得跨它保留任务借用。
        if PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            if message.message == WM_QUIT {
                PostQuitMessage(message.wParam.0 as i32);
                input.job.borrow_mut().take();
                return Ok(());
            } else if message.message == WM_KEYDOWN && message.wParam.0 == 0x1b {
                cancel(window.0, input, None);
            } else {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        // 输入队列持续有消息时也及时领取结果，不等队列完全空闲。
        poll(window.0, input);
        if input.result.borrow().is_some() || !IsWindow(Some(window.0)).as_bool() {
            break;
        }
        let handle = input
            .job
            .borrow()
            .as_ref()
            .and_then(|job| {
                job.completion_handle()
                    .map(|handle| HANDLE(handle.as_raw_handle()))
            })
            .ok_or("Capture wait lost its pending worker")?;
        // 此等待不分派窗口消息；下一次 Peek 前结束句柄借用，任务仍由 Input 持有。
        if MsgWaitForMultipleObjectsEx(Some(&[handle]), INFINITE, QS_ALLINPUT, MWMO_INPUTAVAILABLE)
            == WAIT_FAILED
        {
            return Err(windows::core::Error::from_win32().to_string());
        }
    }
    Ok(())
}
