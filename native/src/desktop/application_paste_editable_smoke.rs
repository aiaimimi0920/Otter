//! Editable paste through the real worker and staged window transaction; no clipboard access.
use super::*;
use crate::{
    desktop::{clipboard_editable::Content, image::Image},
    editor::{Editor, Tool},
    viewport::Point,
};
use std::sync::Arc;

pub(super) unsafe fn run(app: &mut Application) -> Result<()> {
    let image = Image {
        width: 8,
        height: 8,
        pixels: bytes(&[255; 256])?,
    };
    let mut editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (8, 8),
        base: image.pixels.clone(),
        annotations: Some(bytes(&[0; 256])?),
        annotation_size: None,
    })
    .map_err(|e| Error::new(E_FAIL, format!("Editable fixture: {e:?}")))?;
    editor
        .begin(Tool::Rectangle, Point { x: 2.0, y: 2.0 })
        .unwrap();
    editor.extend(Point { x: 6.0, y: 6.0 }).unwrap();
    editor.commit();
    editor
        .crop_to(crate::crop_geometry::Rect {
            x: 1.0,
            y: 1.0,
            width: 6.0,
            height: 5.0,
        })
        .unwrap();
    let content = Arc::new(Content {
        image,
        editor,
        opacity: 0.5,
        origin: None,
        capture_source: Some(crate::desktop::canvas::CaptureSource::from_session(
            r#"{"dynamicRange":"sdr","bitDepth":8,"colorSpace":"srgb","futureClipboard":{"keep":true}}"#,
        )?),
    });
    let source_stamp = content.editor.document_stamp();
    let count = app.documents.len();
    let original = app.documents[0]
        .state
        .canvas
        .borrow()
        .editor
        .document_stamp();
    for _ in 0..2 {
        let source = &app.documents[0];
        paste_input::deliver(
            source.window.0,
            &source.state,
            Snapshot::Editable(content.clone()),
            true,
        )?;
        open_job::finish(source.window.0, &source.state, true);
        app.pending_windows(false)?;
    }
    if app.documents.len() != count + 2 {
        return Err(Error::new(
            E_FAIL,
            "Editable paste did not publish two independent pins",
        ));
    }
    let first = &app.documents[count];
    let second = &app.documents[count + 1];
    if first.state.unit_id == second.state.unit_id
        || first.state.unit_id == app.documents[0].state.unit_id
    {
        return Err(Error::new(E_FAIL, "Editable paste reused unit identity"));
    }
    let first_stamp = first.state.canvas.borrow().editor.document_stamp();
    let first_id = first.state.canvas.borrow().editor.elements()[0]
        .annotation_id()
        .to_owned();
    for pin in [first, second] {
        let canvas = pin.state.canvas.borrow();
        if canvas.editor.elements().len() != 1
            || source_stamp.same_document(&canvas.editor)
            || canvas.editor.elements()[0].annotation_id()
                == content.editor.elements()[0].annotation_id()
            || canvas.editor.annotation_frame() != content.editor.annotation_frame()
            || canvas.applied_crop != canvas.editor.annotation_frame()
            || canvas.normal_opacity != 0.5
            || canvas.window_opacity != 0.5
            || !canvas
                .editor
                .layers()
                .unwrap()
                .base
                .same_storage(&content.image.pixels)
            || canvas.capture_source().map(|source| source.metadata())
                != content
                    .capture_source
                    .as_ref()
                    .map(|source| source.metadata())
        {
            return Err(Error::new(
                E_FAIL,
                "Editable paste lost content or isolated identity",
            ));
        }
        let mut rect = RECT::default();
        GetClientRect(pin.window.0, &mut rect)?;
        if rect.right != 6 || rect.bottom != 5 {
            return Err(Error::new(
                E_FAIL,
                "Editable crop was not used for initial pin size",
            ));
        }
        let crop = canvas.editor.annotation_frame().unwrap();
        if (canvas.viewport.origin.x + crop.x as f32 * canvas.viewport.scale).abs() > 0.001
            || (canvas.viewport.origin.y + crop.y as f32 * canvas.viewport.scale).abs() > 0.001
        {
            return Err(Error::new(
                E_FAIL,
                "Editable crop viewport has the wrong origin",
            ));
        }
        drop(canvas);
        let mut before = RECT::default();
        let mut after = RECT::default();
        GetWindowRect(pin.window.0, &mut before)?;
        crate::desktop::crop_window::sync(pin.window.0, &pin.state)?;
        GetWindowRect(pin.window.0, &mut after)?;
        if before != after {
            return Err(Error::new(E_FAIL, "Initial editable crop moved twice"));
        }
    }
    {
        let mut canvas = second.state.canvas.borrow_mut();
        if first_stamp.same_document(&canvas.editor)
            || first_id == canvas.editor.elements()[0].annotation_id()
            || canvas.editor.undo()
        {
            return Err(Error::new(
                E_FAIL,
                "Editable paste shared identity or source undo history",
            ));
        }
        canvas
            .editor
            .begin(Tool::Brush, Point { x: 3.0, y: 3.0 })
            .unwrap();
        canvas.editor.commit();
        if !canvas.editor.undo() {
            return Err(Error::new(E_FAIL, "Pasted editor cannot undo its own edit"));
        }
    }
    if !first_stamp.matches(&first.state.canvas.borrow().editor)
        || !source_stamp.matches(&content.editor)
        || !original.matches(&app.documents[0].state.canvas.borrow().editor)
    {
        return Err(Error::new(E_FAIL, "Pasted edit changed source or sibling"));
    }
    for pin in &app.documents[count..] {
        SendMessageW(pin.window.0, WM_CLOSE, None, None);
    }
    app.collect_closed();
    let source = &app.documents[0];
    paste_input::deliver(
        source.window.0,
        &source.state,
        Snapshot::Editable(content.clone()),
        true,
    )?;
    open_job::cancel(&source.state);
    open_job::finish(source.window.0, &source.state, true);
    if source.state.new_pin.borrow().is_some()
        || paste_input::deliver(
            source.window.0,
            &source.state,
            Snapshot::Editable(content),
            false,
        )
        .is_ok()
    {
        return Err(Error::new(
            E_FAIL,
            "Editable cancellation or replacement boundary failed",
        ));
    }
    println!("native-paste-editable-smoke: independent-units=true fresh-annotations=true crop=true opacity=true undo-isolated=true cancelled=true clipboard-access=false");
    println!("native-paste-capture-smoke: metadata=true unknown-fields=true new-document-binding=true path-access=false");
    Ok(())
}
