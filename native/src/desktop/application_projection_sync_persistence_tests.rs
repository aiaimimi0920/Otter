//! 源像素证明跨真实保存/重开：历史消失或旧 Binding 无证明也不能覆盖不明本地图。
use super::*;
unsafe fn reopened(mode: u8) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let mut fixture = Fixture::new();
    saved(&mut fixture);
    let id = fixture.ticket().unit().to_owned();
    let before = {
        let doc = fixture
            .app
            .documents
            .iter()
            .find(|d| d.state.unit_id.as_str() == id)
            .unwrap();
        let mut canvas = doc.state.canvas.borrow_mut();
        if mode == 0 {
            let old = canvas.editor.layers().unwrap().clone();
            let erased = old
                .erase(
                    crate::erase_layers::Mode::Content,
                    &[crate::viewport::Point { x: 0.5, y: 0.5 }],
                    4.0,
                    || false,
                )
                .unwrap();
            let revision = canvas.editor.revision();
            assert!(canvas
                .editor
                .replace_layers(revision, &old, erased)
                .unwrap());
            canvas.sync_raster_source();
        }
        canvas.editor.layers().unwrap().base.clone()
    };
    if mode == 1 {
        let record = fixture.app.restored_session.as_mut().unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&record.original).unwrap();
        value["stickers"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|s| s["id"] == id)
            .unwrap()["qrProjection"]
            .as_object_mut()
            .unwrap()
            .remove("sourcePixels");
        record.edit_original(serde_json::to_vec(&value).unwrap().into());
    }
    fixture
        .app
        .start_checkpoint_with(&mut |_, _, message, warning| assert!(!warning, "{message}"));
    checkpoint(&mut fixture.app);
    let rx = sync_fixture::peer_with_renew(&mut fixture, vec![4, 2], true);
    super::super::fixture::reopen(&mut fixture);
    {
        let doc = fixture
            .app
            .documents
            .iter()
            .find(|d| d.state.unit_id.as_str() == id)
            .unwrap();
        let mut canvas = doc.state.canvas.borrow_mut();
        assert_eq!(&*canvas.editor.layers().unwrap().base, &*before);
        assert!(!canvas.editor.undo()); // 不能靠临时历史识别已烘焙的破坏性编辑。
    }
    cycle(&mut fixture, &rx);
    assert_eq!(binding(&fixture)["revision"], 5);
    cycle(&mut fixture, &rx);
    joined(&mut fixture);
    assert_eq!(binding(&fixture)["stopped"], true);
    let canvas = fixture
        .app
        .documents
        .iter()
        .find(|d| d.state.unit_id.as_str() == id)
        .unwrap()
        .state
        .canvas
        .borrow();
    assert_eq!(&*canvas.editor.layers().unwrap().base, &*before);
    assert!(!fixture.app.projection_cleanup.pending_local());
}
#[test]
fn projection_sync_saved_destructive_pixels_cannot_be_overwritten_after_history_is_gone() {
    unsafe { reopened(0) }
}
#[test]
fn projection_sync_legacy_binding_without_pixel_proof_reads_terminal_but_keeps_local_source() {
    unsafe { reopened(1) }
}
