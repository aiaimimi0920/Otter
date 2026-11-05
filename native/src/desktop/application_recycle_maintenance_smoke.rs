//! 加载后/配置更新裁剪的真实 worker、原文发布和保存；没有磁盘删除或图片解码。
use super::*;
use crate::session_library::Kind;
fn settings(maximum: usize) -> std::sync::Arc<crate::settings_source::Source> {
    crate::settings_source::Source::new(
        "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-prune-settings-fixture.json".into(),
        serde_json::to_vec(&serde_json::json!({"cache":{"recycleBinMaxEntries":maximum}})).unwrap(),
    )
    .unwrap()
}
unsafe fn settle(app: &mut Application) -> Result<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        app.pending_windows(false)?;
        if app.restore_job.is_none() && app.save_job.is_none() && !app.recycle_maintenance_pending {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            return Err(Error::new(E_FAIL, "Recycle maintenance timeout"));
        }
        crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1));
    }
}
pub(in crate::desktop::application) unsafe fn run(instance: HINSTANCE) -> Result<()> {
    let first = DocumentWindow::create(instance, State::new(None, false)?, false)?;
    let mut app = Application::new(instance, first);
    app.install_startup_settings(settings(2))?;
    let entries: Vec<_> = (0..20)
        .map(|i| {
            serde_json::json!({"entryId":format!("e-{i}"),
        "sourceStickerId":"a","createdAt":"legacy","snapshot":{"id":"a","x":0,"y":0,"w":32,"h":32}})
        })
        .collect();
    let bytes = serde_json::to_vec(
        &serde_json::json!({"stickers":[],"links":[],"documentSchemaVersion":1,
        "documentRevision":9,"recycleBin":entries,"referenceLibrary":[],"extension":{"keep":true}}),
    )
    .unwrap();
    app.request_restore(
        bytes.clone(),
        None,
        crate::session_placement::Space {
            origin: (0, 0),
            scale: 1.0,
        },
    )?;
    settle(&mut app)?;
    let original = &app.restored_session.as_ref().unwrap().original;
    let library = crate::session_library::decode(original, Kind::RecycleBin).unwrap();
    if library.entries().len() != 2
        || library.entries()[0].entry_id != "e-18"
        || crate::session_document::decode(original).unwrap().revision != 9
    {
        return Err(Error::new(
            E_FAIL,
            "Loaded session was not pruned with startup policy",
        ));
    }
    let hwnd = app.documents[0].window.0;
    let path = save_publish::smoke_destination()?;
    reference::smoke::save(&mut app, Some(path.clone()))?;
    app.install_startup_settings(settings(1))?;
    settle(&mut app)?;
    if app
        .restored_session
        .as_ref()
        .unwrap()
        .disk_original
        .is_none()
    {
        return Err(Error::new(
            E_FAIL,
            "Maintenance lost committed disk baseline",
        ));
    }
    reference::smoke::save(&mut app, None)?;
    let before = app.restored_session.as_ref().unwrap().original.clone();
    app.install_startup_settings(settings(1))?;
    settle(&mut app)?;
    if !app
        .restored_session
        .as_ref()
        .unwrap()
        .original
        .same_storage(&before)
        || app.save_status() != save_state::Status::Unchanged
    {
        return Err(Error::new(
            E_FAIL,
            "No-op pruning dirtied or replaced session",
        ));
    }
    let mut reset: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    reset["documentRevision"] = crate::session_document::decode(&before)
        .unwrap()
        .revision
        .into();
    let reset = serde_json::to_vec(&reset).unwrap();
    for mode in 0..3 {
        app.restored_session
            .as_mut()
            .unwrap()
            .edit_original(reset.clone().into());
        let source = app.restored_session.as_ref().unwrap().original.clone();
        app.schedule_recycle_maintenance();
        app.start_recycle_maintenance()?;
        if mode == 0 {
            app.cancel_save_candidate();
        }
        if mode == 1 {
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.4;
        }
        if mode == 2 {
            app.recycle_policy = settings(3).document().recycle_policy().unwrap();
        }
        let result = settle(&mut app);
        app.documents[0].state.canvas.borrow_mut().normal_opacity = 1.0;
        if result.is_ok()
            || !app
                .restored_session
                .as_ref()
                .unwrap()
                .original
                .same_storage(&source)
            || !IsWindow(Some(hwnd)).as_bool()
        {
            return Err(Error::new(
                E_FAIL,
                "Cancelled/stale policy pruning changed session",
            ));
        }
    }
    app.install_startup_settings(settings(1))?;
    settle(&mut app)?;
    reference::smoke::save(&mut app, None)?;
    let disk = std::fs::read(path).map_err(|error| Error::new(E_FAIL, error.to_string()))?;
    if crate::session_library::decode(&disk, Kind::RecycleBin)
        .unwrap()
        .entries()
        .len()
        != 1
        || !crate::session_document::decode(&disk)
            .unwrap()
            .windows
            .is_empty()
    {
        return Err(Error::new(E_FAIL, "Pruned session disk roundtrip mismatch"));
    }
    println!("native-recycle-maintenance-smoke: load=true settings=true noop-clean=true cancel=true stale=true policy-stale=true save=true windows-preserved=true");
    Ok(())
}
