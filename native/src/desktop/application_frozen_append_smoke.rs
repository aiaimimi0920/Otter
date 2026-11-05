//! 独立隐藏 HWND 的冻结追加事务验证；真实图片资源、旧窗口与保存回读均走生产路径。
use super::*;
use crate::{
    desktop::{session_binding::Bound, session_candidate::VectorCandidate},
    workspace_id::UnitId,
};

unsafe fn prepare(app: &Application, kind: Kind, id: &str) -> Result<Append> {
    let cancel = AtomicBool::new(false);
    let source = app.restored_session.as_ref().unwrap().original.clone();
    let document = crate::session_document::decode(&source)
        .map_err(|error| Error::new(E_FAIL, format!("Fixture: {error:?}")))?;
    let copy = crate::session_library::candidate::prepare(
        &document,
        kind,
        "entry",
        UnitId::parse(id).unwrap(),
        (20.0, 30.0),
        &cancel,
    )
    .map_err(|error| Error::new(E_FAIL, error))?;
    let candidate =
        crate::desktop::session_candidate::prepare(copy.bytes.as_ref().to_vec(), None, &cancel)?;
    let bound = Bound::prepare_for(VectorCandidate::prepare(candidate, &cancel)?, None, &cancel)
        .map_err(|error| error.1)?;
    let states = bound
        .into_parts_for(None, &cancel)
        .ok()
        .unwrap()
        .prepare_states(&cancel)?;
    let space = app.restored_session.as_ref().unwrap().space.unwrap();
    let windows = app.prepare_restored_batch(states, &cancel, |placement| {
        space
            .physical(placement.rect)
            .map_err(|_| Error::new(E_FAIL, "Frozen fixture frame"))
    })?;
    app.prepare_frozen_append(windows, source, kind, "entry", space, &cancel)
}

pub(in crate::desktop::application) unsafe fn run(instance: HINSTANCE) -> Result<()> {
    let first = DocumentWindow::create(instance, State::new(None, false)?, false)?;
    let mut app = Application::new(instance, first);
    app.add_window(false)?;
    let old: Vec<_> = app
        .documents
        .iter()
        .map(|doc| (doc.state.unit_id.clone(), doc.window.0))
        .collect();
    let group = UnitId::parse("frozen-fixture-group").unwrap();
    app.workspace
        .upsert_group(crate::workspace::Group {
            id: group.clone(),
            name: "保留分组".into(),
            hidden: false,
            locked: false,
        })
        .map_err(|_| Error::new(E_FAIL, "Fixture group"))?;
    app.workspace
        .assign_group(&[old[0].0.clone()], Some(&group))
        .map_err(|_| Error::new(E_FAIL, "Fixture membership"))?;
    app.workspace
        .add_link(crate::workspace::Link {
            id: UnitId::parse("frozen-fixture-link").unwrap(),
            from_unit: old[0].0.clone(),
            from_port: "output".into(),
            to_unit: old[1].0.clone(),
            to_port: "image".into(),
        })
        .map_err(|_| Error::new(E_FAIL, "Fixture link"))?;
    app.sync_groups();
    let groups = app.workspace.groups().to_vec();
    let links = app.workspace.links().to_vec();
    let cancel = AtomicBool::new(false);
    let png = crate::loom_base64::encode_png_data_url(
        crate::desktop::application::smoke::image_fixture::PNG,
        &cancel,
    )
    .map_err(|_| Error::new(E_FAIL, "Fixture image encoding"))?;
    let entry = serde_json::json!({"entryId":"entry","sourceStickerId":"frozen-old","createdAt":"legacy",
        "snapshot":{"id":"frozen-old","src":png.as_str(),"x":0,"y":0,"w":100,"h":80,"extensionState":{"keep":"原文"}}});
    app.restored_session.as_mut().unwrap().original = serde_json::to_vec(&serde_json::json!({
        "documentRevision":7,"stickers":[],"links":[],"recycleBin":[entry.clone()],"referenceLibrary":[entry],"rootExtension":{"keep":true}
    })).unwrap().into();
    let retained = app.restored_session.as_ref().unwrap().original.clone();
    app.restored_session.as_mut().unwrap().annotation_ids.push((
        UnitId::parse("already-closed").unwrap(),
        vec!["old-element".into()],
    ));

    // 原窗口发生编辑后整批失败；隐藏候选释放、原库仍由同一 owner 持有。
    let append = prepare(&app, Kind::ReferenceLibrary, "stale-edit")?;
    let rejected = append.windows.documents[0].window.0;
    let opacity = app.documents[0].state.canvas.borrow().normal_opacity;
    app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
    let failed = app.publish_frozen_append(append, false).is_err();
    app.documents[0].state.canvas.borrow_mut().normal_opacity = opacity;
    if !failed
        || IsWindow(Some(rejected)).as_bool()
        || app.documents.len() != 2
        || !app
            .restored_session
            .as_ref()
            .unwrap()
            .original
            .same_storage(&retained)
    {
        return Err(Error::new(E_FAIL, "Stale frozen append changed workspace"));
    }
    // 即使字节相同，会话 owner 被替换也使先前准备失效。
    let append = prepare(&app, Kind::RecycleBin, "stale-source")?;
    let rejected = append.windows.documents[0].window.0;
    app.restored_session.as_mut().unwrap().original = retained.as_ref().to_vec().into();
    if app.publish_frozen_append(append, false).is_ok() || IsWindow(Some(rejected)).as_bool() {
        return Err(Error::new(E_FAIL, "Frozen append accepted replaced source"));
    }
    let original_space = app.restored_session.as_ref().unwrap().space;
    for space in [
        None,
        Some(crate::session_placement::Space {
            origin: (1, 2),
            scale: 2.0,
        }),
    ] {
        let append = prepare(&app, Kind::RecycleBin, "stale-space")?;
        let rejected = append.windows.documents[0].window.0;
        let source = app.restored_session.as_ref().unwrap().original.clone();
        app.restored_session.as_mut().unwrap().space = space;
        let failed = app.publish_frozen_append(append, false).is_err();
        app.restored_session.as_mut().unwrap().space = original_space;
        if !failed
            || IsWindow(Some(rejected)).as_bool()
            || app.documents.len() != 2
            || !app
                .restored_session
                .as_ref()
                .unwrap()
                .original
                .same_storage(&source)
        {
            return Err(Error::new(
                E_FAIL,
                "Frozen append accepted changed or missing coordinates",
            ));
        }
    }
    for (kind, id, recycled) in [
        (Kind::ReferenceLibrary, "reference-copy", 1),
        (Kind::RecycleBin, "recycle-copy", 0),
    ] {
        let append = prepare(&app, kind, id)?;
        let hwnd = append.windows.documents[0].window.0;
        app.publish_frozen_append(append, false)?;
        let record = app.restored_session.as_ref().unwrap();
        let document = crate::session_document::decode(&record.original)
            .map_err(|error| Error::new(E_FAIL, format!("Retained frozen document: {error:?}")))?;
        if document.revision != 7
            || document.library(Kind::RecycleBin).unwrap().entries().len() != recycled
            || document
                .library(Kind::ReferenceLibrary)
                .unwrap()
                .entries()
                .len()
                != 1
            || record
                .annotation_ids
                .iter()
                .any(|(unit, _)| !app.workspace.contains(unit))
            || app.workspace.groups() != groups
            || app.workspace.links() != links
            || app
                .documents
                .last()
                .unwrap()
                .state
                .canvas
                .borrow()
                .image
                .is_none()
            || app.source_window(&UnitId::parse(id).unwrap()) != Some(hwnd)
            || IsWindowVisible(hwnd).as_bool()
            || old
                .iter()
                .any(|(unit, hwnd)| app.source_window(unit) != Some(*hwnd))
        {
            return Err(Error::new(
                E_FAIL,
                "Frozen publication lost ownership or library state",
            ));
        }
    }
    let (_, snapshot) = app.capture_session_snapshot()?;
    let saved = snapshot
        .encode(&cancel)
        .map_err(|error| Error::new(E_FAIL, format!("Frozen save: {error:?}")))?;
    let value: serde_json::Value = serde_json::from_slice(saved.bytes.as_ref()).unwrap();
    let stickers = value["stickers"].as_array().unwrap();
    if !value["recycleBin"].as_array().unwrap().is_empty()
        || value["rootExtension"]["keep"] != true
        || ["reference-copy", "recycle-copy"].iter().any(|id| {
            !stickers
                .iter()
                .any(|item| item["id"] == *id && item["extensionState"]["keep"] == "原文")
        })
        || app.capture_windows.borrow().len() != app.documents.len() * 2
    {
        return Err(Error::new(
            E_FAIL,
            "Frozen publication did not survive save assembly",
        ));
    }
    let windows: Vec<_> = app.documents.iter().map(|doc| doc.window.0).collect();
    drop(app);
    if windows.iter().any(|hwnd| IsWindow(Some(*hwnd)).as_bool()) {
        return Err(Error::new(E_FAIL, "Frozen publication leaked HWND"));
    }
    println!("native-frozen-append-smoke: append=true old-windows=true topology=true stale-rejected=true recycle-commit=true save=true cleanup=true");
    Ok(())
}
