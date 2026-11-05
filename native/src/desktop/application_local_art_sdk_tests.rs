//! 公开SDK示例原字节经原签名包/真实child/TLS/Application运行；不复制示例业务或浏览器模拟。
use super::*;
unsafe fn offline_reopen(bytes: Vec<u8>, peer: &package_peer::Peer, root: &Path) {
    let mut app = image_app();
    app.art_issuers = trust(peer);
    assert!(app
        .authorization_owner
        .lease(
            &peer.origin,
            crate::desktop::surface_windows::now_ms().unwrap()
        )
        .is_err());
    app.request_restore_input(crate::desktop::session_job::Input::Bytes(bytes), None, None)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.restore_job.is_some() {
        app.process_restore(false).unwrap();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    let target = app.documents[1].state.unit_id.to_string();
    app.open_local_art(
        &peer.origin,
        package::ART.into(),
        &target,
        config(peer, root),
    )
    .unwrap();
    settle_local(&mut app, (1, 1));
    SetWindowPos(
        app.local_arts
            .test_window()
            .unwrap()
            .dialog_owner()
            .unwrap(),
        None,
        -30000,
        -30000,
        0,
        0,
        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
    )
    .unwrap();
    pump(&mut app, "SDK offline input enabled", |app| {
        app.local_arts
            .test_window()
            .is_some_and(|w| w.pointer_enabled())
    });
    let window = app.local_arts.test_window().unwrap();
    window.javascript_queue_click_for_test(&window.javascript_first_button_for_test());
    pump(&mut app, "SDK offline formal output", |app| {
        values(app).is_some_and(|v| v["mode"] == "offline")
    });
    assert!(!saved(&mut app, "offline").is_empty());
    app.shutdown();
    assert!(!app.local_arts.busy());
    assert!(app.authorization_owner.finished());
    peer.assert_no_offline_connections();
}
#[test]
#[ignore = "requires the public native Art SDK example and independently frozen SHA-256"]
fn published_sdk_example_runs_package_service_result_and_save_in_application() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let path = std::path::PathBuf::from(std::env::var_os("OTTER_ART_SDK_EXAMPLE").unwrap());
    assert!(path.is_absolute());
    let source = std::fs::read_to_string(path).unwrap();
    assert_eq!(
        package::digest(source.as_bytes()),
        std::env::var("OTTER_ART_SDK_SHA256").unwrap()
    );
    unsafe {
        for status in [200, 503] {
            let _environment = environment();
            let root = root();
            let mut peer = package_peer::Peer::with_source(&root, &source);
            {
                let mut state = peer.state.lock().unwrap();
                state.submit_status = Some(status);
                state.release = true;
            }
            let mut app = image_app();
            authorize(&mut app, &peer, &root);
            app.art_issuers = trust(&peer);
            let original = app.documents[0]
                .state
                .canvas
                .borrow()
                .image
                .as_ref()
                .unwrap()
                .pixels
                .clone();
            start(&mut app, &peer, &root);
            pump(&mut app, "SDK formal output", |app| values(app).is_some());
            let mode = if status == 200 {
                "online"
            } else {
                "service_unavailable"
            };
            let snapshot = saved(&mut app, mode);
            assert!(!snapshot.is_empty());
            assert!(original.same_storage(
                &app.documents[0]
                    .state
                    .canvas
                    .borrow()
                    .image
                    .as_ref()
                    .unwrap()
                    .pixels
            ));
            app.shutdown();
            assert!(!app.local_arts.busy());
            assert!(app.authorization_owner.finished());
            peer.stop();
            let requests = peer.state.lock().unwrap().requests.clone();
            assert_eq!(
                requests.iter().filter(|p| p.ends_with("/submit")).count(),
                1
            );
            assert_eq!(
                requests.iter().filter(|p| p.ends_with("/cancel")).count(),
                usize::from(status != 200)
            );
            assert!(!requests.iter().any(|p| p.ends_with("/poll")));
            offline_reopen(snapshot, &peer, &root);
            assert_eq!(peer.state.lock().unwrap().requests, requests);
            std::fs::write(
                root.join("sdk-example-validated.json"),
                serde_json::to_vec(&serde_json::json!({
                    "sourceSha256":package::digest(source.as_bytes()),"status":status,"mode":mode,
                    "signedChild":true,"hiddenApplication":true,"formalPixels":true,
                    "saveRestore":true,"sourceUnchanged":true,"shutdown":true,"offlineReopen":true
                }))
                .unwrap(),
            )
            .unwrap();
        }
    }
}
