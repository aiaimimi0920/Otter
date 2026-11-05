use super::*;
use crate::live_relay_http::CatalogEntry;
use crate::surface_live_binding::Binding;
use serde_json::json;
fn view() -> AuthorizationView {
    let origin = "https://loom.example";
    let binding = Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:1","generation":1},"attachments":{"attach:1":{"descriptor":{"attachmentId":"attach:1","instanceId":"instance:1","hookNodeId":"node:1","deviceId":"viewer:1"},"lifecycle":"active","lifecycleRevision":2}}}}"#, origin,"viewer:1","node:1").unwrap();
    let mut view = AuthorizationView {
        origin: Some(origin.into()),
        ..Default::default()
    };
    view.live.bindings.push(ViewerHost::Surface(binding));
    view.catalog = Some(crate::live_relay_catalog_runtime::Catalog {
        origin: origin.into(),
        entries: vec![entry("live:1")],
    });
    view
}
fn entry(session: &str) -> CatalogEntry {
    let snapshot = crate::live_relay_viewer_identity::Snapshot::parse(&serde_json::to_vec(&json!({"session":{"protocolVersion":"loom.live.v1","sessionId":session,"sourceDeviceId":"source:1","sourceHookId":"hook:1","revision":1,"viewerDevices":[]},"epoch":1,"closed":false})).unwrap(),session).unwrap();
    CatalogEntry {
        identity: snapshot.identity().clone(),
        revision: 1,
        member: false,
    }
}
#[test]
fn immutable_choices_reject_changed_catalog_binding_or_authorization() {
    let mut view = view();
    let (_menu, mut selected) = unsafe { choices(&view, &view.live.bindings[0]).unwrap() };
    let selected = selected.pop().unwrap();
    assert!(current(&view, &selected));
    view.catalog.as_mut().unwrap().entries[0] = entry("live:2");
    assert!(!current(&view, &selected));
    view.catalog.as_mut().unwrap().entries[0] = entry("live:1");
    view.live.bindings.clear();
    assert!(!current(&view, &selected));
    view.live.bindings.push(selected.binding.clone());
    for mode in 0..3 {
        view.busy = mode == 0;
        view.catalog_busy = mode == 1;
        view.closed = mode == 2;
        assert!(!current(&view, &selected));
        assert!(!available(&view));
    }
}
#[test]
fn full_catalog_fits_common_menu_budget_and_refuses_invalid_binding() {
    let mut view = view();
    view.catalog.as_mut().unwrap().entries = (0..MAX_CATALOG)
        .map(|id| entry(&format!("live:{id}")))
        .collect();
    let (menu, selected) = unsafe { choices(&view, &view.live.bindings[0]).unwrap() };
    assert_eq!(selected.len(), MAX_CATALOG);
    assert_eq!(
        crate::native_menu::snapshot(menu.0).unwrap().len(),
        MAX_CATALOG
    );
    view.catalog
        .as_mut()
        .unwrap()
        .entries
        .push(entry("live:overflow"));
    assert!(unsafe { choices(&view, &view.live.bindings[0]) }.is_err());
    view.catalog.as_mut().unwrap().entries.pop();
    let binding = view.live.bindings.remove(0);
    assert!(unsafe { choices(&view, &binding) }.is_err());
}
#[test]
fn watch_disabled_without_real_surface_and_command_range_is_bounded() {
    let mut view = view();
    assert!(available(&view));
    view.live.bindings.clear();
    assert!(!available(&view));
    let menu = Menu(unsafe { CreatePopupMenu().unwrap() });
    unsafe {
        append_menu(menu.0, &view).unwrap();
    }
    assert!(!crate::native_menu::snapshot(menu.0).unwrap()[0].enabled);
    assert!(!handles(WATCH - 1));
    assert!(handles(WATCH));
    assert!(handles(STOP_BASE + MAX_VIEWERS - 1));
    assert!(!handles(STOP_BASE + MAX_VIEWERS));
}
