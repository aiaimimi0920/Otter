//! 恢复文件和记录只由明确菜单动作选择；路径不在回调中读取，旧提示不授予权限。
use super::*;
use std::{os::windows::ffi::OsStringExt, path::PathBuf};
use windows::{core::PWSTR, Win32::UI::Controls::Dialogs::*};
pub(super) unsafe fn append_menu(menu: HMENU, view: &View) -> Result<()> {
    let enabled = !view.closed && !view.busy && !view.recovery_busy && view.request.is_none();
    AppendMenuW(
        menu,
        MF_STRING | if enabled { MF_ENABLED } else { MF_GRAYED },
        LOAD_RECOVERY,
        w!("读取 Surface 恢复记录…"),
    )?;
    let restore_enabled = enabled
        && !view.mount_busy
        && view
            .recovery_candidate
            .as_ref()
            .is_some_and(|candidate| view.origin.as_deref() == Some(candidate.origin()));
    AppendMenuW(
        menu,
        MF_STRING
            | if restore_enabled {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
        RESTORE_SURFACE,
        w!("重新挂载恢复候选…"),
    )?;
    if !view.recovery_status.is_empty() {
        AppendMenuW(
            menu,
            MF_STRING | MF_GRAYED,
            0,
            &HSTRING::from(view.recovery_status.replace('&', "&&")),
        )?;
    }
    for (index, record) in view
        .recovery
        .iter()
        .enumerate()
        .take(crate::surface_recovery_record::MAX_RECORDS)
    {
        let enabled =
            enabled && view.origin.as_deref() == Some(record.origin()) && record.art().is_some();
        let label = format!(
            "查询恢复：{} / {}",
            record.art().unwrap_or("未知 Art"),
            record.node()
        )
        .replace('&', "&&");
        AppendMenuW(
            menu,
            MF_STRING | if enabled { MF_ENABLED } else { MF_GRAYED },
            RECOVERY_BASE + index,
            &HSTRING::from(label),
        )?;
    }
    Ok(())
}

pub(super) unsafe fn confirm(hwnd: HWND, state: &State) -> Result<()> {
    confirm_with(hwnd, state, |candidate| {
        let text = format!("重新挂载此候选？\n服务：{}\nArt：{}\n节点：{}\n附件：{}\n\n此操作会联系服务端，不会重新创建 Art。",
            candidate.origin(), candidate.art(), candidate.node(), candidate.attachment());
        MessageBoxW(
            Some(hwnd),
            &HSTRING::from(text),
            w!("恢复 Surface"),
            MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
        ) == IDYES
    })
}
pub(super) unsafe fn confirm_with(
    hwnd: HWND,
    state: &State,
    choose: impl FnOnce(&crate::live_relay_http::RecoveryCandidate) -> bool,
) -> Result<()> {
    if state.modals.blocked() || state.exit_requested.get() {
        return Ok(());
    }
    if !IsWindow(Some(hwnd)).as_bool() {
        return Err(Error::new(E_ABORT, "Recovery requester closed"));
    }
    let candidate = {
        let view = state.authorization.borrow();
        if view.closed
            || view.busy
            || view.recovery_busy
            || view.mount_busy
            || view.request.is_some()
        {
            return Err(Error::new(E_ABORT, "Recovery request unavailable"));
        }
        view.recovery_candidate
            .clone()
            .filter(|candidate| view.origin.as_deref() == Some(candidate.origin()))
            .ok_or_else(|| Error::new(E_ABORT, "Recovery candidate unavailable"))?
    };
    let flag = state.file_dialog.clone();
    let guard = super::super::file_dialog::Guard::begin(&flag)
        .ok_or_else(|| Error::new(E_ABORT, "Another dialog is active"))?;
    let accepted = choose(&candidate);
    drop(guard);
    if !accepted {
        return Ok(());
    }
    let current = {
        let view = state.authorization.borrow();
        view.origin.as_deref() == Some(candidate.origin())
            && view
                .recovery_candidate
                .as_ref()
                .is_some_and(|current| std::sync::Arc::ptr_eq(current, &candidate))
    };
    if !current {
        return Err(Error::new(
            E_ABORT,
            "Recovery candidate changed during confirmation",
        ));
    }
    queue(hwnd, state, Request::RestoreSurface(candidate))
}
pub(super) unsafe fn choose(hwnd: HWND, state: &State) -> Result<()> {
    if state.modals.blocked() || state.exit_requested.get() {
        return Ok(());
    }
    let flag = state.file_dialog.clone();
    let guard = super::super::file_dialog::Guard::begin(&flag)
        .ok_or_else(|| Error::new(E_ABORT, "Another dialog is active"))?;
    let mut name = vec![0u16; 32768];
    let mut options = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFile: PWSTR(name.as_mut_ptr()),
        nMaxFile: name.len() as u32,
        lpstrFilter: w!("Surface recovery metadata\0*.json\0\0"),
        lpstrTitle: w!("读取 Surface 恢复记录"),
        Flags: OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        ..Default::default()
    };
    let accepted = GetOpenFileNameW(&mut options).as_bool();
    let error = CommDlgExtendedError();
    drop(guard);
    if !accepted {
        return if error == COMMON_DLG_ERRORS(0) {
            Ok(())
        } else {
            Err(Error::new(
                E_FAIL,
                format!("Recovery dialog failed: {error:?}"),
            ))
        };
    }
    let length = name
        .iter()
        .position(|c| *c == 0)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid recovery file path"))?;
    queue(
        hwnd,
        state,
        Request::LoadRecovery(PathBuf::from(std::ffi::OsString::from_wide(
            &name[..length],
        ))),
    )
}
