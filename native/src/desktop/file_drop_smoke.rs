//! 隐藏窗口拖入验证；只发送到本进程 HWND，不操作 Explorer、剪贴板或系统鼠标。
use super::State;
use std::{os::windows::ffi::OsStrExt, path::Path};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        System::Memory::*,
        UI::{Shell::*, WindowsAndMessaging::*},
    },
};

fn require(ok: bool, message: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::new(E_FAIL, message))
    }
}

pub(super) unsafe fn send(hwnd: HWND, path: &Path) -> Result<()> {
    send_many(hwnd, &[path])
}
pub(super) unsafe fn send_many(hwnd: HWND, paths: &[&Path]) -> Result<()> {
    let name: Vec<u16> = paths
        .iter()
        .flat_map(|path| path.as_os_str().encode_wide().chain([0]))
        .chain([0])
        .collect();
    require(
        name.len() > 2 && name.len() <= 32769 * super::open_job::MAX_BATCH,
        "Drop smoke path exceeds limit",
    )?;
    let size = std::mem::size_of::<DROPFILES>();
    let memory = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, size + name.len() * 2)?;
    let pointer = GlobalLock(memory).cast::<u8>();
    if pointer.is_null() {
        let error = Error::from_win32();
        let _ = GlobalFree(Some(memory));
        return Err(error);
    }
    pointer.cast::<DROPFILES>().write(DROPFILES {
        pFiles: size as u32,
        fWide: true.into(),
        ..Default::default()
    });
    std::ptr::copy_nonoverlapping(
        name.as_ptr().cast::<u8>(),
        pointer.add(size),
        name.len() * 2,
    );
    let _ = GlobalUnlock(memory);
    // WM_DROPFILES 接收方负责 DragFinish；同步投递后发送方不得再次释放。
    SendMessageW(hwnd, WM_DROPFILES, Some(WPARAM(memory.0 as usize)), None);
    Ok(())
}

pub(super) unsafe fn run(hwnd: HWND, state: &State, path: &Path) -> Result<()> {
    require(
        GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_ACCEPTFILES.0 != 0,
        "Drop acceptance not enabled",
    )?;
    let stamp = state.canvas.borrow().editor.document_stamp();
    {
        let _modal = super::file_dialog::Guard::begin(&state.file_dialog)
            .ok_or_else(|| Error::new(E_FAIL, "Unexpected active file dialog"))?;
        super::open_image::start(hwnd, state);
        super::export_job::start(hwnd, state);
        send(hwnd, path)?;
        require(
            state.open_job.borrow().is_none() && state.export.borrow().is_none(),
            "Modal reentry started worker",
        )?;
        require(
            stamp.matches(&state.canvas.borrow().editor),
            "Modal reentry changed document",
        )?;
        require(
            state.file_dialog.get(),
            "Reentry released outer modal guard",
        )?;
    }
    require(!state.file_dialog.get(), "Modal guard did not release")?;
    send(hwnd, path)?;
    require(
        state.open_job.borrow().is_some(),
        "Drop did not start image worker",
    )?;
    require(
        stamp.matches(&state.canvas.borrow().editor),
        "Drop committed before timer",
    )?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while state.open_job.borrow().is_some() {
        require(std::time::Instant::now() < deadline, "Drop worker timeout")?;
        require(IsWindow(Some(hwnd)).as_bool(), "Drop window destroyed")?;
        let mut message = MSG::default();
        while PeekMessageW(&mut message, Some(hwnd), 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        if let Some(error) = state.error.borrow_mut().take() {
            return Err(error);
        }
        crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1));
    }
    require(
        stamp.matches(&state.canvas.borrow().editor),
        "Drop replaced source document",
    )?;
    require(
        state.new_pin.borrow().is_some(),
        "Drop produced no prepared pin",
    )?;
    // 该独立宿主探针没有 Application owner，消费候选避免影响后续截图探针。
    state.new_pin.borrow_mut().take();
    println!("native-drop-smoke: shell=true worker=true timer=true source-preserved=true");
    println!("native-file-dialog-smoke: open-blocked=true save-blocked=true drop-blocked=true recovered=true");
    Ok(())
}
