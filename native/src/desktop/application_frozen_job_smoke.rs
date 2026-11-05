//! 菜单命令将使用的异步应用入口：真 worker、完整来源凭证与隐藏追加，不触碰用户会话。
use super::super::smoke::settle;
use super::*;

pub(in crate::desktop::application::restore_batch::job) unsafe fn run(
    instance: HINSTANCE,
) -> Result<()> {
    let first = DocumentWindow::create(instance, State::new(None, false)?, false)?;
    let mut app = Application::new(instance, first);
    let original = app.documents[0].window.0;
    let space = crate::session_placement::Space {
        origin: (-1920, -200),
        scale: 1.5,
    };
    let point = POINT { x: -1800, y: -80 };
    let png = crate::loom_base64::encode_png_data_url(
        crate::desktop::application::smoke::image_fixture::PNG,
        &AtomicBool::new(false),
    )
    .map_err(|_| Error::new(E_FAIL, "Frozen worker fixture encoding"))?;
    let entry = serde_json::json!({"entryId":"entry","sourceStickerId":"old","createdAt":"legacy",
        "snapshot":{"id":"old","src":png.as_str(),"x":0,"y":0,"w":100,"h":80,"extension":{"keep":true}}});
    let source: Source = serde_json::to_vec(&serde_json::json!({"stickers":[],"links":[],
        "recycleBin":[entry.clone()],"referenceLibrary":[entry]}))
    .unwrap()
    .into();
    app.restored_session.as_mut().unwrap().original = source.clone();
    app.restored_session.as_mut().unwrap().space = Some(space);

    let bare = session_job::Input::Frozen(session_job::frozen::Input {
        source: source.clone(),
        kind: Kind::RecycleBin,
        entry_id: "entry".into(),
        unit: UnitId::parse("must-not-replace").unwrap(),
        mouse: (0.0, 0.0),
    });
    if app.request_restore_input(bare, None, space).is_ok() {
        return Err(Error::new(
            E_FAIL,
            "Bare frozen input entered workspace replacement mode",
        ));
    }
    preserved(&app, &source, original)?;

    app.request_frozen(source.clone(), Kind::RecycleBin, "entry".into(), point)?;
    if app
        .request_frozen(
            source.clone(),
            Kind::ReferenceLibrary,
            "entry".into(),
            point,
        )
        .is_ok()
        || app.capture_session_snapshot().is_ok()
    {
        return Err(Error::new(
            E_FAIL,
            "Frozen worker admitted overlapping task",
        ));
    }
    app.cancel_restore();
    settle(&mut app)?;
    preserved(&app, &source, original)?;

    app.request_frozen(source.clone(), Kind::RecycleBin, "entry".into(), point)?;
    app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
    if settle(&mut app).is_ok() {
        return Err(Error::new(E_FAIL, "Frozen worker accepted edited origin"));
    }
    app.documents[0].state.canvas.borrow_mut().normal_opacity = 1.0;
    preserved(&app, &source, original)?;

    app.request_frozen(source.clone(), Kind::RecycleBin, "entry".into(), point)?;
    app.restored_session.as_mut().unwrap().space = None;
    let stale = settle(&mut app).is_err();
    app.restored_session.as_mut().unwrap().space = Some(space);
    if !stale {
        return Err(Error::new(
            E_FAIL,
            "Frozen worker accepted missing coordinates",
        ));
    }
    preserved(&app, &source, original)?;

    app.request_frozen(source.clone(), Kind::RecycleBin, "missing".into(), point)?;
    if settle(&mut app).is_ok() {
        return Err(Error::new(E_FAIL, "Frozen worker invented an entry"));
    }
    preserved(&app, &source, original)?;

    let copied = app.request_frozen(
        source.clone(),
        Kind::ReferenceLibrary,
        "entry".into(),
        point,
    )?;
    settle(&mut app)?;
    if app.documents.len() != 2
        || app.source_window(&copied).is_none()
        || !IsWindow(Some(original)).as_bool()
    {
        return Err(Error::new(
            E_FAIL,
            "Frozen worker replaced workspace instead of appending",
        ));
    }
    let document = app.documents.last().unwrap();
    let mut rect = RECT::default();
    GetWindowRect(document.window.0, &mut rect)?;
    // 物理鼠标转换为逻辑坐标后 +50，再按同一保存空间还原；不是直接加 50 物理像素。
    if rect.left != point.x + 75
        || rect.top != point.y + 75
        || document.state.canvas.borrow().image.is_none()
        || IsWindowVisible(document.window.0).as_bool()
    {
        return Err(Error::new(
            E_FAIL,
            "Frozen worker lost image or coordinate context",
        ));
    }
    // 第一轮追加已替换来源 owner，旧菜单命令不能再次消费旧快照。
    if app
        .request_frozen(source, Kind::RecycleBin, "entry".into(), point)
        .is_ok()
    {
        return Err(Error::new(
            E_FAIL,
            "Frozen worker accepted stale menu source",
        ));
    }
    let source = app.restored_session.as_ref().unwrap().original.clone();
    let restored = app.request_frozen(source, Kind::RecycleBin, "entry".into(), point)?;
    settle(&mut app)?;
    let record = app.restored_session.as_ref().unwrap();
    let decoded = crate::session_document::decode(&record.original).unwrap();
    if app.documents.len() != 3
        || app.source_window(&restored).is_none()
        || !decoded
            .library(Kind::RecycleBin)
            .unwrap()
            .entries()
            .is_empty()
        || decoded
            .library(Kind::ReferenceLibrary)
            .unwrap()
            .entries()
            .len()
            != 1
        || app.save_status() != crate::desktop::application::save_state::Status::Unsaved
    {
        return Err(Error::new(
            E_FAIL,
            "Frozen worker consumption or save state mismatch",
        ));
    }
    let windows: Vec<_> = app.documents.iter().map(|doc| doc.window.0).collect();
    drop(app);
    if windows.iter().any(|hwnd| IsWindow(Some(*hwnd)).as_bool()) {
        return Err(Error::new(E_FAIL, "Frozen worker leaked a window"));
    }
    println!("native-frozen-worker-smoke: worker=true append=true cancel=true stale=true coordinates=true recycle=true reference=true cleanup=true");
    Ok(())
}

unsafe fn preserved(app: &Application, source: &Source, hwnd: HWND) -> Result<()> {
    if app.restore_job.is_some()
        || app.documents.len() != 1
        || !IsWindow(Some(hwnd)).as_bool()
        || !app
            .restored_session
            .as_ref()
            .unwrap()
            .original
            .same_storage(source)
    {
        Err(Error::new(
            E_FAIL,
            "Failed frozen worker changed the source workspace",
        ))
    } else {
        Ok(())
    }
}
