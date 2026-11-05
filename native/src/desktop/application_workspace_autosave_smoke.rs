//! 真实产品的菜单→共享设置→保存线程→原子落盘→重新恢复；只用 linshi 合成会话。
use super::*;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

pub(super) struct File(pub PathBuf);
impl Drop for File {
    fn drop(&mut self) {
        // smoke_destination 创建的独占 GUID 路径，仅删除本次 linshi 文件。
        let _ = std::fs::remove_file(&self.0);
    }
}
pub(super) unsafe fn fixture(instance: HINSTANCE) -> Result<(Application, File)> {
    let image = crate::desktop::image::Image::from_encoded(
        crate::desktop::application::smoke::image_fixture::PNG,
    )?;
    let first = DocumentWindow::create(instance, State::new(Some(image), false)?, false)?;
    let mut app = Application::new(instance, first);
    let record = app.restored_session.as_mut().unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&record.original)
        .map_err(|error| Error::new(E_FAIL, error.to_string()))?;
    value["extension"] = serde_json::json!({"autosave-test":"preserved"});
    record.edit_original(
        serde_json::to_vec(&value)
            .map_err(|error| Error::new(E_FAIL, error.to_string()))?
            .into(),
    );
    let path = File(save_publish::smoke_destination()?);
    crate::desktop::application::frozen_menu::reference::smoke::save(
        &mut app,
        Some(path.0.clone()),
    )?;
    Ok((app, path))
}
pub(super) unsafe fn toggle(app: &Application, index: usize) -> Result<()> {
    let doc = &app.documents[index];
    crate::desktop::context_menu::execute(doc.window.0, &doc.state, commands::AUTO_WORKSPACE)
}
pub(super) unsafe fn settle(app: &mut Application) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let mut error = None;
        app.pending_workspace_save_with(|_, _, message, warning| {
            if warning {
                error = Some(message.to_string());
            }
        });
        if let Some(error) = error {
            return Err(Error::new(E_FAIL, error));
        }
        if app.save_job.is_none() && app.workspace_autosave_pending.is_none() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            app.cancel_save_candidate();
            return Err(Error::new(E_FAIL, "Workspace autosave smoke deadline"));
        }
        crate::desktop::smoke_timing::sleep(Duration::from_millis(1));
    }
}
pub(in crate::desktop::application) unsafe fn run(instance: HINSTANCE) -> Result<()> {
    let (mut app, path) = fixture(instance)?;
    app.add_window(false)?;
    let windows: Vec<_> = app.documents.iter().map(|doc| doc.window.0).collect();
    toggle(&app, 1)?;
    let menu = CreatePopupMenu()?;
    let appended = commands::append_menu(menu, &app.documents[0].state);
    let checked =
        GetMenuState(menu, commands::AUTO_WORKSPACE as u32, MF_BYCOMMAND) & MF_CHECKED.0 != 0;
    let destroyed = DestroyMenu(menu);
    appended?;
    destroyed?;
    if !checked
        || !std::rc::Rc::ptr_eq(
            &app.workspace_autosave,
            &app.documents[1].state.workspace_autosave,
        )
    {
        return Err(Error::new(
            E_FAIL,
            "Autosave menu did not share current workspace control",
        ));
    }
    let source = std::fs::read(&path.0).map_err(|e| Error::new(E_FAIL, e.to_string()))?;
    let before_revision = crate::session_document::decode(&source).unwrap().revision;
    let flag = app.documents[0].state.file_dialog.clone();
    let dialog = crate::desktop::file_dialog::Guard::begin(&flag).unwrap();
    app.pending_workspace_save_with(|_, _, _, _| unreachable!("modal must defer"));
    if app.save_job.is_some() {
        return Err(Error::new(E_FAIL, "Modal autosave started"));
    }
    drop(dialog);
    app.documents[0]
        .state
        .canvas
        .borrow_mut()
        .editor
        .begin(
            crate::editor::Tool::Line,
            crate::viewport::Point { x: 0.0, y: 0.0 },
        )
        .map_err(|_| Error::new(E_FAIL, "Autosave draft fixture failed"))?;
    app.pending_workspace_save_with(|_, _, _, _| unreachable!("draft must defer"));
    if app.save_job.is_some() {
        return Err(Error::new(E_FAIL, "Draft autosave started"));
    }
    app.documents[0].state.canvas.borrow_mut().editor.cancel();
    app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.4;
    settle(&mut app)?;
    let disk = std::fs::read(&path.0).map_err(|e| Error::new(E_FAIL, e.to_string()))?;
    let loaded = crate::session_document::decode(&disk).unwrap();
    if loaded.revision != before_revision + 1
        || loaded.windows.len() != 2
        || app.save_status() != save_state::Status::Unchanged
        || windows.iter().any(|hwnd| !IsWindow(Some(*hwnd)).as_bool())
    {
        return Err(Error::new(
            E_FAIL,
            "Autosave did not commit current shared workspace",
        ));
    }
    let value: serde_json::Value = serde_json::from_slice(&disk).unwrap();
    if value["extension"]["autosave-test"] != "preserved"
        || app.restored_session.as_ref().unwrap().workflow.is_some()
    {
        return Err(Error::new(
            E_FAIL,
            "Autosave changed extensions or started workflow",
        ));
    }
    settle(&mut app)?;
    if std::fs::read(&path.0).map_err(|e| Error::new(E_FAIL, e.to_string()))? != disk {
        return Err(Error::new(E_FAIL, "Unchanged workspace saved repeatedly"));
    }
    app.request_restore_input(
        crate::desktop::session_job::Input::File(path.0.clone()),
        None,
        None,
    )?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.restore_job.is_some() || app.save_job.is_some() || app.recycle_maintenance_pending {
        app.pending_windows(false)?;
        if Instant::now() >= deadline {
            return Err(Error::new(E_FAIL, "Autosave restore deadline"));
        }
        crate::desktop::smoke_timing::sleep(Duration::from_millis(1));
    }
    if app.workspace_autosave.enabled()
        || app.documents.len() != 2
        || app.documents[0].state.canvas.borrow().normal_opacity != 0.4
    {
        return Err(Error::new(
            E_FAIL,
            "Autosave disk restoration or consent reset failed",
        ));
    }
    println!("native-workspace-autosave-smoke: menu=true shared=true worker=true unchanged-noop=true modal=true draft=true restored=true workflow=false");
    Ok(())
}
