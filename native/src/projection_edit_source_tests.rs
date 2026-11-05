//! stage写前/原initial/精确请求恢复；合成PNG和文档oracle不经过被测网络流程。
use super::*;
use crate::projection_edit::{fixture::*, journal::Store, Operation, Request, Route};
use serde_json::json;
fn frozen(color: u8, session: &str, x: u8) -> Frozen {
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[color, 0, 0, 255].repeat(4))
            .unwrap();
    }
    Frozen::new(
        PROJECTION,
        session,
        Frame {
            png: png.into(),
            width: 2,
            height: 2,
            kind: sender::Kind::Sticker,
        },
        Layout::new(2.0, 2.0, 0.0, 0.0).unwrap(),
        |_, _| {
            let mut objects = view();
            objects.get_mut("shape-1").unwrap()["x"] = json!(x);
            Ok(objects)
        },
        &AtomicBool::new(false),
    )
    .unwrap()
}
#[test]
fn source_stage_is_durable_reopen_keeps_original_session_png_and_initial_despite_later_edits() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let store = directory.store();
    let staged = store
        .stage_source(
            context("https://loom.example"),
            frozen(17, SESSION, 3),
            &cancel,
        )
        .unwrap();
    let before = std::fs::read(directory.path()).unwrap();
    let later = store
        .stage_source(
            context("https://loom.example"),
            frozen(17, "edit:33333333333333333333333333333333", 99),
            &cancel,
        )
        .unwrap();
    assert_eq!(later, staged);
    assert_eq!(std::fs::read(directory.path()).unwrap(), before);
    assert!(store.pending(&cancel).unwrap().is_empty());
    drop((store, later));
    let reopened = directory.store();
    let loaded = reopened
        .source(&context("https://loom.example"), &cancel)
        .unwrap()
        .unwrap();
    assert_eq!(loaded, staged);
    assert_eq!(loaded.initial()["shape-1"]["x"], 3);
    assert_eq!(
        loaded.frame(sender::Kind::Sticker, &cancel).unwrap().png,
        staged.frame(sender::Kind::Sticker, &cancel).unwrap().png
    );
    assert_eq!(std::fs::read(directory.path()).unwrap(), before);
}
#[test]
fn source_stage_rejects_changed_basis_device_cancel_or_tampering_without_overwriting_original() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let store = directory.store();
    let ctx = context("https://loom.example");
    store
        .stage_source(ctx.clone(), frozen(17, SESSION, 3), &cancel)
        .unwrap();
    let before = std::fs::read(directory.path()).unwrap();
    assert!(store
        .stage_source(ctx.clone(), frozen(18, SESSION, 3), &cancel)
        .is_err());
    let other = crate::projection_edit::journal::Context::new(
        ctx.scope(),
        ctx.unit(),
        ctx.origin(),
        "other-device",
    )
    .unwrap();
    assert!(store
        .stage_source(other, frozen(17, SESSION, 3), &cancel)
        .is_err());
    assert!(store.source(&ctx, &AtomicBool::new(true)).is_err());
    assert_eq!(std::fs::read(directory.path()).unwrap(), before);
    drop(store);
    let mut corrupt: serde_json::Value = serde_json::from_slice(&before).unwrap();
    corrupt["entries"][0]["source"]["snapshot"]["width"] = json!(3);
    std::fs::write(directory.path(), serde_json::to_vec(&corrupt).unwrap()).unwrap();
    assert!(Store::open(&directory.0, &cancel).is_err());
}
#[test]
fn source_initialize_pending_is_exact_and_only_checkpoint_commit_retires_it_not_frozen_basis() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let store = directory.store();
    let ctx = context("https://loom.example");
    let source = store
        .stage_source(ctx.clone(), frozen(17, SESSION, 3), &cancel)
        .unwrap();
    let request = Request::prepare(Operation::Initialize {
        projection_id: PROJECTION.into(),
        session_id: SESSION.into(),
        expected_digest: source.basis().digest().into(),
        objects: source.initial().clone(),
    })
    .unwrap();
    let ticket = store
        .begin(
            ctx.clone(),
            Route::Shared,
            request,
            None,
            source.initial().clone(),
            &cancel,
        )
        .unwrap();
    let bytes = ticket.request().bytes().to_vec();
    drop((ticket, store));
    let store = directory.store();
    let pending = store.pending(&cancel).unwrap().remove(0);
    assert_eq!(pending.request().bytes(), bytes);
    assert!(store.source_document(&ctx, &cancel).unwrap().is_none());
    let mut value = document_value();
    value["basis"] = serde_json::to_value(source.basis()).unwrap();
    pending
        .commit_after_checkpoint(
            crate::projection_edit::Document::parse(&serde_json::to_vec(&value).unwrap()).unwrap(),
            &cancel,
        )
        .unwrap();
    assert!(store.pending(&cancel).unwrap().is_empty());
    assert_eq!(store.source(&ctx, &cancel).unwrap(), Some(source));
}
