//! 分组隐藏恢复链注入托盘安装/显示适配器，不注册系统通知图标或抢占前景窗口。
use super::*;
use crate::desktop::{
    group_menu::{self, Request},
    tray,
};
pub(super) unsafe fn run(app: &mut Application) -> Result<()> {
    let id = app.documents[0].state.group_id.borrow().clone().unwrap();
    let mut calls = 0;
    let failed = app.set_group_hidden_with(
        &id,
        true,
        |hwnd, state| {
            calls += 1;
            if calls == 2 {
                return Err(Error::new(E_FAIL, "fixture tray installation failed"));
            }
            tray::hide_with(hwnd, state, || Ok(()))
        },
        |hwnd, state| tray::restore_with(hwnd, state, |_, _| {}, |_| {}),
    );
    if failed.is_ok()
        || app
            .workspace
            .groups()
            .iter()
            .find(|group| group.id == id)
            .unwrap()
            .hidden
        || app
            .documents
            .iter()
            .any(|document| tray::is_hidden(&document.state))
    {
        return Err(Error::new(
            E_FAIL,
            "Failed group hide retained partial hidden state",
        ));
    }
    let source = &app.documents[0];
    super::super::context_menu::execute(source.window.0, &source.state, group_menu::HIDE)?;
    let request = source.state.group_request.borrow_mut().take();
    if !matches!(request, Some(Request::SetHidden(ref target, true)) if *target == id) {
        return Err(Error::new(E_FAIL, "Hide menu did not target current group"));
    }
    app.set_group_hidden_with(
        &id,
        true,
        |hwnd, state| tray::hide_with(hwnd, state, || Ok(())),
        |hwnd, state| tray::restore_with(hwnd, state, |_, _| {}, |_| {}),
    )?;
    app.sync_groups();
    if !app.documents[0].state.group_hidden.get()
        || !app.documents[1].state.group_hidden.get()
        || app.documents[2].state.group_hidden.get()
        || tray::is_hidden(&app.documents[2].state)
    {
        return Err(Error::new(E_FAIL, "Group hide included unrelated window"));
    }
    let source = &app.documents[1];
    tray::restore(source.window.0, &source.state);
    let request = source.state.group_request.borrow_mut().take();
    if !matches!(request, Some(Request::SetHidden(ref target, false)) if *target == id) {
        return Err(Error::new(
            E_FAIL,
            "Tray restore did not request whole group",
        ));
    }
    app.set_group_hidden_with(
        &id,
        false,
        |_, _| Err(Error::new(E_FAIL, "Restore must not install tray")),
        |hwnd, state| tray::restore_with(hwnd, state, |_, _| {}, |_| {}),
    )?;
    app.sync_groups();
    if app.documents.iter().any(|document| {
        document.state.group_hidden.get()
            || tray::is_hidden(&document.state)
            || IsWindowVisible(document.window.0).as_bool()
    }) {
        return Err(Error::new(
            E_FAIL,
            "Group restore retained hidden state or displayed fixture",
        ));
    }
    println!("native-group-visibility-smoke: members-only=true failure-rollback=true tray-restores-group=true state-restored=true system-tray=false");
    Ok(())
}
