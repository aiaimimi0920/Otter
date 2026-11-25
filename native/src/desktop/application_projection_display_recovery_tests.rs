//! Explicit retry of saved receipts/actual profile reopening; no reaccept, and old presentation never proves a new frame.
use super::*;
#[path = "application_projection_display_recovery_activity_tests.rs"]
mod activity;
#[path = "application_projection_display_recovery_checkpoint_tests.rs"]
mod checkpoint_tests;
#[path = "application_projection_sync_tests.rs"]
mod continuous;
#[path = "application_projection_display_recovery_fixture.rs"]
mod fixture;
#[path = "application_projection_display_recovery_guard_tests.rs"]
mod guards;
#[path = "application_projection_paint_trace_tests.rs"]
mod paint_trace;
use fixture::{choose, joined, peer, poll, reopen, saved, settle};

unsafe fn run(mode: u8, restart: bool) {
    run_routed(mode, restart, false)
}
unsafe fn run_routed(mode: u8, restart: bool, offline: bool) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let mut fixture = if offline {
        Fixture::offline(false)
    } else {
        Fixture::new()
    };
    saved(&mut fixture);
    if restart {
        let doc = fixture
            .app
            .documents
            .iter()
            .find(|doc| doc.state.unit_id.as_str() == fixture.ticket().unit())
            .unwrap();
        SetWindowPos(
            doc.window.0,
            None,
            0,
            0,
            160,
            80,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        fixture
            .app
            .start_checkpoint_with(&mut |_, _, message, warning| assert!(!warning, "{message}"));
        checkpoint(&mut fixture.app);
    }
    let journal = fixture.journal();
    let disk = std::fs::read(fixture.files.root().join("workspace.json")).unwrap();
    let intent = fixture.ticket().intent_id().to_owned();
    peer(&mut fixture, mode, restart);
    if restart {
        reopen(&mut fixture);
        let doc = fixture
            .app
            .documents
            .iter()
            .find(|doc| doc.state.unit_id.as_str() == fixture.ticket().unit())
            .unwrap();
        let canvas = doc.state.canvas.borrow();
        let source = canvas.minified_source.as_ref().unwrap().image();
        assert_eq!((source.width, source.height), (2, 1));
        let display = canvas.image.as_ref().unwrap();
        assert_eq!((display.width, display.height), (160, 80));
    }
    assert_eq!(fixture.ticket().intent_id(), intent);
    assert!(!fixture.app.projection_receive.busy()); // Startup/authorization does not automatically show/read/displayed.
    let trace = (mode == 6).then(|| {
        paint_trace::Trace::new(
            fixture
                .app
                .documents
                .iter()
                .find(|doc| doc.state.unit_id.as_str() == fixture.ticket().unit())
                .unwrap()
                .window
                .0,
        )
    });
    let count = fixture.app.documents.len();
    choose(&mut fixture);
    settle(&mut fixture);
    if let Some(trace) = &trace {
        eprintln!("projection-paint-trace-first: {}", trace.describe());
    }
    joined(&mut fixture);
    assert_eq!(fixture.app.documents.len(), count); // No extra Unit, append, or image replacement.
    assert!(fixture.unit_present());
    assert_eq!(fixture.journal(), journal);
    assert_eq!(
        std::fs::read(fixture.files.root().join("workspace.json")).unwrap(),
        disk
    );
    assert!(!fixture.app.projection_cleanup.pending_local());
    assert_eq!(
        fixture.app.authorization.borrow().projection.status,
        if mode == 0 {
            crate::ui_text::text("app-projection-presented")
        } else {
            crate::ui_text::text("app-projection-reception-incomplete")
        }
    );
    if mode == 0 {
        let doc = fixture
            .app
            .documents
            .iter()
            .find(|doc| doc.state.unit_id.as_str() == fixture.ticket().unit())
            .unwrap();
        assert!(IsWindowVisible(doc.window.0).as_bool());
        assert!(doc.state.canvas.borrow().confirmed_paints > 0);
    }
    let mut ticket = fixture.ticket().retry();
    ticket.activate().unwrap();
    drop(ticket);
    // The next explicit operation may idempotently settle lost/negative replies, without recurring timers or background replay.
    if matches!(mode, 5 | 6) {
        peer(&mut fixture, 0, false);
        choose(&mut fixture);
        settle(&mut fixture);
        if let Some(trace) = &trace {
            eprintln!("projection-paint-trace-second: {}", trace.describe());
        }
        // Preserve the presentation-rejection state first, so the peer's displayed timeout cannot obscure main-thread state.
        assert_eq!(
            fixture.app.authorization.borrow().projection.status,
            crate::ui_text::text("app-projection-presented")
        );
        joined(&mut fixture);
        assert_eq!(fixture.journal(), journal);
    }
}
#[test]
fn projection_display_recovery_explicit_saved_menu_reads_original_then_presents_and_reports() {
    unsafe { run(0, false) }
}
#[test]
fn projection_display_recovery_reopens_original_store_and_canonical_workspace_before_explicit_read()
{
    unsafe { run(0, true) }
}
#[test]
fn projection_display_recovery_denied_read_never_reaccepts_or_reports() {
    unsafe { run(1, false) }
}
#[test]
fn projection_display_recovery_new_remote_revision_is_not_the_saved_binding() {
    unsafe { run(2, false) }
}
#[test]
fn projection_display_recovery_mismatched_png_digest_cannot_authorize_report() {
    unsafe { run(3, false) }
}
#[test]
fn projection_display_recovery_wrong_receiver_device_never_reports() {
    unsafe { run(7, false) }
}
#[test]
fn projection_display_recovery_negative_receipt_stays_unconfirmed_then_explicitly_retries() {
    unsafe { run(5, false) }
}
#[test]
fn projection_display_recovery_lost_reply_reopens_and_can_explicitly_retry_idempotently() {
    unsafe { run(6, true) }
}
#[test]
fn projection_offline_display_recovery_reopens_store_and_workspace_then_reads_original_route() {
    unsafe { run_routed(0, true, true) }
}
#[test]
fn projection_offline_display_recovery_lost_reply_reopens_and_explicitly_retries() {
    unsafe { run_routed(6, true, true) }
}
