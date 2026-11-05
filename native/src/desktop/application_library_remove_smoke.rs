//! 两库第二页“删除此条”经生产队列/worker/提交，并在 linshi 覆盖保存。
use super::*;
use crate::session_library::Kind;
unsafe fn settle(app: &mut Application) -> Result<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        app.pending_windows(false)?;
        if app.save_job.is_none() {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            app.cancel_save_candidate();
            return Err(Error::new(E_FAIL, "Frozen removal timeout"));
        }
        crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1));
    }
}
pub(in crate::desktop::application) unsafe fn run(instance: HINSTANCE) -> Result<()> {
    let first = DocumentWindow::create(instance, State::new(None, false)?, false)?;
    let mut app = Application::new(instance, first);
    let hwnd = app.documents[0].window.0;
    let entries: Vec<_> = (0..65).map(|i| serde_json::json!({"entryId":format!("entry-{i}"),"sourceStickerId":format!("source-{i}"),
        "createdAt":"legacy", "snapshot":{"id":format!("source-{i}"),"x":0,"y":0,"w":32,"h":32}})).collect();
    app.restored_session.as_mut().unwrap().original = serde_json::to_vec(&serde_json::json!({"stickers":[],"links":[],"referenceLibrary":entries,"recycleBin":entries})).unwrap().into();
    let path = save_publish::smoke_destination()?;
    reference::smoke::save(&mut app, Some(path.clone()))?;
    for library in 0..2 {
        for mode in 0..3 {
            let source = app.restored_session.as_ref().unwrap().original.clone();
            let choice = crate::desktop::popup_menu::smoke_frozen_action(
                hwnd,
                source.clone(),
                library,
                true,
            )?;
            crate::desktop::frozen_library_ui::enqueue(
                hwnd,
                &app.documents[0].state,
                choice,
                POINT { x: 0, y: 0 },
            )?;
            app.pending_windows(false)?;
            if !app
                .save_job
                .as_ref()
                .is_some_and(|(_, job)| job.is_library_update())
                || app.take_save_candidate().is_ok()
            {
                return Err(Error::new(
                    E_FAIL,
                    "Frozen removal was confused with restore or disk save",
                ));
            }
            let opacity = app.documents[0].state.canvas.borrow().normal_opacity;
            if mode == 0 {
                app.cancel_save_candidate();
            }
            if mode == 1 {
                app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.4;
            }
            let result = settle(&mut app);
            app.documents[0].state.canvas.borrow_mut().normal_opacity = opacity;
            if mode < 2 {
                if result.is_ok()
                    || !app
                        .restored_session
                        .as_ref()
                        .unwrap()
                        .original
                        .same_storage(&source)
                {
                    return Err(Error::new(
                        E_FAIL,
                        "Cancelled/stale frozen removal changed library",
                    ));
                }
            } else {
                result?;
            }
        }
        let source = &app.restored_session.as_ref().unwrap().original;
        let kind = [Kind::RecycleBin, Kind::ReferenceLibrary][library];
        let selected = crate::session_library::decode(source, kind).unwrap();
        if selected.entries().len() != 64
            || selected.find("entry-64").is_some()
            || app.documents.len() != 1
            || !IsWindow(Some(hwnd)).as_bool()
        {
            return Err(Error::new(
                E_FAIL,
                "Frozen removal changed wrong entry or active window",
            ));
        }
        reference::smoke::save(&mut app, None)?;
    }
    let bytes = std::fs::read(path).map_err(|error| Error::new(E_FAIL, error.to_string()))?;
    let disk = crate::session_document::decode(&bytes).unwrap();
    if disk.revision != 3
        || !disk.windows.is_empty()
        || disk.library(Kind::RecycleBin).unwrap().entries().len() != 64
        || disk
            .library(Kind::ReferenceLibrary)
            .unwrap()
            .entries()
            .len()
            != 64
    {
        return Err(Error::new(E_FAIL, "Frozen removal disk roundtrip mismatch"));
    }
    println!("native-frozen-remove-smoke: menu=true paged=true both-libraries=true cancel=true stale=true windows-preserved=true disk-revision=3");
    Ok(())
}
