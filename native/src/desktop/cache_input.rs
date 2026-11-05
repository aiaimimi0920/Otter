//! 缓存偏好的内存编辑入口；模态只持草稿，确认不立即清理磁盘，保存交给应用协调器。
use super::State;
use crate::cache_policy::Policy;
use windows::{
    core::{w, Error, Result, HSTRING},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
#[path = "cache_dialog.rs"]
mod dialog;
pub(super) const COMMAND: usize = 0x1063;
pub(super) unsafe fn edit(hwnd: HWND, state: &State) -> Result<()> {
    if let Err(error) = edit_with(hwnd, state, |old| dialog::show(hwnd, old)) {
        MessageBoxW(
            Some(hwnd),
            &HSTRING::from(format!("缓存设置未修改。\n{error}")),
            w!("Otter"),
            MB_OK | MB_ICONWARNING,
        );
    }
    Ok(())
}
unsafe fn edit_with(
    hwnd: HWND,
    state: &State,
    choose: impl FnOnce(Policy) -> Option<Policy>,
) -> Result<()> {
    if state.modals.blocked() {
        return Ok(());
    }
    if !IsWindow(Some(hwnd)).as_bool() {
        return Err(Error::new(E_ABORT, "Cache requester closed"));
    }
    let old = state.cache_policy.get();
    let flag = state.file_dialog.clone();
    let guard = super::file_dialog::Guard::begin(&flag)
        .ok_or_else(|| Error::new(E_ABORT, "Another dialog is active"))?;
    let selected = choose(old);
    drop(guard);
    let Some(next) = selected else {
        return Ok(());
    };
    if !IsWindow(Some(hwnd)).as_bool() || state.cache_policy.get() != old {
        return Err(Error::new(E_ABORT, "Cache policy changed while editing"));
    }
    next.validate()
        .map_err(|error| Error::new(E_INVALIDARG, error))?;
    state.cache_policy.set(next);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancel_invalid_stale_and_shared_settings_use_the_same_edit_transaction() {
        unsafe {
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("Otter cache test"),
                WS_OVERLAPPED,
                0,
                0,
                10,
                10,
                None,
                None,
                None,
                None,
            )
            .unwrap();
            let state = State::new(None, false).unwrap();
            let other = state.cache_policy.clone();
            let old = other.get();
            edit_with(hwnd, &state, |_| None).unwrap();
            assert_eq!(other.get(), old);
            assert!(edit_with(hwnd, &state, |mut value| {
                value.temp_cache_max_bytes = 1;
                Some(value)
            })
            .is_err());
            assert_eq!(other.get(), old);
            assert!(edit_with(hwnd, &state, |value| {
                state.cache_policy.set(Policy {
                    temp_cache_max_bytes: 0,
                    temp_cache_retention_days: 0,
                });
                Some(value)
            })
            .is_err());
            assert_eq!(other.get().temp_cache_retention_days, 0);
            edit_with(hwnd, &state, |_| Some(old)).unwrap();
            assert_eq!(other.get(), old);
            state.file_dialog.set(true);
            assert!(edit_with(hwnd, &state, |_| {
                panic!("modal must not open another editor")
            })
            .is_err());
            state.file_dialog.set(false);
            assert!(super::super::application::handles(COMMAND));
            assert!(super::super::group_lock::allows(COMMAND));
            DestroyWindow(hwnd).unwrap();
            assert!(edit_with(hwnd, &state, |_| Some(old)).is_err());
        }
    }
}
