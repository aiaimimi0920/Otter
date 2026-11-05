//! 失败 read/解码/像素核验完成未领取时，原 Job 仍独占同 intent 许可及四槽之一。
use super::*;

unsafe fn failed(mode: u8) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let mut fixture = Fixture::new();
    saved(&mut fixture);
    let journal = fixture.journal();
    if mode == 4 {
        let doc = fixture
            .app
            .documents
            .iter()
            .find(|doc| doc.state.unit_id.as_str() == fixture.ticket().unit())
            .unwrap();
        doc.state
            .canvas
            .borrow_mut()
            .replace_image(crate::image_decode::Image {
                width: 1,
                height: 1,
                pixels: vec![0, 0, 0, 255].into(),
            })
            .unwrap();
        fixture
            .app
            .start_checkpoint_with(&mut |_, _, message, warning| assert!(!warning, "{message}"));
        checkpoint(&mut fixture.app);
        poll(&mut fixture);
    }
    peer(&mut fixture, mode, false);
    choose(&mut fixture);
    let handle = fixture.app.projection_receive.wait_handle().unwrap();
    assert_eq!(
        WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000),
        WAIT_OBJECT_0
    );
    joined(&mut fixture);
    let mut retry = fixture.ticket().retry();
    assert_eq!(
        retry.activate(),
        Err(crate::projection_delivery::journal::Error::Busy)
    );
    let slots = (0..3)
        .map(|_| crate::live_relay_control_runtime::Slot::acquire().unwrap())
        .collect::<Vec<_>>();
    assert!(crate::live_relay_control_runtime::Slot::acquire().is_err());
    drop(slots);
    assert!(fixture.app.projection_receive.busy());
    settle(&mut fixture);
    assert_eq!(fixture.journal(), journal);
    assert!(!fixture.app.projection_cleanup.pending_local());
    assert_ne!(
        fixture.app.authorization.borrow().projection.status,
        "投送已呈现"
    );
    retry.activate().unwrap();
}
#[test]
fn projection_display_recovery_failed_http_retains_activity_and_slot_until_ui_reaps() {
    unsafe { failed(1) }
}
#[test]
fn projection_display_recovery_failed_decode_retains_activity_and_slot_until_ui_reaps() {
    unsafe { failed(3) }
}
#[test]
fn projection_display_recovery_failed_pixels_retains_activity_and_slot_until_ui_reaps() {
    unsafe { failed(4) }
}
