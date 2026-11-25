//! Asynchronous application entry point used by menu commands: real worker, complete source credentials, and hidden append, without touching user sessions.
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
        active: app.workspace.units().to_vec(),
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

    app.request_frozen(source.clone(), Kind::RecycleBin, "entry".into(), point)?;
    let replacement: Source = source.as_ref().to_vec().into();
    app.restored_session.as_mut().unwrap().original = replacement.clone();
    if settle(&mut app).is_ok() {
        return Err(Error::new(
            E_FAIL,
            "Frozen worker accepted a different source owner",
        ));
    }
    preserved(&app, &replacement, original)?;
    app.restored_session.as_mut().unwrap().original = source.clone();

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
    // Convert physical mouse coordinates to logical coordinates, add 50, then restore through the same save space; do not simply add 50 physical pixels.
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
    // The first append replaced the source owner; the old menu command cannot consume the old snapshot again.
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
