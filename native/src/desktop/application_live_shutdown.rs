//! 正常关闭继续泵消息，避免驱动/跨线程窗口调用等待 UI 而 UI 正在 join worker。
use super::*;

struct Quit(Option<i32>);
impl Drop for Quit {
    fn drop(&mut self) {
        if let Some(code) = self.0 {
            unsafe { PostQuitMessage(code) };
        }
    }
}

pub(super) fn drain(
    sessions: &mut Vec<Session>,
    visibility: &mut crate::live_visibility_owner::Owner,
    authorization: &mut crate::device_authorization_owner::Owner,
    remote: &mut crate::desktop::composition::relay::sessions::Sessions,
) -> Result<()> {
    for session in sessions.iter_mut() {
        session.stop();
    }
    visibility.close();
    remote.close();
    authorization.close();
    let mut quit = Quit(None);
    let mut first = None;
    while !sessions.is_empty()
        || !visibility.finished()
        || !authorization.finished()
        || !remote.finished()
    {
        authorization.poll();
        remote.poll_sources();
        remote.poll_projection();
        for (_, error) in remote.poll_surfaces() {
            first.get_or_insert_with(|| Error::new(E_FAIL, format!("{error:?}")));
        }
        if let Some(error) = remote.poll_catalog() {
            first.get_or_insert_with(|| Error::new(E_FAIL, format!("{error:?}")));
        }
        for event in remote.poll() {
            if let Some(error) = event.outcome.error_message() {
                first.get_or_insert_with(|| Error::new(E_FAIL, error));
            }
        }
        if let Err(error) = visibility.poll() {
            first.get_or_insert_with(|| Error::new(E_FAIL, error.to_string()));
        }
        let mut index = 0;
        while index < sessions.len() {
            if let Some(result) = sessions[index].poll() {
                sessions.swap_remove(index);
                if let Err(error) = result {
                    first.get_or_insert_with(|| Error::new(E_FAIL, error));
                }
            } else {
                index += 1;
            }
        }
        if sessions.is_empty()
            && visibility.finished()
            && authorization.finished()
            && remote.finished()
        {
            break;
        }
        let mut handles = [HANDLE::default();
            MAX_SESSIONS * 2 + 6 + crate::desktop::composition::relay::sessions::MAX_SESSIONS];
        let mut count = 0;
        for session in sessions.iter() {
            for handle in [
                session.completion_handle(),
                session.input_completion_handle(),
            ]
            .into_iter()
            .flatten()
            {
                append_wait_handle(&mut handles, &mut count, HANDLE(handle.as_raw_handle()))?;
            }
        }
        for handle in [visibility.completion_handle(), visibility.child_handle()]
            .into_iter()
            .flatten()
        {
            append_wait_handle(&mut handles, &mut count, HANDLE(handle.as_raw_handle()))?;
        }
        for handle in authorization.wait_handles() {
            append_wait_handle(&mut handles, &mut count, HANDLE(handle.as_raw_handle()))?;
        }
        for handle in remote.wait_handles() {
            append_wait_handle(&mut handles, &mut count, HANDLE(handle.as_raw_handle()))?;
        }
        if count == 0 {
            return Err(Error::new(E_FAIL, "Live shutdown has no completion owner"));
        }
        let mut message = MSG::default();
        unsafe {
            if wait::next(&handles[..count], &mut message)? == 0 {
                // WM_QUIT 不取消清理；结束后原样还给外层，而不是重新入队造成忙循环。
                quit.0 = Some(message.wParam.0 as i32);
            } else {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
    first.map_or(Ok(()), Err)
}

#[cfg(test)]
#[path = "application_live_shutdown_tests.rs"]
mod tests;
