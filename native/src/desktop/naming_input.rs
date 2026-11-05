//! 共享命名设置编辑；模态期间不持有设置借用，校验后整体应用，失败保留旧配置。
use super::State;
use crate::file_naming_settings::Settings;
#[path = "naming_dialog.rs"]
mod dialog;
use windows::{
    core::{w, Error, Result, HSTRING},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
pub(super) const COMMAND: usize = 0x105d;
pub(super) unsafe fn edit(hwnd: HWND, state: &State) -> Result<()> {
    if let Err(error) = edit_with(hwnd, state, |value| dialog::show(hwnd, value)) {
        MessageBoxW(
            Some(hwnd),
            &HSTRING::from(format!("命名设置未修改。\n{error}")),
            w!("Otter"),
            MB_OK | MB_ICONWARNING,
        );
    }
    Ok(())
}
unsafe fn edit_with(
    hwnd: HWND,
    state: &State,
    choose: impl FnOnce(Settings) -> Option<Settings>,
) -> Result<()> {
    if state.modals.blocked() {
        return Ok(());
    }
    if !IsWindow(Some(hwnd)).as_bool() {
        return Err(Error::new(E_ABORT, "Naming requester closed"));
    }
    let old = state.naming.borrow().clone();
    let flag = state.file_dialog.clone();
    let dialog = super::file_dialog::Guard::begin(&flag)
        .ok_or_else(|| Error::new(E_ABORT, "Another dialog is active"))?;
    let selected = choose(old.clone());
    drop(dialog);
    let Some(next) = selected else {
        return Ok(());
    };
    if !IsWindow(Some(hwnd)).as_bool() || *state.naming.borrow() != old {
        return Err(Error::new(E_ABORT, "Naming settings changed while editing"));
    }
    next.validate().map_err(|error| Error::new(E_INVALIDARG, format!("无效命名模板：{error:?}。支持 {{date}}、{{time}}、{{width}}、{{height}}、{{shortId}} 等占位符。")))?;
    *state.naming.borrow_mut() = next;
    Ok(())
}

pub(super) unsafe fn verify(
    first: &super::document_window::DocumentWindow,
    second: &super::document_window::DocumentWindow,
) -> Result<()> {
    if !std::rc::Rc::ptr_eq(&first.state.naming, &second.state.naming) {
        return Err(Error::new(
            E_FAIL,
            "Workspace windows do not share naming settings",
        ));
    }
    let old = first.state.naming.borrow().clone();
    edit_with(first.window.0, &first.state, |_| None)?;
    if *first.state.naming.borrow() != old {
        return Err(Error::new(
            E_FAIL,
            "Cancelled naming input changed settings",
        ));
    }
    // 模态期间的新设置不能被旧草稿覆盖；回调执行时不持有 RefCell 借用。
    let stale = edit_with(first.window.0, &first.state, |draft| {
        first.state.naming.borrow_mut().title_max_length = 17;
        Some(draft)
    });
    let retained = first.state.naming.borrow().title_max_length == 17;
    *first.state.naming.borrow_mut() = old.clone();
    if stale.is_ok() || !retained {
        return Err(Error::new(
            E_FAIL,
            "Stale naming draft overwrote current settings",
        ));
    }
    if edit_with(first.window.0, &first.state, |mut value| {
        value.sticker_save_pattern = "{bad}".into();
        Some(value)
    })
    .is_ok()
        || *first.state.naming.borrow() != old
    {
        return Err(Error::new(E_FAIL, "Invalid naming input changed settings"));
    }
    edit_with(first.window.0, &first.state, |mut value| {
        value.sticker_save_pattern = "共享_{shortId}_{width}x{height}".into();
        value.drag_export_pattern = "拖出_{title}".into();
        value.clipboard_file_pattern = "剪贴板_{kind}".into();
        value.title_max_length = 12;
        Some(value)
    })?;
    let shared = second.state.naming.borrow().clone();
    *first.state.naming.borrow_mut() = old;
    if shared.sticker_save_pattern != "共享_{shortId}_{width}x{height}"
        || shared.title_max_length != 12
        || shared.drag_export_pattern != "拖出_{title}"
        || shared.clipboard_file_pattern != "剪贴板_{kind}"
    {
        return Err(Error::new(E_FAIL, "Naming input was not shared"));
    }
    println!(
        "native-naming-input-smoke: cancel=true invalid-preserved=true shared=true dialogs=false"
    );
    Ok(())
}
