//! 真实收藏菜单命令到后台快照、库提交及取消；不写用户文件。
use super::*;
use crate::session_library::Kind;

unsafe fn request(app: &Application, remove: bool) -> Result<()> {
    let doc = &app.documents[0];
    crate::desktop::context_menu::execute(
        doc.window.0,
        &doc.state,
        if remove {
            crate::desktop::reference_ui::REMOVE
        } else {
            crate::desktop::reference_ui::SET
        },
    )
}
unsafe fn settle(app: &mut Application) -> Result<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        app.process_reference(false)?;
        if app.save_job.is_none() {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            app.cancel_save_candidate();
            return Err(Error::new(E_FAIL, "Reference smoke timeout"));
        }
        crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1));
    }
}
pub(in crate::desktop::application) unsafe fn save(
    app: &mut Application,
    destination: Option<std::path::PathBuf>,
) -> Result<()> {
    if let Some(path) = destination {
        app.request_save_new(path)?;
    } else {
        app.request_save_existing()?;
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        if let Some(prepared) = app.take_save_candidate()? {
            prepared.commit_ready(app, &std::sync::atomic::AtomicBool::new(false))?;
            app.sync_frozen_menu();
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            app.cancel_save_candidate();
            return Err(Error::new(E_FAIL, "Reference save timeout"));
        }
        crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1));
    }
}
pub(in crate::desktop::application) unsafe fn run(instance: HINSTANCE) -> Result<()> {
    recycle_smoke::run(instance)?;
    let image =
        crate::desktop::image::Image::from_encoded(super::super::super::smoke::image_fixture::PNG)?;
    let first = DocumentWindow::create(instance, State::new(Some(image), false)?, false)?;
    let mut app = Application::new(instance, first);
    let hwnd = app.documents[0].window.0;
    let path = save_publish::smoke_destination()?;
    save(&mut app, Some(path.clone()))?;
    app.sync_frozen_menu();
    let original = app.restored_session.as_ref().unwrap().original.clone();
    for mode in 0..3 {
        request(&app, false)?;
        app.process_reference(false)?;
        if !app
            .save_job
            .as_ref()
            .is_some_and(|(_, job)| job.is_library_update())
            || app.request_save_candidate().is_ok()
            || app.take_save_candidate().is_ok()
        {
            return Err(Error::new(
                E_FAIL,
                "Reference worker did not own the save slot",
            ));
        }
        let before = app.documents[0].state.canvas.borrow().normal_opacity;
        if mode == 0 {
            app.cancel_save_candidate();
        }
        if mode == 1 {
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.4;
        }
        let result = settle(&mut app);
        app.documents[0].state.canvas.borrow_mut().normal_opacity = before;
        if mode < 2 {
            if result.is_ok()
                || !app
                    .restored_session
                    .as_ref()
                    .unwrap()
                    .original
                    .same_storage(&original)
            {
                return Err(Error::new(
                    E_FAIL,
                    "Cancelled or stale reference was committed",
                ));
            }
        } else {
            result?;
        }
    }
    let first = app.restored_session.as_ref().unwrap().original.clone();
    let library = crate::session_library::decode(&first, Kind::ReferenceLibrary)
        .map_err(|error| Error::new(E_FAIL, error))?;
    if library.entries().len() != 1
        || library.entries()[0]
            .window
            .src
            .as_ref()
            .is_none_or(|src| !src.starts_with("data:image/png"))
    {
        return Err(Error::new(
            E_FAIL,
            "Reference did not capture the current image",
        ));
    }
    save(&mut app, None)?;
    if app
        .request_frozen(
            first.clone(),
            Kind::ReferenceLibrary,
            library.entries()[0].entry_id.to_string(),
            POINT { x: 20, y: 20 },
        )
        .is_ok()
    {
        return Err(Error::new(
            E_FAIL,
            "Saved reference accepted stale menu source",
        ));
    }
    // 上次保存换了来源 owner，旧菜单必须被拒绝；用当前库来源再复制。
    let current = app.restored_session.as_ref().unwrap().original.clone();
    let id = crate::session_library::decode(&current, Kind::ReferenceLibrary)
        .unwrap()
        .entries()[0]
        .entry_id
        .to_string();
    app.request_frozen(current, Kind::ReferenceLibrary, id, POINT { x: 20, y: 20 })?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while app.restore_job.is_some() {
        app.pending_windows(false)?;
        if std::time::Instant::now() >= deadline {
            app.cancel_restore();
            return Err(Error::new(E_FAIL, "Reference copy timeout"));
        }
        crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1));
    }
    if app.documents.len() != 2 || app.documents[1].state.canvas.borrow().image.is_none() {
        return Err(Error::new(E_FAIL, "Created reference could not be copied"));
    }
    save(&mut app, None)?;
    let first_id = library.entries()[0].entry_id.to_string();
    app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.65;
    request(&app, false)?;
    settle(&mut app)?;
    let updated = app.restored_session.as_ref().unwrap().original.clone();
    let library = crate::session_library::decode(&updated, Kind::ReferenceLibrary)
        .map_err(|error| Error::new(E_FAIL, error))?;
    let snapshot: serde_json::Value =
        serde_json::from_str(library.entries()[0].snapshot_json()).unwrap();
    if library.entries().len() != 1
        || library.entries()[0].entry_id == first_id
        || (snapshot["opacityNormal"].as_f64().unwrap_or(0.0) - 0.65).abs() > 0.001
    {
        return Err(Error::new(
            E_FAIL,
            "Reference replacement lost current editing state",
        ));
    }
    request(&app, true)?;
    settle(&mut app)?;
    let record = app.restored_session.as_ref().unwrap();
    if !crate::session_library::decode(&record.original, Kind::ReferenceLibrary)
        .unwrap()
        .entries()
        .is_empty()
        || crate::session_document::decode(&record.original)
            .unwrap()
            .revision
            != 3
        || app.documents.len() != 2
        || !IsWindow(Some(hwnd)).as_bool()
        || app.save_status() != save_state::Status::Unsaved
    {
        return Err(Error::new(
            E_FAIL,
            "Reference cancellation changed window or disk baseline",
        ));
    }
    save(&mut app, None)?;
    let bytes = std::fs::read(&path).map_err(|error| Error::new(E_FAIL, error.to_string()))?;
    let disk = crate::session_document::decode(&bytes).unwrap();
    if disk.revision != 4
        || disk.windows.len() != 2
        || !disk
            .library(Kind::ReferenceLibrary)
            .unwrap()
            .entries()
            .is_empty()
        || app
            .restored_session
            .as_ref()
            .unwrap()
            .disk_original
            .is_some()
    {
        return Err(Error::new(
            E_FAIL,
            "Reference disk roundtrip lost baseline or library changes",
        ));
    }
    // 仅修改本测试独占的 linshi 文件：竞争者改变即使只有空白也不能被覆盖。
    let mut external = bytes.clone();
    external.push(b' ');
    std::fs::write(&path, &external).map_err(|error| Error::new(E_FAIL, error.to_string()))?;
    if save(&mut app, None).is_ok() || std::fs::read(&path).ok().as_ref() != Some(&external) {
        return Err(Error::new(
            E_FAIL,
            "Reference save bypassed external file conflict",
        ));
    }
    println!("native-reference-disk-smoke: create=true update=true copy=true remove=true revision=4 external-conflict=true");
    println!("native-reference-write-smoke: menu=true current-snapshot=true worker=true replace=true remove=true stale=true cancel=true revision-preserved=true");
    Ok(())
}

#[path = "application_recycle_prepare_smoke.rs"]
mod recycle_smoke;
