//! Real sharing buttons verify LAN-invitation command boundaries before and after binding retirement.
use super::*;
use crate::projection_delivery::sender::{
    self, binding,
    tests::{prepare, setup},
};
use serde_json::json;

#[test]
fn lan_invitation_waiting_for_unlink_cannot_be_replaced() {
    let (dir, store) = setup();
    let scope = "a".repeat(64);
    let selection =
        sender::Selection::invitation(&scope, "https://loom.example", "unit-1").unwrap();
    let intent = prepare(&store, &dir, &selection).unwrap();
    let cancel = AtomicBool::new(false);
    let response = sender::Response::parse(
        &serde_json::to_vec(&json!({"envelope":intent.envelope(),"revision":1,
            "digest":intent.digest(),"linked":true,"delivery":null,
            "receiverDeviceId":null,"receiverUnitId":null}))
        .unwrap(),
        &intent,
        false,
        &cancel,
    )
    .unwrap();
    let source: crate::session_source::Source = serde_json::to_vec(&json!({
        "stickers":[{"id":"unit-1","x":0,"y":0,"w":4,"h":4}],"links":[],
        "otterNativeProjectionScope":scope
    }))
    .unwrap()
    .into();
    let active = binding::Binding::prepare(&source, &intent, &response, "LAN", &cancel).unwrap();
    let pending =
        binding::stop::prepare(active.source(), active.binding(), false, &cancel).unwrap();
    let pending = binding::checkpoint::Index::capture(&pending).unwrap();

    unsafe {
        let f = Fixture::new(false, false);
        f.popup.model.switch_scope(false).unwrap();
        f.view.borrow_mut().request.take();
        let button = GetDlgItem(Some(f.hwnd()), 108).unwrap();
        let assert_button = |expected: &str, enabled| {
            let mut text = [0; 32];
            let n = GetWindowTextW(button, &mut text) as usize;
            assert_eq!(String::from_utf16_lossy(&text[..n]), expected);
            assert_eq!(
                windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(button).as_bool(),
                enabled
            );
        };
        {
            let mut view = f.view.borrow_mut();
            view.sender.stop_source = Some(active.source().clone());
            view.sender.bindings = vec![active.binding().clone()];
        }
        f.popup.model.refresh_lan();
        assert_button(crate::ui_text::text("share-unlink"), true);
        f.command(108);
        assert!(matches!(
            f.view.borrow_mut().request.take(),
            Some(Request::StopProjectionSend(_))
        ));

        f.view.borrow_mut().sender.bindings = pending.entries().to_vec();
        f.popup.model.refresh_lan();
        assert_button(crate::ui_text::text("share-unavailable"), false);
        f.command(108);
        assert!(f.view.borrow().request.is_none());

        f.view.borrow_mut().sender.bindings.clear();
        f.popup.model.refresh_lan();
        assert_button(crate::ui_text::text("share-generate"), true);
        f.command(108);
        assert!(matches!(
            f.view.borrow_mut().request.take(),
            Some(Request::ShareProjection(_))
        ));
    }
}
