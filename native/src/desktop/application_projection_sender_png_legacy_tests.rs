//! legacy有效凭证必须来自真实恢复；菜单发送覆盖reset/undo及canonical保存再恢复。
use super::*;
use crate::desktop::{session_binding::Bound, session_candidate};
use serde_json::json;

fn pixel(x: usize, y: usize) -> [u8; 4] {
    [(x * 24) as u8, (y * 24) as u8, 32, 255]
}

pub(super) unsafe fn restore_canvas(app: &mut Application, bytes: Vec<u8>) {
    let cancel = AtomicBool::new(false);
    let candidate = session_candidate::prepare(bytes, None, &cancel).unwrap();
    let vectors = session_candidate::VectorCandidate::prepare(candidate, &cancel).unwrap();
    let bound = Bound::prepare_for(vectors, None, &cancel).ok().unwrap();
    let mut prepared = bound
        .into_parts_for(None, &cancel)
        .ok()
        .unwrap()
        .prepare_states(&cancel)
        .unwrap();
    assert_eq!(prepared.states.len(), 1);
    let restored = prepared.states.remove(0);
    assert_eq!(restored.unit_id, app.documents[0].state.unit_id);
    let mut recovered = restored.canvas.borrow_mut();
    let mut canvas = app.documents[0].state.canvas.borrow_mut();
    // 保留实际恢复所创建的Editor文档身份及其retained关联，不手工设置active标志。
    canvas.editor = std::mem::take(&mut recovered.editor);
    canvas.minified_source = recovered.minified_source.take();
    canvas.image = recovered.image.take();
}

pub(super) unsafe fn install(app: &mut Application, case: Case) -> Image {
    let unit = app.documents[0].state.unit_id.clone();
    let natural = Image {
        width: 8,
        height: 8,
        pixels: (0..64)
            .flat_map(|i| pixel(i % 8, i / 8))
            .collect::<Vec<_>>()
            .into(),
    };
    let cancel = AtomicBool::new(false);
    let png = crate::desktop::export::png_raw(&natural, &cancel).unwrap();
    let url = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
    let original = serde_json::to_vec(&json!({"stickers":[{
        "id":unit.as_str(),"x":0,"y":0,"w":8,"h":8,"src":url.as_str(),
        "imageEditState":{"cropRect":{"x":2,"y":0,"w":2,"h":8}}
    }],"links":[]}))
    .unwrap();
    restore_canvas(app, original.clone());
    app.restored_session
        .as_mut()
        .unwrap()
        .edit_original(original.into());
    settle(app);
    let reset = matches!(
        case,
        Case::NaturalLegacyReset | Case::NaturalLegacyResetSaveReopen
    );
    if reset || matches!(case, Case::NaturalLegacyUndo) {
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        assert!(
            canvas.editor.reset_crop(),
            "actual restored legacy crop must be resettable"
        );
        if matches!(case, Case::NaturalLegacyUndo) {
            assert!(canvas.editor.undo());
        }
    }
    if matches!(
        case,
        Case::NaturalLegacySaveReopen | Case::NaturalLegacyResetSaveReopen
    ) {
        // 原Save/Checkpoint完成后只从已确认canonical读回；不把暂存候选当恢复凭证。
        settle(app);
        let saved = app
            .workspace_checkpoint
            .as_ref()
            .unwrap()
            .sender_source()
            .unwrap();
        restore_canvas(app, saved.to_vec());
    }
    if reset {
        return natural;
    }
    Image {
        width: 8,
        height: 8,
        pixels: (0..64)
            .flat_map(|i| pixel(2 + (i % 8) / 4, i / 8))
            .collect::<Vec<_>>()
            .into(),
    }
}

#[test]
fn native_sender_png_input_replays_only_a_proven_active_legacy_pixel_crop() {
    tls::run_source(0, Case::NaturalLegacyCrop);
}
#[test]
fn native_sender_restored_legacy_reset_uses_full_source_without_canonical_crop_resurrection() {
    tls::run_source(0, Case::NaturalLegacyReset);
}
#[test]
fn native_sender_restored_legacy_undo_recovers_active_crop_exactly_once() {
    tls::run_source(0, Case::NaturalLegacyUndo);
}
#[test]
fn native_sender_restored_legacy_canonical_save_reopen_does_not_materialize_crop_twice() {
    tls::run_source(0, Case::NaturalLegacySaveReopen);
}
#[test]
fn native_sender_restored_legacy_reset_save_reopen_never_revives_old_crop() {
    tls::run_source(0, Case::NaturalLegacyResetSaveReopen);
}
