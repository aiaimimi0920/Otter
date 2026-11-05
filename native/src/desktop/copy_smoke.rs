//! HWND 复制任务验证；不泵系统消息，直接调用共用轮询，唯一发布回调永不访问剪贴板。
use super::{copy_job, State};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

fn require(ok: bool, message: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::new(E_FAIL, message))
    }
}
unsafe fn finish(hwnd: HWND, state: &State) -> Result<usize> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut publications = 0;
    let mut valid = true;
    while state.copy_job.borrow().is_some() {
        require(
            std::time::Instant::now() < deadline,
            "Copy smoke worker timeout",
        )?;
        // 不 DispatchMessage，避免 WM_TIMER 进入真实剪贴板发布。共用 poll 会 KillTimer。
        copy_job::poll_using(hwnd, state, |owner, payload| {
            publications += 1;
            valid &= owner == hwnd
                && payload.png.starts_with(b"\x89PNG\r\n\x1a\n")
                && payload.dibv5.len() >= 124
                && payload.dibv5[..4] == 124u32.to_le_bytes();
            Ok(())
        });
        crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1));
    }
    require(valid, "Copy smoke malformed representations")?;
    // 过期定时器即使再次轮询也不能再次交付。
    copy_job::poll_using(hwnd, state, |_, _| {
        publications += 1;
        Ok(())
    });
    Ok(publications)
}
pub(super) unsafe fn run(hwnd: HWND, state: &State) -> Result<()> {
    let stamp = state.canvas.borrow().editor.document_stamp();
    copy_job::start_with_plan(hwnd, state, None);
    require(
        state.copy_job.borrow().is_some(),
        "Copy smoke task not started",
    )?;
    copy_job::start_with_plan(hwnd, state, None); // 重复复制不增加待处理队列。
    require(
        finish(hwnd, state)? == 1,
        "Copy smoke did not publish exactly once",
    )?;
    require(
        stamp.matches(&state.canvas.borrow().editor),
        "Copy changed source document",
    )?;

    copy_job::start_with_plan(hwnd, state, None);
    require(
        state.copy_job.borrow().is_some(),
        "Copy cancellation task not started",
    )?;
    SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(0x1B)), None);
    require(finish(hwnd, state)? == 0, "Cancelled copy published")?;

    copy_job::start_with_plan(hwnd, state, None);
    require(
        state.copy_job.borrow().is_some(),
        "Stale copy task not started",
    )?;
    let original = {
        let mut canvas = state.canvas.borrow_mut();
        let replacement = canvas.editor.export_snapshot();
        std::mem::replace(&mut canvas.editor, replacement)
    };
    let result = finish(hwnd, state);
    state.canvas.borrow_mut().editor = original;
    require(result? == 0, "Replaced document accepted old copy")?;
    require(
        stamp.matches(&state.canvas.borrow().editor),
        "Copy smoke did not restore document",
    )?;
    println!(
        "native-copy-smoke: prepared=true once=true escape=true stale=true clipboard-access=false"
    );
    Ok(())
}
