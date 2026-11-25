//! Save/restore active attachments, source versions, and visibility while preserving unknown schemas unchanged; do not bake overlays into base images.
use super::*;
use crate::desktop::{extension_document, session_binding::Bound, session_candidate, State};
use serde_json::json;
#[test]
fn legacy_cached_full_text_survives_real_restore_save_and_becomes_stale_after_content_commit() {
    use crate::sticker_extra_text::{text, Kind};
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let original = json!({"stickers":[{"id":"cached","x":10,"y":20,"w":40,"h":30,
        "ocrResult":{"fullText":" 原文😀\n "},"translationResult":{"fullText":" 译文 ","sourceRevision":4},
        "stickerEditPropagation":{"revision":4}}],"links":[]});
    let bytes = serde_json::to_vec(&original).unwrap();
    let restore = |bytes: Vec<u8>| {
        let candidate = session_candidate::prepare(bytes, None, &cancel).unwrap();
        let vectors = session_candidate::VectorCandidate::prepare(candidate, &cancel).unwrap();
        Bound::prepare_for(vectors, None, &cancel)
            .ok()
            .unwrap()
            .into_parts_for(None, &cancel)
            .ok()
            .unwrap()
            .prepare_states(&cancel)
            .unwrap()
    };
    let prepared = restore(bytes.clone());
    let owner = prepared.states[0].extension_document.borrow();
    assert_eq!(
        text(
            Kind::Ocr,
            &owner.store,
            owner.revision(),
            &owner.legacy_text
        ),
        Some(" 原文😀\n ")
    );
    assert_eq!(
        text(
            Kind::Translation,
            &owner.store,
            owner.revision(),
            &owner.legacy_text
        ),
        Some(" 译文 ")
    );
    let mut document = doc("cached");
    document.extensions = Some(owner.snapshot());
    drop(owner);
    let mut snapshot = data(vec![document]);
    snapshot.original = bytes.into();
    let saved = snapshot.assemble(&cancel, plain).unwrap();
    let saved_value: Value = serde_json::from_slice(&saved.bytes).unwrap();
    for key in ["ocrResult", "translationResult"] {
        assert_eq!(
            saved_value["stickers"][0][key],
            original["stickers"][0][key]
        );
    }
    let restored = restore(saved.bytes.to_vec());
    let state = &restored.states[0];
    extension_document::observe(state);
    {
        let mut c = state.canvas.borrow_mut();
        c.editor
            .begin(
                crate::editor::Tool::Brush,
                crate::viewport::Point { x: 1., y: 1. },
            )
            .unwrap();
        assert!(c.editor.commit());
    }
    extension_document::observe(state);
    let owner = state.extension_document.borrow();
    assert_eq!(owner.revision(), Some(5));
    assert_eq!(
        text(
            Kind::Translation,
            &owner.store,
            owner.revision(),
            &owner.legacy_text
        ),
        None
    );
    assert_eq!(
        text(
            Kind::Ocr,
            &owner.store,
            owner.revision(),
            &owner.legacy_text
        ),
        Some(" 原文😀\n ")
    );
}
#[test]
fn attachment_visibility_and_source_revision_survive_real_session_save_restore() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let image = crate::desktop::image::Image {
        width: 40,
        height: 30,
        pixels: vec![255; 40 * 30 * 4].into(),
    };
    let png = crate::desktop::export::png_raw(&image, &cancel).unwrap();
    let url = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
    let original = json!({"stickers":[{"id":"overlay","x":10,"y":20,"w":40,"h":30,"src":url.as_str(),"stickerEditPropagation":{"revision":4,"future":true}}],"links":[]});
    let state = State::new(Some(image), true).unwrap();
    let raw = serde_json::value::RawValue::from_string("{\"revision\":4}".into()).unwrap();
    *state.extension_document.borrow_mut() = extension_document::Owner::restore(
        crate::extension_visual::tests::attachments(),
        Some(&raw),
    );
    extension_document::observe(&state);
    let stamp = state.extension_document.borrow().stamp();
    state
        .extension_document
        .borrow_mut()
        .store
        .visibility(&[("neuro.official/ocr.result", false)])
        .unwrap();
    assert!(!stamp.matches(&state.extension_document.borrow()));
    state.canvas.borrow_mut().normal_opacity = 0.5;
    state.canvas.borrow_mut().editor.record_pin_change(
        crate::editor::PinValue::Opacity {
            mini: false,
            value: 1.0,
        },
        crate::editor::PinValue::Opacity {
            mini: false,
            value: 0.5,
        },
    );
    extension_document::observe(&state);
    let mut document = doc("overlay");
    document.editor = state.canvas.borrow().editor.export_snapshot();
    document.extensions = Some(state.extension_document.borrow().snapshot());
    let mut snapshot = data(vec![document]);
    snapshot.original = serde_json::to_vec(&original).unwrap().into();
    let saved = snapshot.assemble(&cancel, plain).unwrap();
    let read = crate::session_document::decode(&saved.bytes).unwrap();
    let propagation: Value =
        serde_json::from_str(read.windows[0].sticker_edit_propagation.unwrap().get()).unwrap();
    assert_eq!(propagation["revision"], 4);
    assert_eq!(propagation["future"], true);
    let candidate = session_candidate::prepare(saved.bytes.to_vec(), None, &cancel).unwrap();
    let vectors = session_candidate::VectorCandidate::prepare(candidate, &cancel).unwrap();
    let restored = Bound::prepare_for(vectors, None, &cancel)
        .ok()
        .unwrap()
        .into_parts_for(None, &cancel)
        .ok()
        .unwrap()
        .prepare_states(&cancel)
        .unwrap();
    let owner = restored.states[0].extension_document.borrow();
    assert_eq!(owner.revision(), Some(4));
    assert_eq!(owner.store.items().len(), 2);
    assert_eq!(owner.store.items()[0].visible, Some(false));
    assert!(restored.states[0]
        .canvas
        .borrow()
        .image
        .as_ref()
        .unwrap()
        .pixels
        .iter()
        .all(|v| *v == 255));
}
#[test]
fn unknown_schema_is_retained_and_empty_owner_clears_old_attachments_on_save() {
    let original = json!({"stickers":[{"id":"unknown","x":10,"y":20,"w":40,"h":30,"extensionState":{"schemaVersion":2,"future":[1,2,3]}}],"links":[]});
    let raw = serde_json::value::RawValue::from_string(
        original["stickers"][0]["extensionState"].to_string(),
    )
    .unwrap();
    for keep in [true, false] {
        let mut d = doc("unknown");
        let owner = if keep {
            extension_document::Owner::restore(
                crate::unit_attachment_store::Store::restore(Some(&raw)),
                None,
            )
        } else {
            extension_document::Owner::default()
        };
        d.extensions = Some(owner.snapshot());
        let mut s = data(vec![d]);
        s.original = serde_json::to_vec(&original).unwrap().into();
        let saved = s.assemble(&AtomicBool::new(false), plain).unwrap();
        let value: Value = serde_json::from_slice(&saved.bytes).unwrap();
        if keep {
            assert_eq!(
                value["stickers"][0]["extensionState"],
                original["stickers"][0]["extensionState"]
            );
        } else {
            assert!(value["stickers"][0].get("extensionState").is_none());
        }
    }
}
#[test]
fn source_revision_tracks_commits_and_external_frames_not_presentation_or_drafts() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let state = State::new(None, true).unwrap();
    extension_document::observe(&state);
    {
        let mut c = state.canvas.borrow_mut();
        c.normal_opacity = 0.5;
        c.viewport.scale = 2.;
        c.editor.record_pin_change(
            crate::editor::PinValue::Opacity {
                mini: false,
                value: 1.,
            },
            crate::editor::PinValue::Opacity {
                mini: false,
                value: 0.5,
            },
        );
    }
    extension_document::observe(&state);
    assert_eq!(state.extension_document.borrow().revision(), Some(0));
    for redo in [false, true] {
        {
            let mut c = state.canvas.borrow_mut();
            assert!(if redo {
                c.editor.redo()
            } else {
                c.editor.undo()
            });
            c.editor.take_pin_change();
        }
        extension_document::observe(&state);
        assert_eq!(state.extension_document.borrow().revision(), Some(0));
    }
    state
        .canvas
        .borrow_mut()
        .editor
        .begin(
            crate::editor::Tool::Brush,
            crate::viewport::Point { x: 1., y: 1. },
        )
        .unwrap();
    extension_document::observe(&state);
    assert_eq!(state.extension_document.borrow().revision(), Some(0));
    assert!(state.canvas.borrow_mut().editor.commit());
    extension_document::observe(&state);
    assert_eq!(state.extension_document.borrow().revision(), Some(1));
    for expected in [2, 3] {
        *state.resolved_display.borrow_mut() = Some(crate::desktop::display_frame::Frame::capture(
            state.unit_id.clone(),
            crate::desktop::image::Image {
                width: 1,
                height: 1,
                pixels: vec![255; 4].into(),
            },
            None,
        ));
        extension_document::observe(&state);
        assert_eq!(state.extension_document.borrow().revision(), Some(expected));
        extension_document::observe(&state);
        assert_eq!(state.extension_document.borrow().revision(), Some(expected));
    }
}
#[test]
fn malformed_propagation_is_preserved_without_authority_and_null_is_optional_absence() {
    for propagation in [
        json!(false),
        json!([1]),
        json!("future"),
        json!({"revision":"future"}),
        Value::Null,
    ] {
        let original = json!({"stickers":[{"id":"legacy","x":10,"y":20,"w":40,"h":30,"stickerEditPropagation":propagation}],"links":[]});
        let bytes = serde_json::to_vec(&original).unwrap();
        let read = crate::session_document::decode(&bytes).unwrap();
        let owner = extension_document::Owner::restore(
            crate::unit_attachment_store::Store::default(),
            read.windows[0].sticker_edit_propagation,
        );
        assert_eq!(owner.revision(), propagation.is_null().then_some(0));
        let mut d = doc("legacy");
        d.extensions = Some(owner.snapshot());
        let mut s = data(vec![d]);
        s.original = bytes.into();
        let saved = s.assemble(&AtomicBool::new(false), plain).unwrap();
        let value: Value = serde_json::from_slice(&saved.bytes).unwrap();
        assert_eq!(
            value["stickers"][0]["stickerEditPropagation"],
            if propagation.is_null() {
                json!({"revision":0,"acceptUpstream":true,"locallyEdited":false})
            } else {
                propagation
            }
        );
    }
}
