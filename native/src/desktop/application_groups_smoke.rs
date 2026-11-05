//! 自有隐藏窗口验证分组请求/菜单/关闭链；不依赖人工输入名称或操作系统键鼠。
use super::*;
use crate::desktop::group_menu::{self, Request};

pub(super) unsafe fn run(instance: HINSTANCE) -> Result<()> {
    let first = DocumentWindow::create(instance, State::new(None, false)?, false)?;
    let mut app = Application::new(instance, first);
    app.add_window(false)?;
    app.add_window(false)?; // 第三个窗口不入组，验证关闭组不等于退出应用。
    let source = &app.documents[0];
    group_menu::queue(
        source.window.0,
        &source.state,
        Request::Create("原生分组".into()),
    )?;
    group_menu::queue(
        source.window.0,
        &source.state,
        Request::Create("不应排队".into()),
    )?;
    let modal_flag = app.documents[2].state.file_dialog.clone();
    let modal = super::super::file_dialog::Guard::begin(&modal_flag).unwrap();
    app.pending_groups(false)?;
    if !app.workspace.groups().is_empty() || app.documents[0].state.group_request.borrow().is_none()
    {
        return Err(Error::new(
            E_FAIL,
            "Group request was not deferred during modal",
        ));
    }
    drop(modal);
    app.pending_groups(false)?;
    if app.workspace.groups().len() != 1 || app.documents[1].state.group_catalog.borrow().len() != 1
    {
        return Err(Error::new(
            E_FAIL,
            "Group creation did not synchronize menus",
        ));
    }
    let id = app.workspace.groups()[0].id.clone();
    let second = &app.documents[1];
    super::super::context_menu::execute(second.window.0, &second.state, group_menu::JOIN_BASE)?;
    app.pending_groups(false)?;
    let source = &app.documents[0];
    group_menu::queue(
        source.window.0,
        &source.state,
        Request::Rename(id.clone(), "新名称".into()),
    )?;
    app.pending_groups(false)?;
    if app.documents[1].state.group_catalog.borrow()[0].name != "新名称"
        || app.documents[1].state.group_id.borrow().as_ref() != Some(&id)
    {
        return Err(Error::new(
            E_FAIL,
            "Group rename or assignment did not synchronize",
        ));
    }
    super::group_lock_smoke::run(&mut app)?;
    super::group_visibility_smoke::run(&mut app)?;
    let source = &app.documents[0];
    super::super::context_menu::execute(source.window.0, &source.state, group_menu::LEAVE)?;
    app.pending_groups(false)?;
    if app.documents[0].state.group_id.borrow().is_some()
        || app.documents[1].state.group_id.borrow().is_none()
    {
        return Err(Error::new(E_FAIL, "Leaving group changed other member"));
    }
    let second = &app.documents[1];
    super::super::context_menu::execute(second.window.0, &second.state, group_menu::DELETE)?;
    app.pending_groups(false)?;
    if app.documents.len() != 3
        || !app.workspace.groups().is_empty()
        || app
            .documents
            .iter()
            .any(|document| document.state.group_id.borrow().is_some())
    {
        return Err(Error::new(
            E_FAIL,
            "Deleting group closed windows or retained members",
        ));
    }
    empty_catalog(&mut app)?;
    let source = &app.documents[0];
    group_menu::queue(
        source.window.0,
        &source.state,
        Request::Create("关闭测试".into()),
    )?;
    app.pending_groups(false)?;
    let second = &app.documents[1];
    group_menu::execute(second.window.0, &second.state, group_menu::JOIN_BASE)?;
    app.pending_groups(false)?;
    let source = &app.documents[0];
    super::super::context_menu::execute(source.window.0, &source.state, group_menu::CLOSE)?;
    app.pending_groups(false)?;
    if app.documents.len() != 1
        || !app.documents[0].is_alive()
        || !app.workspace.groups().is_empty()
    {
        return Err(Error::new(
            E_FAIL,
            "Group close did not preserve unrelated window",
        ));
    }
    request_exit(app.documents[0].window.0, &app.documents[0].state)?;
    app.close_requested_with(|_| exit::Choice::Discard);
    println!("native-group-commands-smoke: create=true join=true rename=true leave=true delete-preserves=true close-members=true modal-deferred=true");
    Ok(())
}

unsafe fn empty_catalog(app: &mut Application) -> Result<()> {
    use group_menu::catalog;
    let id = super::super::document_state::new_unit_id()?;
    app.workspace
        .upsert_group(crate::workspace::Group {
            id: id.clone(),
            name: "空组".into(),
            hidden: false,
            locked: false,
        })
        .map_err(|error| Error::new(E_FAIL, format!("Empty group fixture: {error:?}")))?;
    app.sync_groups();
    let source = &app.documents[0];
    catalog::execute_with(source.window.0, &source.state, catalog::RENAME_BASE, |_| {
        Some("空组重命名".into())
    })?;
    app.pending_groups(false)?;
    if app.workspace.groups()[0].name != "空组重命名" {
        return Err(Error::new(E_FAIL, "Empty group rename failed"));
    }
    for command in [catalog::LOCK_BASE, catalog::DELETE_BASE] {
        let source = &app.documents[0];
        super::super::context_menu::execute(source.window.0, &source.state, command)?;
        app.pending_groups(false)?;
        if app.documents.len() != 3
            || app
                .documents
                .iter()
                .any(|document| document.state.group_locked.get())
        {
            return Err(Error::new(
                E_FAIL,
                "Empty group action changed unrelated documents",
            ));
        }
    }
    if !app.workspace.groups().is_empty() {
        return Err(Error::new(E_FAIL, "Empty group deletion failed"));
    }
    app.workspace
        .upsert_group(crate::workspace::Group {
            id,
            name: "关闭空组".into(),
            hidden: false,
            locked: false,
        })
        .map_err(|error| Error::new(E_FAIL, format!("Empty group fixture: {error:?}")))?;
    app.sync_groups();
    let source = &app.documents[0];
    super::super::context_menu::execute(source.window.0, &source.state, catalog::CLOSE_BASE)?;
    app.pending_groups(false)?;
    if !app.workspace.groups().is_empty() || app.documents.len() != 3 {
        return Err(Error::new(E_FAIL, "Closing empty group affected documents"));
    }
    println!("native-group-catalog-smoke: empty-rename=true empty-delete=true empty-close=true unrelated-preserved=true dialog-access=false");
    Ok(())
}
