//! 自动持续读取/原位更新/持久重开，及迟到、撤权、终态与不可重基编辑保护。
use super::*;
use crate::projection_delivery::accept::tests as png;
#[path = "application_projection_edit_receive_tests.rs"]
mod editing;
#[path = "application_projection_sync_guard_tests.rs"]
mod guards;
#[path = "application_projection_sync_persistence_tests.rs"]
mod persistence;
#[path = "application_projection_sync_poll_tests.rs"]
mod polling;
#[path = "application_projection_sync_fixture.rs"]
mod sync_fixture;
use sync_fixture::{completed, cycle, peer as sync_peer};
fn source(fixture: &Fixture) -> crate::session_source::Source {
    fixture
        .app
        .restored_session
        .as_ref()
        .unwrap()
        .original
        .clone()
}
fn binding(fixture: &Fixture) -> serde_json::Value {
    let value: serde_json::Value = serde_json::from_slice(&source(fixture)).unwrap();
    value["stickers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == fixture.ticket().unit())
        .unwrap()["qrProjection"]
        .clone()
}
#[test]
fn projection_sync_automatic_unchanged_changed_and_terminal_preserve_local_edits_then_reopen() {
    unsafe { run_routed(false) }
}
unsafe fn run_routed(offline: bool) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = if offline {
            Fixture::offline(false)
        } else {
            Fixture::new()
        };
        saved(&mut fixture);
        let id = fixture.ticket().unit().to_owned();
        let count = fixture.app.documents.len();
        let journal = fixture.journal();
        let old = source(&fixture);
        let doc = fixture
            .app
            .documents
            .iter()
            .find(|d| d.state.unit_id.as_str() == id)
            .unwrap();
        let mut rect = RECT::default();
        GetWindowRect(doc.window.0, &mut rect).unwrap();
        doc.state
            .execution_results
            .borrow_mut()
            .restore_value_outputs(
                &serde_json::value::RawValue::from_string(r#"{"text":"old source output"}"#.into())
                    .unwrap(),
            )
            .unwrap();
        {
            let mut canvas = doc.state.canvas.borrow_mut();
            canvas.normal_opacity = 0.6;
            canvas.window_opacity = 0.6;
            canvas
                .editor
                .begin(
                    crate::editor::Tool::Brush,
                    crate::viewport::Point { x: 0.5, y: 0.5 },
                )
                .unwrap();
            assert!(canvas.editor.commit());
        }
        let rx = sync_peer(&mut fixture, vec![0, 1, if offline { 5 } else { 2 }]);
        cycle(&mut fixture, &rx);
        assert!(source(&fixture).same_storage(&old)); // unchanged 不重编码/换图。
        cycle(&mut fixture, &rx);
        assert_eq!(binding(&fixture)["revision"], 6);
        assert_eq!(fixture.app.documents.len(), count);
        let doc = fixture
            .app
            .documents
            .iter()
            .find(|d| d.state.unit_id.as_str() == id)
            .unwrap();
        let mut current = RECT::default();
        GetWindowRect(doc.window.0, &mut current).unwrap();
        assert_eq!(current, rect);
        assert!(doc.state.execution_results.borrow().ports().is_empty());
        {
            let mut canvas = doc.state.canvas.borrow_mut();
            assert_eq!(canvas.normal_opacity, 0.6);
            assert_eq!(canvas.editor.elements().len(), 1);
            let base = canvas.editor.layers().unwrap().base.clone();
            assert!(canvas.editor.undo());
            assert!(canvas.editor.layers().unwrap().base.same_storage(&base));
            assert!(canvas.editor.redo());
            assert_eq!(
                (
                    canvas.minified_source.as_ref().unwrap().image().width,
                    canvas.minified_source.as_ref().unwrap().image().height
                ),
                (3, 1)
            );
        }
        cycle(&mut fixture, &rx);
        joined(&mut fixture);
        assert_eq!(binding(&fixture)["stopped"], true);
        if offline {
            assert_eq!(binding(&fixture)["stopReason"], "projection_peer_revoked");
            assert_eq!(
                binding(&fixture)["offlineTransport"]["origin"],
                fixture.origin
            );
            assert_eq!(
                binding(&fixture)["otterNativeReceive"]["peerId"],
                crate::projection_delivery::tests::offline::PEER
            );
        }
        assert_eq!(fixture.journal(), journal);
        assert!(!fixture.app.projection_cleanup.pending_local());
        fixture
            .app
            .start_checkpoint_with(&mut |_, _, message, warning| assert!(!warning, "{message}"));
        checkpoint(&mut fixture.app);
        super::fixture::reopen_workspace(&mut fixture);
        assert_eq!(binding(&fixture)["revision"], 6);
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
        assert_eq!(canvas.editor.elements().len(), 1);
        assert_eq!(canvas.normal_opacity, 0.6);
        assert!(fixture
            .app
            .documents
            .iter()
            .find(|d| d.state.unit_id.as_str() == id)
            .unwrap()
            .state
            .execution_results
            .borrow()
            .ports()
            .is_empty());
        assert!(fixture
            .app
            .projection_receive
            .deadline(crate::desktop::surface_windows::now_ms().unwrap())
            .is_none());
    }
}
#[test]
fn projection_offline_sync_unchanged_changed_peer_revoked_preserves_edits_and_reopens() {
    unsafe { run_routed(true) }
}
#[test]
fn projection_sync_late_edit_discards_completed_frame_and_holds_original_permit_until_join() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = Fixture::new();
        saved(&mut fixture);
        let old = source(&fixture);
        let rx = sync_peer(&mut fixture, vec![1]);
        completed(&mut fixture, &rx);
        let mut retry = fixture.ticket().retry();
        assert_eq!(
            retry.activate(),
            Err(crate::projection_delivery::journal::Error::Busy)
        );
        let doc = fixture
            .app
            .documents
            .iter()
            .find(|d| d.state.unit_id.as_str() == fixture.ticket().unit())
            .unwrap();
        {
            let mut canvas = doc.state.canvas.borrow_mut();
            canvas
                .editor
                .begin(
                    crate::editor::Tool::Brush,
                    crate::viewport::Point { x: 0.5, y: 0.5 },
                )
                .unwrap();
            assert!(canvas.editor.commit());
        }
        poll(&mut fixture);
        joined(&mut fixture);
        assert!(source(&fixture).same_storage(&old));
        assert_eq!(binding(&fixture)["revision"], 5);
        retry.activate().unwrap();
        drop(retry);
        assert!(!fixture.app.projection_cleanup.pending_local());
    }
}
#[test]
fn projection_sync_terminal_is_read_even_when_destructive_edit_cannot_be_rebased() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = Fixture::new();
        saved(&mut fixture);
        let doc = fixture
            .app
            .documents
            .iter()
            .find(|d| d.state.unit_id.as_str() == fixture.ticket().unit())
            .unwrap();
        let base = {
            let mut canvas = doc.state.canvas.borrow_mut();
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
            canvas.editor.layers().unwrap().base.clone()
        };
        let rx = sync_peer(&mut fixture, vec![4, 2]);
        cycle(&mut fixture, &rx);
        assert_eq!(binding(&fixture)["revision"], 5);
        cycle(&mut fixture, &rx);
        joined(&mut fixture);
        assert_eq!(binding(&fixture)["stopped"], true);
        let doc = fixture
            .app
            .documents
            .iter()
            .find(|d| d.state.unit_id.as_str() == fixture.ticket().unit())
            .unwrap();
        assert!(doc
            .state
            .canvas
            .borrow()
            .editor
            .layers()
            .unwrap()
            .base
            .same_storage(&base));
    }
}
