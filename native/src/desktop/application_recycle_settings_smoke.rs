//! 菜单请求、模态事务、裁剪和配置保存/自动保存的真实 HWND 验证；对话框选择由夹具注入。
use super::*;
fn policy(count: usize) -> Policy {
    Policy {
        recycle_bin_max_entries: count,
        recycle_bin_retention_days: 0,
    }
}
unsafe fn queue(app: &Application) -> Result<()> {
    let doc = &app.documents[0];
    crate::desktop::context_menu::execute(
        doc.window.0,
        &doc.state,
        crate::desktop::reference_ui::CONFIGURE,
    )
}
unsafe fn edit(app: &mut Application, count: usize) -> Result<()> {
    app.sync_frozen_menu();
    queue(app)?;
    app.edit_recycle_settings_with(|app, _, _| {
        assert!(app.modals.blocked());
        Some(policy(count))
    })
}
unsafe fn settle(app: &mut Application, automatic: bool) -> Result<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        app.pending_windows(false)?;
        if automatic {
            app.pending_naming_save_auto_with(|_, _| panic!("Unexpected settings dialog"))?;
        } else {
            app.pending_naming_save_with(|_, _| panic!("Unexpected settings dialog"))?;
        }
        if app.save_job.is_none()
            && app.naming_save_job.is_none()
            && !app.recycle_maintenance_pending
        {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            return Err(Error::new(E_FAIL, "Recycle settings smoke timeout"));
        }
        crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1));
    }
}
pub(in crate::desktop::application) unsafe fn run(instance: HINSTANCE) -> Result<()> {
    let first = DocumentWindow::create(instance, State::new(None, false)?, false)?;
    let mut app = Application::new(instance, first);
    app.naming_document = std::sync::Arc::new(
        crate::app_settings_document::Document::decode(br#"{"cache":{"future":{"keep":true}}}"#)
            .unwrap(),
    );
    let source = app.naming_document.clone();
    queue(&app)?;
    queue(&app)?;
    app.edit_recycle_settings_with(|app, _, _| {
        assert!(app.modals.blocked());
        None
    })?;
    if app.frozen_library.borrow().configure_recycle.is_some()
        || app.recycle_policy != Policy::default()
    {
        return Err(Error::new(
            E_FAIL,
            "Recycle settings cancellation changed state",
        ));
    }
    if edit(&mut app, 501).is_ok()
        || app.recycle_policy != Policy::default()
        || !std::sync::Arc::ptr_eq(&source, &app.naming_document)
    {
        return Err(Error::new(
            E_FAIL,
            "Invalid recycle settings changed document baseline",
        ));
    }
    let entries: Vec<_> = (0..3)
        .map(|i| {
            serde_json::json!({"entryId":format!("e-{i}"),"sourceStickerId":"a",
        "createdAt":"legacy","snapshot":{"id":"a","x":0,"y":0,"w":32,"h":32}})
        })
        .collect();
    app.restored_session.as_mut().unwrap().original =
        serde_json::to_vec(&serde_json::json!({"stickers":[],"links":[],"recycleBin":entries}))
            .unwrap()
            .into();
    edit(&mut app, 1)?;
    settle(&mut app, false)?;
    if crate::session_library::decode(
        &app.restored_session.as_ref().unwrap().original,
        crate::session_library::Kind::RecycleBin,
    )
    .unwrap()
    .entries()
    .len()
        != 1
    {
        return Err(Error::new(
            E_FAIL,
            "Recycle settings did not trigger maintenance",
        ));
    }
    let path = save_publish::smoke_destination()?;
    let doc = &app.documents[0];
    crate::desktop::context_menu::execute(doc.window.0, &doc.state, SAVE_NAMING)?;
    app.pending_naming_save_with(|_, _| Ok(Some(path.clone())))?;
    settle(&mut app, false)?;
    let bytes = std::fs::read(&path).map_err(|error| Error::new(E_FAIL, error.to_string()))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    if value["cache"]["recycleBinMaxEntries"] != 1 || value["cache"]["future"]["keep"] != true {
        return Err(Error::new(E_FAIL, "Recycle policy save lost fields"));
    }
    edit(&mut app, 2)?;
    app.naming_autosave.set(true);
    if !app.naming_autosave_blocks_exit() {
        return Err(Error::new(
            E_FAIL,
            "Recycle-only edit did not enter autosave",
        ));
    }
    settle(&mut app, true)?;
    if app.naming_autosave_blocks_exit()
        || app.naming_document.recycle_policy().unwrap() != policy(2)
    {
        return Err(Error::new(
            E_FAIL,
            "Recycle autosave did not update baseline",
        ));
    }
    app.naming_autosave.set(false);
    let baseline = std::fs::read(&path).unwrap();
    let doc = &app.documents[0];
    crate::desktop::context_menu::execute(doc.window.0, &doc.state, UPDATE_NAMING)?;
    app.pending_naming_save_with(|_, _| panic!("Unexpected save chooser"))?;
    edit(&mut app, 3)?;
    if settle(&mut app, false).is_ok() || std::fs::read(&path).unwrap() != baseline {
        return Err(Error::new(
            E_FAIL,
            "Stale settings worker overwrote recycle edit",
        ));
    }
    // 先回收裁剪任务，随后验证模态期间源窗口关闭不会安装新偏好。
    settle(&mut app, false)?;
    queue(&app)?;
    let result = app.edit_recycle_settings_with(|_, hwnd, _| {
        DestroyWindow(hwnd).unwrap();
        Some(policy(4))
    });
    if result.is_ok() || app.recycle_policy != policy(3) {
        return Err(Error::new(
            E_FAIL,
            "Closed settings requester published draft",
        ));
    }
    println!("native-recycle-settings-smoke: menu=true cancel=true invalid=true maintenance=true save=true autosave=true stale=true owner-closed=true dialog-injected=true");
    Ok(())
}
