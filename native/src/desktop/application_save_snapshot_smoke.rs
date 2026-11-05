//! 真隐藏 Application 保存快照，不写磁盘；验证数据可传 worker、共享原文及过期拒绝。
use super::*;
pub(in crate::desktop::application) unsafe fn verify(app: &mut Application) -> Result<()> {
    fn send<T: Send>() {}
    send::<Data>();
    let index = app
        .documents
        .iter()
        .position(|doc| !doc.state.is_art())
        .ok_or_else(|| Error::new(E_FAIL, "Missing snapshot sticker fixture"))?;
    app.documents[index].state.file_dialog.set(true);
    let blocked = app.capture_session_snapshot().is_err();
    app.documents[index].state.file_dialog.set(false);
    if !blocked {
        return Err(Error::new(E_FAIL, "Save snapshot admitted active dialog"));
    }
    {
        let mut canvas = app.documents[index].state.canvas.borrow_mut();
        canvas.editor = Editor::with_layers(crate::erase_layers::Layers {
            size: (2, 2),
            base: crate::pixels::Pixels::from_reserved(
                vec![255; 16],
                crate::pixels::reserve(16)
                    .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Snapshot fixture budget"))?,
            ),
            annotation_size: None,
            annotations: None,
        })
        .map_err(|_| Error::new(E_FAIL, "Cannot create snapshot pixel fixture"))?;
        canvas.sync_raster_source();
        let source = crate::desktop::image::Image {
            width: 4,
            height: 4,
            pixels: crate::pixels::Pixels::from_reserved(
                vec![255; 64],
                crate::pixels::reserve(64)
                    .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Snapshot source fixture budget"))?,
            ),
        };
        canvas.minified_source = Some(crate::desktop::canvas::minified_source::Source::new(
            source,
            &canvas.editor,
        ));
    }
    let (receipt, data) = app.capture_session_snapshot()?;
    let original = &app.restored_session.as_ref().unwrap().original;
    let record = app.restored_session.as_ref().unwrap();
    let old: serde_json::Value = serde_json::from_slice(&data.original)
        .map_err(|_| Error::new(E_FAIL, "Invalid snapshot fixture source"))?;
    let mut saved_arts = 0;
    if data.space.origin != record.space.unwrap().origin
        || data.space.scale != record.space.unwrap().scale
        || data.links != app.workspace.links()
        || data.groups != app.workspace.groups()
        || data.transient_host != record.transient_host
    {
        return Err(Error::new(
            E_FAIL,
            "Snapshot lost graph or coordinate context",
        ));
    }
    for frozen in &data.documents {
        let original_window = old["stickers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|window| window["id"].as_str() == Some(frozen.unit.as_str()))
            .unwrap();
        let saved_geometry = crate::session_window_write::merge(
            original_window.clone(),
            frozen.geometry()?,
            data.space,
        )
        .map_err(|_| Error::new(E_FAIL, "Snapshot geometry merge failed"))?;
        let geometry_json = saved_geometry.to_string();
        let window: crate::session_window::Window = serde_json::from_str(&geometry_json)
            .map_err(|_| Error::new(E_FAIL, "Snapshot geometry readback failed"))?;
        if data.space.physical(window.rect()).ok() != Some(frozen.frame) {
            return Err(Error::new(
                E_FAIL,
                "Snapshot geometry lost actual window frame",
            ));
        }
        let doc = app
            .documents
            .iter()
            .find(|doc| doc.state.unit_id == frozen.unit)
            .unwrap();
        let canvas = doc.state.canvas.borrow();
        let mapping = crate::session_vector_write::Mapping::from_viewport(
            frozen.viewport,
            frozen.dpi,
            data.space,
        )
        .map_err(|_| Error::new(E_FAIL, "Snapshot presentation cannot map to session units"))?;
        let point = crate::viewport::Point { x: 1.25, y: 2.5 };
        let mapped = mapping
            .point(point)
            .map_err(|_| Error::new(E_FAIL, "Snapshot mapping overflow"))?;
        let physical =
            (point.x * frozen.viewport.scale + frozen.viewport.origin.x) * frozen.dpi as f32 / 96.0;
        if (mapped.x as f64 * data.space.scale - physical as f64).abs() > 0.001 {
            return Err(Error::new(
                E_FAIL,
                "Snapshot mapping lost physical presentation",
            ));
        }
        let selected = doc.state.selected_art.borrow();
        if let (Some(parameters), Some(execution)) = (&frozen.parameters, frozen.execution) {
            let previous = old["stickers"]
                .as_array()
                .unwrap()
                .iter()
                .find(|window| window["id"].as_str() == Some(frozen.unit.as_str()))
                .unwrap()
                .as_object()
                .unwrap()
                .clone();
            let saved = crate::session_art_write::merge(previous, parameters, execution);
            if saved["params"].get("opaque").is_some()
                || saved["artId"].as_str() != Some(parameters.art_id())
                || saved["executionConfig"]["triggerMode"]["paramDriven"] != execution.param_driven
            {
                return Err(Error::new(
                    E_FAIL,
                    "Save Art revived filtered parameters or lost execution state",
                ));
            }
            saved_arts += 1;
        }
        let mut actual = RECT::default();
        GetWindowRect(doc.window.0, &mut actual)?;
        if frozen.frame.x != actual.left
            || frozen.viewport != canvas.viewport
            || frozen.dpi != windows::Win32::UI::HiDpi::GetDpiForWindow(doc.window.0)
            || frozen.frame.y != actual.top
            || i64::from(frozen.frame.width) != i64::from(actual.right) - i64::from(actual.left)
            || i64::from(frozen.frame.height) != i64::from(actual.bottom) - i64::from(actual.top)
            || frozen.minified != canvas.minified.as_ref().map(|mini| mini.geometry())
            || frozen.transition != canvas.minified_restore
            || frozen.normal_opacity != canvas.normal_opacity
            || frozen.mini_opacity != canvas.minified_opacity
            || frozen.pinned != canvas.pinned
            || frozen.art != doc.state.is_art()
            || frozen.group != *doc.state.group_id.borrow()
            || frozen.execution != selected.as_ref().map(|value| value.execution)
            || frozen.parameters.as_ref().map(|value| value.art_id())
                != selected.as_ref().map(|value| value.parameters.art_id())
            || !same_image(frozen.local_image.as_ref(), canvas.image.as_ref())
            || !same_image(
                frozen.restored_source.as_ref(),
                canvas
                    .minified_source
                    .as_ref()
                    .filter(|source| source.matches(&canvas.editor))
                    .map(|source| source.image()),
            )
        {
            return Err(Error::new(
                E_FAIL,
                "Snapshot document does not match its live owner",
            ));
        }
    }
    if !receipt.matches(app)
        || !original.same_storage(&data.original)
        || original.as_ptr() != data.original.as_ptr()
        || data.documents.len() != app.documents.len()
    {
        return Err(Error::new(
            E_FAIL,
            "Snapshot copied original or lost workspace state",
        ));
    }
    if saved_arts != 1 {
        return Err(Error::new(
            E_FAIL,
            "Save Art fixture did not exercise bound parameters",
        ));
    }
    println!("native-save-art-smoke: retained-secret-filtered=true runtime-parameters=true");
    let unit = &app.documents[index].state.unit_id;
    let frozen = data.documents.iter().find(|doc| &doc.unit == unit).unwrap();
    let count = frozen.editor.elements().len();
    let image_editor = frozen.editor.export_snapshot();
    let source = data.original.clone();
    let image_unit = frozen.unit.clone();
    let (encoded, decoded) = std::thread::spawn(move || {
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let encoded = images::encode(&image_editor, &cancel)?
            .ok_or_else(|| Error::new(E_FAIL, "Session image worker lost layers"))?
            .into_data_urls(&cancel)?;
        let dimensions = (
            encoded.base.width,
            encoded.base.height,
            encoded.annotations.is_some(),
        );
        let original = crate::session_document::decode(&source)
            .map_err(|_| Error::new(E_FAIL, "Invalid save image source"))?;
        let topology = crate::workspace_session::Topology::capture(&original.workspace);
        let mut encoded = Some(encoded);
        let candidate =
            crate::session_write::assemble_owned(&source, &topology, &cancel, |unit, fields| {
                Ok(if unit.id == image_unit.as_str() {
                    encoded.take().unwrap().install(fields)?
                } else {
                    crate::session_write::WindowCandidate::new(fields)
                })
            })
            .map_err(|_| Error::new(E_FAIL, "Cannot assemble save image candidate"))?;
        let restored = crate::session_document::decode(&candidate.bytes)
            .map_err(|_| Error::new(E_FAIL, "Invalid save image candidate"))?;
        let window = restored
            .windows
            .iter()
            .find(|window| window.id == image_unit.as_str())
            .ok_or_else(|| Error::new(E_FAIL, "Save image window missing"))?;
        let decoded = crate::desktop::session_image::inline(
            window
                .src
                .as_deref()
                .ok_or_else(|| Error::new(E_FAIL, "Save image source missing"))?,
            &cancel,
        )?;
        Ok::<_, Error>((dimensions, decoded))
    })
    .join()
    .map_err(|_| Error::new(E_FAIL, "Session image worker failed"))??;
    if encoded != (2, 2, false)
        || (decoded.width, decoded.height) != (2, 2)
        || decoded.pixels.as_ref() != frozen.editor.layers().unwrap().base.as_ref()
    {
        return Err(Error::new(
            E_FAIL,
            "Session image worker produced invalid resource",
        ));
    }
    println!("native-save-images-smoke: worker=true data-url=true restored-pixels=true");
    if !frozen.editor.layers().unwrap().base.same_storage(
        &app.documents[index]
            .state
            .canvas
            .borrow()
            .editor
            .layers()
            .unwrap()
            .base,
    ) {
        return Err(Error::new(E_FAIL, "Save snapshot copied source pixels"));
    }
    let opacity = app.documents[index].state.canvas.borrow().normal_opacity;
    let viewport = app.documents[index].state.canvas.borrow().viewport;
    app.documents[index]
        .state
        .canvas
        .borrow_mut()
        .viewport
        .origin
        .x += 1.0;
    let view_stale = !receipt.matches(app);
    app.documents[index].state.canvas.borrow_mut().viewport = viewport;
    if !view_stale || !receipt.matches(app) {
        return Err(Error::new(E_FAIL, "Snapshot ignored viewport-only change"));
    }
    app.documents[index]
        .state
        .canvas
        .borrow_mut()
        .viewport
        .scale = f32::NAN;
    let invalid = app.capture_session_snapshot().is_err();
    app.documents[index].state.canvas.borrow_mut().viewport = viewport;
    if !invalid {
        return Err(Error::new(E_FAIL, "Snapshot accepted invalid viewport"));
    }
    let space = app.restored_session.as_ref().unwrap().space.unwrap();
    app.restored_session.as_mut().unwrap().space = Some(Space {
        origin: (space.origin.0 + 1, space.origin.1),
        ..space
    });
    let context_stale = !receipt.matches(app);
    app.restored_session.as_mut().unwrap().space = Some(space);
    if !context_stale || !receipt.matches(app) {
        return Err(Error::new(
            E_FAIL,
            "Snapshot ignored source coordinate context change",
        ));
    }
    println!("native-save-view-smoke: viewport-stale=true invalid-rejected=true dpi-captured=true context-stale=true");
    app.documents[index]
        .state
        .canvas
        .borrow_mut()
        .normal_opacity = opacity / 2.0;
    let stale = !receipt.matches(app);
    app.documents[index]
        .state
        .canvas
        .borrow_mut()
        .normal_opacity = opacity;
    if !stale || !receipt.matches(app) {
        return Err(Error::new(E_FAIL, "Snapshot ignored presentation change"));
    }
    {
        let mut canvas = app.documents[index].state.canvas.borrow_mut();
        canvas
            .editor
            .begin(
                crate::editor::Tool::Brush,
                crate::viewport::Point::default(),
            )
            .map_err(|_| Error::new(E_FAIL, "Cannot edit snapshot fixture"))?;
        if !canvas.editor.commit() {
            return Err(Error::new(E_FAIL, "Cannot commit snapshot fixture"));
        }
    }
    if receipt.matches(app) || frozen.editor.elements().len() != count {
        return Err(Error::new(
            E_FAIL,
            "Snapshot accepted edited source or shared mutable vectors",
        ));
    }
    println!("native-save-snapshot-smoke: shared-source=true worker-send=true modal-rejected=true stale-rejected=true isolated=true");
    let (_, assembled) = app.capture_session_snapshot()?;
    let expected = assembled.documents.len();
    let expected_annotations: Vec<_> = assembled
        .documents
        .iter()
        .map(|doc| (doc.unit.as_str().to_owned(), doc.editor.elements().len()))
        .collect();
    let candidate = std::thread::spawn(move || {
        let cancel = std::sync::atomic::AtomicBool::new(false);
        assembled.encode(&cancel)
    })
    .join()
    .map_err(|_| Error::new(E_FAIL, "Save assembly worker failed"))?
    .map_err(|_| Error::new(E_FAIL, "Save assembly candidate failed"))?;
    let read = crate::session_document::decode(&candidate.bytes)
        .map_err(|_| Error::new(E_FAIL, "Save assembly readback failed"))?;
    if read.windows.len() != expected {
        return Err(Error::new(E_FAIL, "Save assembly lost active documents"));
    }
    for (unit, count) in expected_annotations {
        let window = read
            .windows
            .iter()
            .find(|window| window.id == unit)
            .unwrap();
        let state: serde_json::Value = serde_json::from_str(window.annotation_state.unwrap().get())
            .map_err(|_| Error::new(E_FAIL, "Save annotation state readback failed"))?;
        if state["elements"].as_array().map(Vec::len) != Some(count) {
            return Err(Error::new(
                E_FAIL,
                "Save candidate retained stale annotations",
            ));
        }
    }
    let saved_image = read
        .windows
        .iter()
        .find(|window| window.id == app.documents[index].state.unit_id.as_str())
        .unwrap();
    let image = crate::desktop::session_image::inline(
        saved_image.src.as_deref().unwrap(),
        &std::sync::atomic::AtomicBool::new(false),
    )?;
    if (image.width, image.height) != (4, 4) || image.pixels.as_ref() != [255; 64] {
        return Err(Error::new(
            E_FAIL,
            "Save assembly lost retained source resolution",
        ));
    }
    super::super::save_control::verify(app)?;
    Ok(())
}

fn same_image(
    a: Option<&crate::desktop::image::Image>,
    b: Option<&crate::desktop::image::Image>,
) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.width == b.width && a.height == b.height && a.pixels.same_storage(&b.pixels)
        }
        _ => false,
    }
}
