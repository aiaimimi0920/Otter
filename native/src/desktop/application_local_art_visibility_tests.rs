//! v3 真 TLS→正式像素→目录/覆盖→保存重建→停服离线；均为自有隐藏窗口。
use super::*;
use crate::art_execution_grant::package::{
    ports::{Port, Ports},
    visibility::Visibility,
};
use crate::desktop::link_catalog::Catalog;
fn declarations() -> (Ports, Visibility) {
    (
        Ports {
            inputs: vec![Port {
                name: "原始图片".into(),
                kind: "image".into(),
            }],
            outputs: vec![Port {
                name: "observed".into(),
                kind: "value".into(),
            }],
        },
        Visibility {
            inputs: vec![false],
            outputs: vec![false],
        },
    )
}
fn check(app: &Application, shown: bool) {
    let catalog = Catalog::capture(&app.documents, &app.live_sessions, &app.workspace);
    let endpoint = &catalog.endpoints[1];
    assert_eq!(endpoint.input_defaults, vec![false]);
    assert_eq!(endpoint.output_defaults, vec![false]);
    assert_eq!(endpoint.input_visibility.as_ref(), &[shown]);
    assert_eq!(endpoint.output_visibility.as_ref(), &[false]);
    assert_eq!(app.workspace.links()[0].to_port, "原始图片");
    observed(app, 1); // 隐藏端口不能撤回图、权限或正式像素。
}
#[test]
fn signed_visibility_and_user_override_survive_save_rebuild_and_offline_execution() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        let mut app = red_app();
        let (ports, visibility) = declarations();
        let peer = peer::Peer::with_visibility(READ, ports, visibility);
        let config = peer.config();
        let origin = peer.origin.clone();
        let trust = peer.authority();
        peer.authorize(&mut app.authorization_owner);
        app.art_issuers = peer.authority();
        let source = app.documents[0].state.unit_id.to_string();
        app.open_local_art(&origin, "vendor/application".into(), &source, peer.config())
            .unwrap();
        settle_local(&mut app, (1, 1));
        peer.finish();
        check(&app, false);
        let unit = app.documents[1].state.unit_id.clone();
        let old = Catalog::capture(&app.documents, &app.live_sessions, &app.workspace);
        let override_value = app
            .workspace
            .port_visibility(&unit)
            .edited("原始图片", Some(true))
            .unwrap();
        assert!(app
            .workspace
            .set_port_visibility(&unit, override_value)
            .unwrap());
        assert!(!old.matches(&app.documents, &app.live_sessions, &app.workspace));
        check(&app, true);
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let (_, snapshot) = app.capture_session_snapshot().unwrap();
        let encoded = snapshot.encode(&cancel).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&encoded.bytes).unwrap();
        assert_eq!(
            value["stickers"][1]["otterNativeLocalArtPorts"]["visibility"]["inputs"],
            serde_json::json!([false])
        );
        app.shutdown();
        let mut reopened = red_app();
        restored(&mut reopened, encoded.bytes.to_vec());
        check(&reopened, true);
        reopened.art_issuers = trust;
        reopened
            .open_local_art(&origin, "vendor/application".into(), unit.as_str(), config)
            .unwrap();
        settle_local(&mut reopened, (1, 1));
        check(&reopened, true);
        let reset = reopened
            .workspace
            .port_visibility(&unit)
            .edited("原始图片", None)
            .unwrap();
        reopened
            .workspace
            .set_port_visibility(&unit, reset)
            .unwrap();
        check(&reopened, false);
        let (_, snapshot) = reopened.capture_session_snapshot().unwrap();
        let saved = snapshot.encode(&cancel).unwrap();
        reopened.shutdown();
        let mut final_app = red_app();
        restored(&mut final_app, saved.bytes.to_vec());
        check(&final_app, false);
        final_app.shutdown();
    }
}
#[test]
fn replacing_local_defaults_invalidates_old_catalog_without_changing_topology_or_override() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        let mut app = red_app();
        let (ports, visibility) = declarations();
        let peer = peer::Peer::with_visibility(READ, ports, visibility);
        peer.authorize(&mut app.authorization_owner);
        app.art_issuers = peer.authority();
        let source = app.documents[0].state.unit_id.to_string();
        app.open_local_art(
            &peer.origin,
            "vendor/application".into(),
            &source,
            peer.config(),
        )
        .unwrap();
        settle_local(&mut app, (1, 1));
        peer.finish();
        let old = Catalog::capture(&app.documents, &app.live_sessions, &app.workspace);
        let topology = app.workspace.topology_stamp();
        let config = app.documents[1].state.local_art_id.as_ref().unwrap();
        let mut next = config.ports.borrow().as_deref().unwrap().clone();
        next.visibility.as_mut().unwrap().inputs[0] = true;
        *config.ports.borrow_mut() = Some(std::sync::Arc::new(next));
        assert!(!old.matches(&app.documents, &app.live_sessions, &app.workspace));
        assert!(topology.matches(&app.workspace));
        let catalog = Catalog::capture(&app.documents, &app.live_sessions, &app.workspace);
        assert_eq!(catalog.endpoints[1].input_defaults, vec![true]);
        assert!(app
            .workspace
            .port_visibility(&app.documents[1].state.unit_id)
            .is_empty());
        app.shutdown();
    }
}
