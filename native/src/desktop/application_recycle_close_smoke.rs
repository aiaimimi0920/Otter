//! 生产菜单到真实关闭/最后空宿主/磁盘往返；失败场景不得发布或丢失 HWND。
use super::*;
use crate::session_library::Kind;
unsafe fn request(app: &mut Application) -> Result<()> {
    app.sync_frozen_menu();
    let doc = &app.documents[0];
    crate::desktop::context_menu::execute(
        doc.window.0,
        &doc.state,
        crate::desktop::reference_ui::RECYCLE,
    )?;
    app.process_reference(false)
}
unsafe fn ready(app: &Application) -> Result<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !app.save_job.as_ref().is_some_and(|(_, job)| job.is_ready()) {
        if std::time::Instant::now() >= deadline {
            return Err(Error::new(E_FAIL, "Recycle close smoke timeout"));
        }
        crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1));
    }
    Ok(())
}
pub(in crate::desktop::application) unsafe fn run(instance: HINSTANCE) -> Result<()> {
    let image = crate::desktop::image::Image::from_encoded(
        crate::desktop::application::smoke::image_fixture::PNG,
    )?;
    let first = DocumentWindow::create(instance, State::new(Some(image.clone()), false)?, false)?;
    let mut app = Application::new(instance, first);
    app.add_state(State::new(Some(image), false)?, false)?;
    let path = save_publish::smoke_destination()?;
    reference::smoke::save(&mut app, Some(path.clone()))?;
    let source = app.restored_session.as_ref().unwrap().original.clone();
    let target = app.documents[0].window.0;
    let other = app.documents[1].window.0;
    for mode in 0..4 {
        request(&mut app)?;
        ready(&app)?;
        if mode == 0 {
            app.cancel_save_candidate();
        }
        if mode == 1 {
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.4;
        }
        if mode == 3 {
            *app.documents[0].state.live_request.borrow_mut() =
                Some(crate::desktop::capture_input::live::Request::Region(RECT {
                    left: 0,
                    top: 0,
                    right: 1,
                    bottom: 1,
                }));
        }
        let result = if mode == 2 {
            app.poll_recycle_with(false, |_| Err(Error::new(E_FAIL, "Injected close failure")))
        } else {
            app.poll_recycle(false)
        };
        app.documents[0].state.canvas.borrow_mut().normal_opacity = 1.0;
        app.documents[0].state.live_request.borrow_mut().take();
        if result.is_ok()
            || app.save_job.is_some()
            || app.documents.len() != 2
            || !IsWindow(Some(target)).as_bool()
            || !IsWindow(Some(other)).as_bool()
            || !app
                .restored_session
                .as_ref()
                .unwrap()
                .original
                .same_storage(&source)
        {
            return Err(Error::new(
                E_FAIL,
                "Failed recycle close published or destroyed window",
            ));
        }
    }
    for remaining in [1, 0] {
        let target = app.documents[0].window.0;
        let unit = app.documents[0].state.unit_id.clone();
        let history = app.executions.begin(unit.clone(), unit.clone()).unwrap();
        if !app.executions.finish(&unit, &history) {
            return Err(Error::new(
                E_FAIL,
                "Cannot prepare inactive execution history",
            ));
        }
        if remaining == 0 {
            verify_last_failure(&mut app)?;
        }
        request(&mut app)?;
        ready(&app)?;
        app.pending_windows(false)?;
        if IsWindow(Some(target)).as_bool()
            || app.executions.next_generation(&unit) != Ok(1)
            || app.workspace.contains(&unit)
            || app.documents.len() != 1
            || app.save_job.is_some()
            || (remaining == 1 && !IsWindow(Some(other)).as_bool())
        {
            return Err(Error::new(
                E_FAIL,
                "Recycle close did not retire target owner",
            ));
        }
        reference::smoke::save(&mut app, None)?;
        let bytes = std::fs::read(&path).map_err(|error| Error::new(E_FAIL, error.to_string()))?;
        let document = crate::session_document::decode(&bytes).unwrap();
        if document.windows.len() != remaining
            || document.library(Kind::RecycleBin).unwrap().entries().len() != 2 - remaining
            || app.capture_windows.borrow().len() != 2
        {
            return Err(Error::new(
                E_FAIL,
                "Recycle close save or host omission failed",
            ));
        }
    }
    let record = app.restored_session.as_ref().unwrap();
    if record.transient_host.as_ref() != Some(&app.documents[0].state.unit_id)
        || !app.documents[0].is_alive()
    {
        return Err(Error::new(E_FAIL, "Last recycle close lost empty host"));
    }
    // 最后一张关闭后，空宿主仍可从同一回收站恢复；不能只证明进程未退出。
    let source = record.original.clone();
    let id = crate::session_library::decode(&source, Kind::RecycleBin)
        .unwrap()
        .entries()[0]
        .entry_id
        .to_string();
    app.request_frozen(source, Kind::RecycleBin, id, POINT { x: 0, y: 0 })?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while app.restore_job.is_some() {
        app.pending_windows(false)?;
        if std::time::Instant::now() >= deadline {
            return Err(Error::new(E_FAIL, "Recycle restore timeout"));
        }
        crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1));
    }
    if app.documents.len() != 2 {
        return Err(Error::new(E_FAIL, "Recycle restore from empty host failed"));
    }
    println!("native-recycle-close-smoke: menu=true cancel=true stale=true close-failure=true retired=true last-host=true save=true restore=true");
    Ok(())
}

unsafe fn verify_last_failure(app: &mut Application) -> Result<()> {
    let source = app.restored_session.as_ref().unwrap().original.clone();
    let hwnd = app.documents[0].window.0;
    let preferred = app.hotkey_enabled.get();
    // 不占用用户 Ctrl+1：在真实宿主准备/失败路径注入注册后端，核验实际转交顺序。
    app.documents[0].state.global_capture.set(true);
    app.hotkey_enabled.set(true);
    let result = (|| -> Result<()> {
        request(app)?;
        ready(app)?;
        let mut calls = Vec::new();
        let result = app.poll_recycle_using(
            false,
            |_| Err(Error::new(E_FAIL, "Injected last close failure")),
            |doc, enabled| {
                calls.push((doc.window.0, enabled));
                doc.state.global_capture.set(enabled);
                doc.state.hotkey_enabled.set(enabled);
                Ok(())
            },
        );
        if result.is_ok()
            || calls.len() != 4
            || calls[0] != (hwnd, false)
            || calls[3] != (hwnd, true)
            || calls[1].0 == hwnd
            || calls[1].0 != calls[2].0
            || !calls[1].1
            || calls[2].1
            || IsWindow(Some(calls[1].0)).as_bool()
            || !app.documents[0].is_alive()
            || !app.documents[0].state.global_capture.get()
            || !app.hotkey_enabled.get()
            || !app
                .restored_session
                .as_ref()
                .unwrap()
                .original
                .same_storage(&source)
        {
            return Err(Error::new(
                E_FAIL,
                "Last host failure leaked successor or lost hotkey rollback",
            ));
        }
        Ok(())
    })();
    app.documents[0].state.global_capture.set(false);
    app.hotkey_enabled.set(preferred);
    result
}
