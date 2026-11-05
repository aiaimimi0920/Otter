//! 隐藏窗口通过生产菜单命令进入文档集合，验证请求合并、首窗口关闭及再次建窗。
use super::*;
use crate::desktop::smoke_timing::run as timed;
#[path = "application_history_smoke.rs"]
mod history;
#[path = "application_smoke_image.rs"]
pub(super) mod image_fixture;

pub(in crate::desktop) unsafe fn run(
    instance: HINSTANCE,
    shader: crate::desktop::startup_entry::shader::Loaded,
) -> Result<()> {
    super::shader_startup::smoke(instance, shader)?;
    timed("workspace-autosave", || {
        super::workspace_autosave::smoke::run(instance)
    })?;
    timed("recycle-settings", || {
        super::frozen_menu::settings::smoke::run(instance)
    })?;
    timed("recycle-maintenance", || {
        super::frozen_menu::maintenance::smoke::run(instance)
    })?;
    timed("recycle-close", || {
        super::frozen_menu::recycle_close::smoke::run(instance)
    })?;
    timed("frozen-remove", || {
        super::frozen_menu::remove_smoke::run(instance)
    })?;
    timed("reference", || {
        super::frozen_menu::reference::smoke::run(instance)
    })?;
    timed("frozen-menu", || super::frozen_menu::smoke::run(instance))?;
    timed("groups", || super::groups_smoke::run(instance))?;
    let first = DocumentWindow::create(instance, State::new(None, false)?, false)?;
    let mut app = Application::new(instance, first);
    timed("save-context", || super::save_context::verify(&mut app))?;
    timed("restore-window", || super::restore_window::smoke(&app))?;
    timed("restore-batch", || {
        super::restore_batch::smoke::run(&mut app)
    })?;
    let original_id = app.documents[0].state.unit_id.clone();
    let stamp = app.documents[0]
        .state
        .canvas
        .borrow()
        .editor
        .document_stamp();
    for _ in 0..2 {
        super::super::context_menu::execute(
            app.documents[0].window.0,
            &app.documents[0].state,
            NEW_WINDOW,
        )?;
    }
    app.pending_windows(false)?;
    if app.documents.len() != 2
        || !stamp.matches(&app.documents[0].state.canvas.borrow().editor)
        || app.documents[1].state.canvas.borrow().image.is_some()
        || app.documents[1].state.unit_id == original_id
    {
        return Err(Error::new(
            E_FAIL,
            "New window request did not preserve independent documents",
        ));
    }
    let old = app.documents[0].window.0;
    timed("shared-capture", || {
        super::super::capture_input::smoke_shared(&app.documents[0], &app.documents[1])
    })?;
    timed("application-hotkey", || {
        super::super::application_hotkey::smoke(&app.documents)
    })?;
    timed("application-pin", || super::pin_smoke::run(&mut app))?;
    timed("paste", || super::paste_smoke::run(&mut app))?;
    timed("history", || history::run(&mut app))?;
    timed("open", || super::open_smoke::run(&mut app))?;
    timed("drop", || super::drop_smoke::run(&mut app))?;
    if app.workspace.units().len() != app.documents.len()
        || !app
            .documents
            .iter()
            .all(|document| app.workspace.contains(&document.state.unit_id))
    {
        return Err(Error::new(
            E_FAIL,
            "Workspace registry diverged from document owners",
        ));
    }
    for (key, from, to) in [("fixture-forward", 0, 1), ("fixture-reverse", 1, 0)] {
        app.workspace
            .add_link(crate::workspace::Link {
                id: crate::workspace_id::UnitId::parse(key).unwrap(),
                from_unit: app.documents[from].state.unit_id.clone(),
                from_port: "output".into(),
                to_unit: app.documents[to].state.unit_id.clone(),
                to_port: "image".into(),
            })
            .map_err(|error| {
                Error::new(
                    E_FAIL,
                    format!("Fixture link registration failed: {error:?}"),
                )
            })?;
    }
    let group_id = crate::workspace_id::UnitId::parse("fixture-group").unwrap();
    app.workspace
        .upsert_group(crate::workspace::Group {
            id: group_id.clone(),
            name: "测试分组".into(),
            hidden: false,
            locked: false,
        })
        .map_err(|error| Error::new(E_FAIL, format!("Fixture group failed: {error:?}")))?;
    let members = app.workspace.units().to_vec();
    app.workspace
        .assign_group(&members, Some(&group_id))
        .map_err(|error| {
            Error::new(
                E_FAIL,
                format!("Fixture group assignment failed: {error:?}"),
            )
        })?;
    if app.documents[0].state.unit_id != original_id || app.source_window(&original_id) != Some(old)
    {
        return Err(Error::new(
            E_FAIL,
            "Image replacement or editing changed workspace identity",
        ));
    }
    let request_unit = app.documents[1].state.unit_id.clone();
    let mut requests = Vec::new();
    for unit in [&original_id, &request_unit] {
        let request = app
            .executions
            .begin(
                unit.clone(),
                crate::workspace_id::UnitId::parse("fixture-request").unwrap(),
            )
            .map_err(|error| Error::new(E_FAIL, format!("Request fixture: {error:?}")))?;
        app.executions
            .mark_preview(unit, &request, "fixture-preview".into())
            .map_err(|error| Error::new(E_FAIL, format!("Preview fixture: {error:?}")))?;
        requests.push(request);
    }
    SendMessageW(old, WM_CLOSE, None, None);
    if app.source_window(&original_id).is_some() {
        return Err(Error::new(
            E_FAIL,
            "Closed workspace identity still resolves a window",
        ));
    }
    app.collect_closed();
    if app.executions.active(&original_id).is_some()
        || !app.executions.is_latest(&request_unit, &requests[1])
        || app.executions.preview_bytes() != "fixture-preview".len()
        || app
            .executions
            .mark_preview(&original_id, &requests[0], "stale".into())
            .unwrap_or(true)
    {
        return Err(Error::new(
            E_FAIL,
            "Closing document did not retire execution request",
        ));
    }
    if app.documents.len() != 1
        || !app.documents[0].is_alive()
        || app.capture_windows.borrow().len() != 2
        || app.workspace.units().len() != 1
        || !app.workspace.links().is_empty()
        || app.workspace.group_of(&original_id).is_some()
        || app
            .workspace
            .group_of(&app.documents[0].state.unit_id)
            .is_none()
    {
        return Err(Error::new(
            E_FAIL,
            "Closing initial window removed the remaining document",
        ));
    }
    let remaining = &app.documents[0];
    let before = remaining.state.canvas.borrow().paints;
    SendMessageW(remaining.window.0, WM_PAINT, None, None);
    if remaining.state.canvas.borrow().paints != before + 1
        || remaining.state.error.borrow().is_some()
    {
        return Err(Error::new(
            E_FAIL,
            "Remaining application window cannot render",
        ));
    }
    request(remaining.window.0, &remaining.state)?;
    app.pending_windows(false)?;
    if app.documents.len() != 2 || !app.documents.iter().all(|document| document.is_alive()) {
        return Err(Error::new(
            E_FAIL,
            "Cannot create another window after initial close",
        ));
    }
    if app.source_window(&original_id).is_some()
        || app.documents[0].state.unit_id == app.documents[1].state.unit_id
    {
        return Err(Error::new(
            E_FAIL,
            "New window reused closed workspace identity",
        ));
    }
    exit::verify(&mut app)?;
    super::super::naming_input::verify(&app.documents[0], &app.documents[1])?;
    let modal_flag = app.documents[0].state.file_dialog.clone();
    let modal = super::super::file_dialog::Guard::begin(&modal_flag).unwrap();
    let windows: Vec<_> = app
        .documents
        .iter()
        .flat_map(|document| [document.window.0, document.panel.0])
        .collect();
    for _ in 0..2 {
        super::super::context_menu::execute(
            app.documents[1].window.0,
            &app.documents[1].state,
            EXIT_APPLICATION,
        )?;
    }
    if app.close_requested_with(|_| exit::Choice::Discard)
        || !app.exit_requested.get()
        || app.documents.len() != 2
    {
        return Err(Error::new(
            E_FAIL,
            "Application exit destroyed a modal document or lost request",
        ));
    }
    drop(modal);
    if !app.close_requested_with(|_| exit::Choice::Discard) || app.exit_requested.get() {
        return Err(Error::new(
            E_FAIL,
            "Deferred application exit did not resume",
        ));
    }
    if !app.documents.is_empty()
        || app.error.is_some()
        || !app.capture_windows.borrow().is_empty()
        || !app.workspace.units().is_empty()
        || !app.workspace.groups().is_empty()
        || !app.workspace.links().is_empty()
        || windows.iter().any(|hwnd| IsWindow(Some(*hwnd)).as_bool())
        || app.executions.active(&request_unit).is_some()
        || app.executions.preview_bytes() != 0
    {
        return Err(Error::new(
            E_FAIL,
            "Application did not release closed document owners",
        ));
    }
    println!("native-application-smoke: command=true coalesced=true close-initial=true reopen=true collected=true");
    println!("native-application-exit-smoke: all-windows=true modal-deferred=true coalesced=true collected=true");
    println!("native-workspace-identity-smoke: unique=true edit-stable=true closed-unresolved=true reopen-distinct=true");
    println!("native-workspace-registry-smoke: import-synced=true close-cascades-links=true exit-cleared=true");
    println!("native-workspace-groups-smoke: close-clears-membership=true remaining-member-preserved=true exit-cleared=true");
    println!("native-execution-owner-smoke: close-retires=true stale-rejected=true unrelated-preserved=true exit-cleared=true network=false");
    Ok(())
}
