//! 完整包资源复用产品TLS夹具；隐藏Application/屏外非激活输入队列，不冒充前台菜单验收。
use super::*;
use crate as native;
use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
#[path = "../../tests/local_art_runtime/package.rs"]
mod package;
#[path = "../../tests/local_art_runtime/peer.rs"]
mod package_peer;
#[path = "application_local_art_sdk_tests.rs"]
mod sdk_tests;
#[path = "application_local_art_service_failure_tests.rs"]
mod service_failure_tests;
#[path = "application_local_art_service_image_tests.rs"]
mod service_image_tests;
fn root() -> PathBuf {
    let root = PathBuf::from(
        "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-art-application-package",
    )
    .join(format!("{:?}", unsafe {
        windows::Win32::System::Com::CoCreateGuid().unwrap()
    }));
    std::fs::create_dir_all(&root).unwrap();
    eprintln!("art-application-package-evidence: {}", root.display());
    root
}
fn config(peer: &package_peer::Peer, root: &Path) -> Config {
    Config::from_lookup(&peer.origin, |key| match key {
        "OTTER_NATIVE_PROFILE_DIR" => Some(root.as_os_str().to_owned()),
        "OTTER_LOOM_TLS_ORIGIN" => Some(peer.origin.clone().into()),
        "OTTER_LOOM_TLS_CA_FILE" => Some(root.join("ca.pem").into_os_string()),
        _ => None,
    })
    .unwrap()
}
fn trust(peer: &package_peer::Peer) -> crate::art_issuer_trust::Authority {
    crate::art_issuer_trust::Authority::new(
        serde_json::from_value(package::settings(&peer.origin)["artIssuerTrust"].clone()).unwrap(),
    )
    .unwrap()
}
fn pump(app: &mut Application, stage: &str, mut ready: impl FnMut(&Application) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        app.process_local_arts(true);
        if ready(app) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "{stage}: {}",
            app.authorization.borrow().status
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn start(app: &mut Application, peer: &package_peer::Peer, root: &Path) {
    let from = app.documents[0].state.unit_id.to_string();
    app.open_local_art(&peer.origin, package::ART.into(), &from, config(peer, root))
        .unwrap();
    // 首帧先隐藏领取，随后只显示屏外自己的Surface；不抢前台、不发全局输入。
    settle_local(app, (1, 1));
    unsafe {
        let hwnd = app
            .local_arts
            .test_window()
            .unwrap()
            .dialog_owner()
            .unwrap();
        SetWindowPos(
            hwnd,
            None,
            -30000,
            -30000,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
    }
    pump(app, "input enabled", |app| {
        app.local_arts
            .test_window()
            .is_some_and(|w| w.pointer_enabled())
    });
    let window = app.local_arts.test_window().unwrap();
    window.javascript_queue_click_for_test(&window.javascript_first_button_for_test());
}
fn values(app: &Application) -> Option<serde_json::Map<String, serde_json::Value>> {
    app.documents[1]
        .state
        .execution_results
        .borrow()
        .save_value_outputs()
}
unsafe fn saved(app: &mut Application, mode: &str) -> Vec<u8> {
    let values = values(app).unwrap();
    assert_eq!(values["answer"], 42);
    assert_eq!(values["mode"], mode);
    let expected = [
        0, 0, 255, 255, 0, 255, 0, 255, 255, 0, 0, 255, 0, 128, 128, 128,
    ];
    assert_eq!(
        app.documents[1]
            .state
            .execution_results
            .borrow()
            .image_source("output_image")
            .unwrap()
            .image
            .pixels
            .as_ref(),
        expected
    );
    assert!(app.documents[0]
        .state
        .execution_results
        .borrow()
        .save_value_outputs()
        .is_none());
    let (_, snapshot) = app.capture_session_snapshot().unwrap();
    let encoded = snapshot.encode(&AtomicBool::new(false)).unwrap();
    let bytes = encoded.bytes.to_vec();
    let candidate =
        crate::desktop::session_candidate::prepare(bytes.clone(), None, &AtomicBool::new(false))
            .unwrap();
    let outputs = candidate.prepare_results(&AtomicBool::new(false)).unwrap();
    let target = outputs
        .iter()
        .find(|r| r.unit() == &app.documents[1].state.unit_id)
        .unwrap();
    assert_eq!(target.save_value_outputs().unwrap()["mode"], mode);
    assert_eq!(
        target
            .image_source("output_image")
            .unwrap()
            .image
            .pixels
            .as_ref(),
        expected
    );
    bytes
}
fn authorize(app: &mut Application, peer: &package_peer::Peer, root: &Path) {
    app.authorization_owner
        .start(crate::device_authorization_runtime::Task::Configured {
            config: config(peer, root),
            action: crate::device_authorization_config::Action::Renew,
        })
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let events = app.authorization_owner.poll();
        if !events.is_empty() {
            assert!(
                matches!(&events[0],crate::device_authorization_owner::Event::Authorized{origin,..} if origin==&peer.origin)
            );
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn complete_package_application_interaction_service_progress_formal_save_and_offline_reopen() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        let root = root();
        let mut peer = package_peer::Peer::new(&root);
        let mut first = image_app();
        authorize(&mut first, &peer, &root);
        first.art_issuers = trust(&peer);
        let original = first.documents[0]
            .state
            .canvas
            .borrow()
            .image
            .as_ref()
            .unwrap()
            .pixels
            .clone();
        start(&mut first, &peer, &root);
        pump(&mut first, "service progress", |app| {
            app.authorization.borrow().status == "Art 服务执行中 37%"
        });
        assert!(
            values(&first).is_none(),
            "progress must not publish a formal result"
        );
        peer.state.lock().unwrap().release = true;
        pump(&mut first, "formal online output", |app| {
            values(app).is_some()
        });
        let online = saved(&mut first, "online");
        assert!(original.same_storage(
            &first.documents[0]
                .state
                .canvas
                .borrow()
                .image
                .as_ref()
                .unwrap()
                .pixels
        ));
        let target = first.documents[1].state.unit_id.clone();
        first.shutdown();
        assert!(!first.local_arts.busy());
        assert!(first.authorization_owner.finished());
        peer.stop();
        let requests = peer.state.lock().unwrap().requests.clone();
        assert_eq!(
            requests
                .iter()
                .filter(|p| p.as_str() == "/v1/art/local/resource")
                .count(),
            3
        );
        let mut offline = image_app();
        offline.art_issuers = trust(&peer);
        assert!(offline
            .authorization_owner
            .lease(
                &peer.origin,
                crate::desktop::surface_windows::now_ms().unwrap()
            )
            .is_err());
        offline
            .request_restore_input(
                crate::desktop::session_job::Input::Bytes(online),
                None,
                None,
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while offline.restore_job.is_some() {
            offline.process_restore(false).unwrap();
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(offline.documents[1].state.unit_id, target);
        assert_eq!(values(&offline).unwrap()["mode"], "online");
        let node = target.to_string();
        offline
            .open_local_art(
                &peer.origin,
                package::ART.into(),
                &node,
                config(&peer, &root),
            )
            .unwrap();
        settle_local(&mut offline, (1, 1));
        let window = offline.local_arts.test_window().unwrap();
        SetWindowPos(
            window.dialog_owner().unwrap(),
            None,
            -30000,
            -30000,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        pump(&mut offline, "offline input enabled", |app| {
            app.local_arts
                .test_window()
                .is_some_and(|w| w.pointer_enabled())
        });
        let window = offline.local_arts.test_window().unwrap();
        window.javascript_queue_click_for_test(&window.javascript_first_button_for_test());
        pump(&mut offline, "formal offline output", |app| {
            values(app).is_some_and(|v| v["mode"] == "offline")
        });
        let bytes = saved(&mut offline, "offline");
        std::fs::write(root.join("offline-session.json"), bytes).unwrap();
        offline.shutdown();
        assert!(!offline.local_arts.busy());
        assert!(offline.authorization_owner.finished());
        peer.assert_no_offline_connections();
        assert_eq!(peer.state.lock().unwrap().requests, requests);
        std::fs::write(root.join("validated.json"),br#"{"hiddenApplication":true,"inputQueue":true,"packageResources":3,"serviceProgress":37,"formalPixels":true,"saveRestore":true,"offlineDeviceSession":false,"shutdown":true}"#).unwrap();
    }
}
