//! Current online Art proof only; reset/replacement cannot keep an old queued menu selection alive.
use super::*;
use crate::desktop::device_authorization_ui::live;
#[test]
fn local_art_viewer_menu_proof_is_stable_and_stale_selection_cannot_start_network() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        let mut app = app();
        let peer = peer::Peer::new();
        peer.authorize(&mut app.authorization_owner);
        app.art_issuers = peer.authority();
        app.authorization.borrow_mut().origin = Some(peer.origin.clone());
        let node = app.documents[0].state.unit_id.to_string();
        app.start_local_art(
            &peer.origin,
            "vendor/application".into(),
            &node,
            peer.config(),
        )
        .unwrap();
        settle_local(&mut app, (1, 1));
        peer.finish();
        app.settle_live_viewers().unwrap();
        let binding = app.authorization.borrow().live.bindings[0].clone();
        app.settle_live_viewers().unwrap();
        assert!(binding.same_host(&app.authorization.borrow().live.bindings[0]));
        let snapshot=crate::live_relay_viewer_identity::Snapshot::parse(br#"{"session":{"protocolVersion":"loom.live.v1","sessionId":"live:test","sourceDeviceId":"source:1","sourceHookId":"node:1","revision":1,"viewerDevices":[]},"epoch":1,"closed":false}"#,"live:test").unwrap();
        let origin = binding.origin().to_owned();
        app.authorization.borrow_mut().catalog = Some(crate::live_relay_catalog_runtime::Catalog {
            origin: origin.clone(),
            entries: vec![crate::live_relay_http::CatalogEntry {
                identity: snapshot.identity().clone(),
                revision: 1,
                member: false,
            }],
        });
        let selected = live::Selection {
            origin,
            identity: snapshot.identity().clone(),
            binding,
        };
        assert!(live::current(&app.authorization.borrow(), &selected));
        live::select(app.documents[0].window.0, &app.documents[0].state, selected).unwrap();
        app.documents[0].state.group_locked.set(true);
        app.process_authorization_with_projection(|_| {
            panic!("stale native host must not load network configuration")
        });
        assert!(app.remote_sessions.viewers().is_empty());
        assert!(app.authorization.borrow().live.status.contains("已变化"));
        settle_local(&mut app, (0, 0));
    }
}
