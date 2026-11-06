//! 快速区域截图只泵消息，不显示瞬时等待框；取消/退出仍由原任务 owner 统一回收。
use super::*;

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
        let status = GetMessageW(&mut message, None, 0, 0).0;
        if status <= 0 {
            if status == 0 {
                PostQuitMessage(message.wParam.0 as i32);
            }
            if let Some(job) = input.job.borrow().as_ref() {
                job.cancel();
            }
            input.job.borrow_mut().take();
            return if status == -1 {
                Err(windows::core::Error::from_win32().to_string())
            } else {
                Ok(())
            };
        }
        if message.message == WM_KEYDOWN && message.wParam.0 == 0x1b {
            cancel(window.0, input, None);
        } else {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    Ok(())
}
