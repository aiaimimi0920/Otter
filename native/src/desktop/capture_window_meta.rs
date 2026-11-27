//! Local naming hints only. Query the confirmed target, never a later foreground
//! window. Do not serialize this data into capture metadata or log it.
use super::State;
use crate::file_naming::SourceFields;
use windows::{
    core::PWSTR,
    Win32::{Foundation::*, System::Threading::*, UI::WindowsAndMessaging::*},
};

#[derive(Clone, Default)]
pub(super) struct Meta {
    pub app: String,
    pub title: String,
}
pub(super) unsafe fn window(
    state: &State,
    target: &crate::live_window_target::Target,
) -> Option<Meta> {
    let fields = SourceFields::for_settings(&state.naming.borrow()).ok()?;
    if !fields.app && !fields.title {
        return None;
    }
    target.validate().ok()?;
    let value = read(target.hwnd(), fields)?;
    target.validate().ok()?;
    Some(value)
}
pub(super) unsafe fn region(state: &State, origin: POINT, width: u32, height: u32) -> Option<Meta> {
    let fields = SourceFields::for_settings(&state.naming.borrow()).ok()?;
    if !fields.app && !fields.title {
        return None;
    }
    let pt = POINT {
        x: i32::try_from(i64::from(origin.x) + i64::from(width) / 2).ok()?,
        y: i32::try_from(i64::from(origin.y) + i64::from(height) / 2).ok()?,
    };
    read(GetAncestor(WindowFromPoint(pt), GA_ROOT), fields)
}
unsafe fn read(hwnd: HWND, fields: SourceFields) -> Option<Meta> {
    let mut pid = 0;
    let tid = GetWindowThreadProcessId(hwnd, Some(&mut pid));
    // Never synchronously query a caption on our own UI thread (selectors,
    // badge, dialogs). GetWindowTextW uses the system caption for foreign apps.
    if tid == 0 || pid == 0 || pid == GetCurrentProcessId() {
        return None;
    }
    let mut meta = Meta::default();
    if fields.app {
        if let Ok(proc) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            let mut buffer = vec![0; 32768];
            let mut size = buffer.len() as u32;
            let ok = QueryFullProcessImageNameW(
                proc,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut size,
            )
            .is_ok();
            let _ = CloseHandle(proc);
            if ok {
                if let Ok(path) = String::from_utf16(&buffer[..size as usize]) {
                    meta.app = std::path::Path::new(&path)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .chars()
                        .take(255)
                        .collect();
                }
            }
        }
    }
    if fields.title {
        let mut title = [0u16; 481];
        let n = GetWindowTextW(hwnd, &mut title).max(0) as usize;
        meta.title = char::decode_utf16(title[..n].iter().copied())
            .take(240)
            .map_while(Result::ok)
            .collect();
    }
    let mut again = 0;
    (GetWindowThreadProcessId(hwnd, Some(&mut again)) == tid && again == pid).then_some(meta)
}
