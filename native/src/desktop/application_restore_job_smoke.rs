//! 真 worker 到隐藏发布的完整链；不访问文件/真实目录服务或注册系统热键。
use super::*;
const EMPTY: &[u8] = br#"{"stickers":[],"links":[],"extension":{"retained":true}}"#;

pub(in crate::desktop::application::restore_batch) unsafe fn verify(
    instance: HINSTANCE,
) -> Result<()> {
    super::context_smoke::verify(instance)?;
    super::frozen::smoke::run(instance)?;
    let first = DocumentWindow::create(instance, State::new(None, false)?, false)?;
    let mut app = Application::new(instance, first);
    app.catalog.borrow_mut().loaded = Some(super::super::smoke::catalog()?);
    let space = crate::session_placement::Space {
        origin: (-1920, -200),
        scale: 1.5,
    };
    let original = app.documents[0].window.0;
    let source = app.restored_session.as_ref().unwrap().original.clone();
    app.request_restore(EMPTY.to_vec(), None, space)?;
    if app.request_restore(EMPTY.to_vec(), None, space).is_ok() {
        return Err(Error::new(
            E_FAIL,
            "Restore coordinator admitted concurrent jobs",
        ));
    }
    app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
    if settle(&mut app).is_ok()
        || !IsWindow(Some(original)).as_bool()
        || app
            .restored_session
            .as_ref()
            .is_none_or(|record| !record.original.same_storage(&source))
    {
        return Err(Error::new(
            E_FAIL,
            "Async restoration overwrote edited source workspace",
        ));
    }
    app.request_restore(EMPTY.to_vec(), None, space)?;
    app.cancel_restore();
    settle(&mut app)?;
    if !IsWindow(Some(original)).as_bool()
        || app
            .restored_session
            .as_ref()
            .is_none_or(|record| !record.original.same_storage(&source))
    {
        return Err(Error::new(
            E_FAIL,
            "Cancelled restoration published candidates",
        ));
    }
    app.request_restore(b"invalid-json".to_vec(), None, space)?;
    if settle(&mut app).is_ok() || !IsWindow(Some(original)).as_bool() {
        return Err(Error::new(E_FAIL, "Invalid restoration damaged workspace"));
    }
    app.request_restore(EMPTY.to_vec(), None, space)?;
    settle(&mut app)?;
    if IsWindow(Some(original)).as_bool()
        || app.documents.len() != 1
        || app
            .restored_session
            .as_ref()
            .is_none_or(|record| record.original.as_slice() != EMPTY)
        || IsWindowVisible(app.documents[0].window.0).as_bool()
    {
        return Err(Error::new(
            E_FAIL,
            "Async restoration failed to publish retained empty session",
        ));
    }
    verify_offline(&mut app, space)?;
    crate::desktop::application::restore_rebind::smoke::verify(&mut app)?;
    crate::desktop::application::save_snapshot::smoke::verify(&mut app)?;
    println!("native-restored-job-smoke: worker=true stale-rejected=true cancel=true publish=true offline=true");
    Ok(())
}

unsafe fn verify_offline(
    app: &mut Application,
    space: crate::session_placement::Space,
) -> Result<()> {
    const SESSION: &[u8] = br#"{"stickers":[{"id":"offline-art","type":"art","artId":"unavailable","x":0,"y":0,"w":20,"h":20,"params":{"opaque":"retained"}},{"id":"offline-sticker","x":30,"y":0,"w":20,"h":20}],"links":[]}"#;
    let old = app.documents[0].window.0;
    app.catalog.borrow_mut().loaded = None;
    app.request_restore(SESSION.to_vec(), None, space)?;
    app.catalog.borrow_mut().loaded = Some(super::super::smoke::catalog()?);
    if settle(app).is_ok() || !IsWindow(Some(old)).as_bool() {
        return Err(Error::new(
            E_FAIL,
            "Offline restoration accepted a newly appeared catalog",
        ));
    }
    app.catalog.borrow_mut().loaded = None;
    app.request_restore(SESSION.to_vec(), None, space)?;
    settle(app)?;
    if app.documents.len() != 2
        || IsWindow(Some(old)).as_bool()
        || app.catalog.borrow().loaded.is_some()
        || app
            .restored_session
            .as_ref()
            .is_none_or(|record| record.original.as_slice() != SESSION)
        || app.documents.iter().any(|doc| {
            doc.state.selected_art.borrow().is_some()
                || doc.state.execution_results.borrow().active().is_some()
        })
    {
        return Err(Error::new(
            E_FAIL,
            "Offline restoration invented executable bindings or lost metadata",
        ));
    }
    for doc in &app.documents {
        let expected_art = doc.state.unit_id.as_str() == "offline-art";
        if doc.state.is_art() != expected_art {
            return Err(Error::new(E_FAIL, "Offline Art type degraded to sticker"));
        }
        if expected_art && crate::desktop::display_edit::prepare_content(&doc.state) {
            return Err(Error::new(
                E_FAIL,
                "Unbound Art admitted sticker-only content editing",
            ));
        }
    }
    Ok(())
}

pub(super) unsafe fn settle(app: &mut Application) -> Result<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while app.restore_job.is_some() {
        app.process_restore(false)?;
        if std::time::Instant::now() >= deadline {
            app.cancel_restore();
            return Err(Error::new(E_FAIL, "Synthetic restoration worker timed out"));
        }
        // 等待仅用于 smoke；生产路径由完成通知唤醒，没有轮询定时器。
        crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1));
    }
    Ok(())
}
