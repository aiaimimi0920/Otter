//! 原Runtime/真实TLS/自有已paint HWND：在途heartbeat不授输入，严格成功ACK后才开放。
use super::*;
use std::sync::mpsc;

#[test]
fn actual_presenter_opens_input_only_after_successful_heartbeat_and_stop_retires_it() {
    run(true);
}
#[test]
fn actual_presenter_rejects_invalid_heartbeat_ack_without_opening_input() {
    run(false);
}
fn run(valid: bool) {
    use windows::Win32::System::Com::*;
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
        .ok()
        .unwrap();
    struct Com;
    impl Drop for Com {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }
    let _com = Com;
    let (listener, client, config) = fixture();
    let origin = client.origin().to_owned();
    let mut grant = Grant::new(session(&origin, u64::MAX));
    let lease = grant.lease(&origin, 0).unwrap();
    let images = grant.lease(&origin, 0).unwrap();
    let art = grant.lease(&origin, 0).unwrap();
    let mut output = crate::tile_tests::output();
    (output.x, output.y) = (-30000, -30000);
    let endpoint = Endpoint::image("device-1", &output);
    let (id, resource) = image();
    let (sent, arrived) = mpsc::sync_channel(1);
    let (release, go) = mpsc::sync_channel(0);
    let peer = std::thread::spawn(move || {
        let end = Instant::now() + Duration::from_secs(25);
        let mut held = false;
        loop {
            assert!(Instant::now() < end, "fixture deadline");
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            let response = if head.starts_with("GET /v1/walls/state ") {
                let mut value = crate::tile_tests::catalog(&endpoint, 2);
                value["layouts"][0]["placements"] = serde_json::json!([{
                    "placementId":"p","source":{"kind":"image","id":id},
                    "rect":{"x":0,"y":0,"width":2,"height":2},
                    "sourceCrop":{"x":0,"y":0,"width":1,"height":1},"zIndex":0,"interactive":false}]);
                value
            } else if head.starts_with("POST /v1/walls/connect ") {
                serde_json::json!({"protocolVersion":wire::PROTOCOL,"leaseId":"presenter","leaseTtlMs":15000})
            } else if head.starts_with("POST /v1/walls/images/read ") {
                assert_eq!(body["resourceId"], id);
                resource.clone()
            } else if head.starts_with("POST /v1/walls/heartbeat ") {
                if !held && body["appliedRevision"] == 2 {
                    held = true;
                    sent.send(()).unwrap();
                    go.recv_timeout(Duration::from_secs(2)).unwrap();
                    if !valid {
                        reply(
                            &mut stream,
                            200,
                            br#"{"protocolVersion":"loom.wall.v1","accepted":false}"#,
                        );
                        break;
                    }
                }
                serde_json::json!({"protocolVersion":wire::PROTOCOL,"accepted":true})
            } else if head.starts_with("POST /v1/walls/disconnect ") {
                assert!(held);
                reply(
                    &mut stream,
                    200,
                    br#"{"protocolVersion":"loom.wall.v1","accepted":true}"#,
                );
                break;
            } else {
                panic!("unexpected {}", head.lines().next().unwrap());
            };
            reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
        }
    });
    let runtime = Runtime::test_start(client, lease, images, art, output.clone()).unwrap();
    let window = crate::desktop::tile_test_window(runtime.shared.clone(), &output);
    let end = Instant::now() + Duration::from_secs(22);
    loop {
        assert!(Instant::now() < end, "{}", runtime.status());
        window.tick().unwrap();
        crate::desktop::tile_test_pump();
        match arrived.try_recv() {
            Ok(()) => break,
            Err(mpsc::TryRecvError::Empty) => (),
            Err(error) => panic!("heartbeat fixture ended: {error}"),
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let key = {
        let state = runtime.shared.lock();
        let key = state.desired.clone().unwrap();
        assert!(
            state.presented.is_some(),
            "a real window receipt already exists"
        );
        assert!(
            !crate::tile_shared::admission::permits(&state, &key),
            "HTTP still in flight"
        );
        key
    };
    release.send(()).unwrap();
    loop {
        assert!(Instant::now() < end, "{}", runtime.status());
        if valid {
            if crate::tile_shared::admission::permits(&runtime.shared.lock(), &key) {
                break;
            }
            assert!(
                !runtime.finished(),
                "successful heartbeat must keep presenter alive"
            );
        } else {
            assert!(!crate::tile_shared::admission::permits(
                &runtime.shared.lock(),
                &key
            ));
            if runtime.finished() {
                break;
            }
        }
        crate::desktop::tile_test_pump();
        std::thread::sleep(Duration::from_millis(10));
    }
    runtime.request_stop();
    while !runtime.finished() {
        assert!(Instant::now() < end);
        crate::desktop::tile_test_pump();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(runtime.shared.lock().input_applied.is_none());
    window.hide();
    drop(window);
    drop(runtime);
    drop(grant);
    peer.join().unwrap();
}
