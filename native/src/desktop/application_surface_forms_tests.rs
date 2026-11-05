//! 真实 TLS 续签、挂载和激活后由 Application 自动提交表单；不注入 Outcome 或 ACK。
use super::*;
use windows::Win32::UI::Controls::{EM_REPLACESEL, EM_SETSEL};

#[test]
fn application_forms_poll_owns_dispatch_config_retry_snapshot_and_cleanup() {
    let _com = Apartment::new().unwrap();
    let (files, store, _) = Files::new();
    let (listener, client, server, pem) = fixture_with_pem();
    let origin = format!(
        "https://localhost:{}",
        listener.local_addr().unwrap().port()
    );
    std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
    let peer = std::thread::spawn(move || {
        let descriptor = serde_json::json!({"attachmentId":"attachment:test",
            "instanceId":"instance:test","hookNodeId":"node:test","deviceId":"device-old"});
        for stage in 0..7 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(server.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            if stage >= 2 {
                let lower = head.to_ascii_lowercase();
                assert!(lower.contains("authorization: device configured-token"));
                assert!(lower.contains("x-loom-device-nonce:"));
            }
            let response = match stage {
                0 => {
                    assert!(head.starts_with("POST /v1/device-sessions/challenges "));
                    serde_json::json!({"deviceId":"device-old","challengeId":"id","challenge":"value"})
                }
                1 => {
                    assert!(head.starts_with("POST /v1/device-sessions "));
                    serde_json::json!({"deviceId":"device-old","token":"configured-token","expiresAtMs":u64::MAX})
                }
                2 => {
                    assert!(head.starts_with("POST /v1/surfaces/attach "));
                    assert_eq!(body["artId"], "art:test");
                    assert_eq!(body["hookNodeId"], "node:test");
                    assert_eq!(body["deviceId"], "device-old");
                    assert_eq!(body["persistence"], "temporary");
                    assert_eq!(
                        body["capabilities"],
                        serde_json::to_value(
                            crate::desktop::surface_capabilities::report().unwrap()
                        )
                        .unwrap()
                    );
                    serde_json::json!({"instance":{
                        "descriptor":{"instanceId":"instance:test","generation":1,"artId":"art:test"},
                        "attachments":{"attachment:test":{"descriptor":descriptor,
                            "lifecycle":"mounted","lifecycleRevision":1,
                            "snapshot":{"protocolVersion":"loom.surface.v1","instanceId":"instance:test",
                                "attachmentId":"attachment:test","artId":"art:test","runtime":"declarative","revision":1,
                                "scene":{"id":"field","type":"input","props":{"value":"source"},
                                    "layout":{"width":180,"height":32},"events":{"input":"typing"}}
                            }}}
                    }})
                }
                3 | 6 => {
                    assert!(
                        head.starts_with("POST /v1/surfaces/instances/instance:test/lifecycle ")
                    );
                    let (state, revision) = if stage == 3 {
                        ("active", 2)
                    } else {
                        ("inactive", 3)
                    };
                    assert_eq!(
                        body,
                        serde_json::json!({"protocolVersion":"loom.surface.v1",
                        "instanceId":"instance:test","attachmentId":"attachment:test",
                        "state":state,"revision":revision})
                    );
                    serde_json::json!({"descriptor":descriptor,"lifecycle":state,"lifecycleRevision":revision})
                }
                4 => {
                    assert!(head.starts_with("POST /v1/surfaces/instances/instance:test/events "));
                    assert!(head
                        .to_ascii_lowercase()
                        .contains("authorization: device configured-token"));
                    assert_eq!(body["nodeId"], "field");
                    assert_eq!(body["event"], "input");
                    assert_eq!(body["action"], "typing");
                    assert_eq!(body["baseRevision"], 1);
                    assert_eq!(body["payload"]["value"], "自动提交🙂");
                    serde_json::json!({"protocolVersion":"loom.surface.v1","instanceId":body["instanceId"],
                        "eventId":body["eventId"],"requestId":"request:automatic","accepted":true,"status":"succeeded"})
                }
                _ => {
                    assert!(head.starts_with("GET /v1/surfaces/stream?"));
                    serde_json::json!({"protocolVersion":"loom.surface-stream.v1","next":1,"reset":true,
                    "messages":[{"method":"loom.surface.snapshot","params":{"generation":1,"snapshot":{
                        "protocolVersion":"loom.surface.v1","instanceId":"instance:test",
                        "attachmentId":"attachment:test","revision":2,
                        "scene":{"id":"done","type":"text","props":{"text":"Accepted"}}
                    }}}]})
                }
            };
            reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
        }
        listener
    });
    unsafe {
        let (_class, mut app) = fixture::application();
        app.authorization.borrow_mut().origin = Some(origin.clone());
        app.authorization_owner
            .start(crate::device_authorization_runtime::Task::Renew { client, store })
            .unwrap();
        {
            let handle = app.authorization_owner.wait_handles().next().unwrap();
            assert_eq!(
                windows::Win32::System::Threading::WaitForSingleObject(
                    HANDLE(handle.as_raw_handle()),
                    5000
                ),
                WAIT_OBJECT_0
            );
        }
        assert!(matches!(
            app.authorization_owner.poll().as_slice(),
            [crate::device_authorization_owner::Event::Authorized { .. }]
        ));
        super::super::super::super::surface_start::start_with(
            &origin,
            "art:test".into(),
            "node:test".into(),
            &mut app.authorization_owner,
            &mut app.remote_sessions,
            &app.surface_windows,
            |o| fixture::config(o, files.root()),
        )
        .unwrap();
        fixture::wait(&app);
        let mut activations = 0;
        app.poll_remote_live_with(false, |o| {
            activations += 1;
            fixture::config(o, files.root())
        })
        .unwrap();
        assert_eq!(activations, 1);
        let window = app.surface_windows.iter_mut().next().unwrap();
        assert!(window.is_alive() && !window.is_shown());
        assert!(window.lifecycle().is_none());
        fixture::wait(&app);
        app.poll_remote_live_with(false, |_| panic!("activation was requested twice"))
            .unwrap();
        let window = app.surface_windows.iter_mut().next().unwrap();
        assert!(window.lifecycle().unwrap().confirmed_active());
        assert!(!window.is_shown());
        // Hidden/idle policy must not eagerly prepare network configuration.
        app.poll_remote_live_with(false, |_| panic!("hidden input read config"))
            .unwrap();
        app.poll_remote_live_with(true, |_| panic!("idle input read config"))
            .unwrap();
        let parent = app
            .surface_windows
            .iter_mut()
            .next()
            .unwrap()
            .edit_test_parent();
        let child = FindWindowExW(Some(parent), None, w!("EDIT"), None).unwrap();
        SendMessageW(child, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
        let value = HSTRING::from("自动提交🙂");
        SendMessageW(
            child,
            EM_REPLACESEL,
            Some(WPARAM(1)),
            Some(LPARAM(value.as_ptr() as isize)),
        );
        assert!(app
            .surface_windows
            .iter_mut()
            .next()
            .unwrap()
            .has_pending_click());
        let mut attempts = 0;
        app.poll_remote_live_with(true, |requested| {
            assert_eq!(requested, origin);
            attempts += 1;
            // Invalid configuration is rejected before consuming the pending input/owner.
            Config::from_lookup("not-an-origin", |_| None)
        })
        .unwrap();
        assert_eq!(attempts, 1);
        assert_eq!(app.remote_sessions.wait_handles().count(), 0);
        assert!(app
            .surface_windows
            .iter_mut()
            .next()
            .unwrap()
            .has_pending_click());
        let mut reads = 0;
        app.poll_remote_live_with(true, |requested| {
            assert_eq!(requested, origin);
            reads += 1;
            fixture::config(requested, files.root())
        })
        .unwrap();
        assert_eq!(reads, 1);
        assert_eq!(app.remote_sessions.wait_handles().count(), 1);
        assert!(app
            .surface_windows
            .iter_mut()
            .next()
            .unwrap()
            .lifecycle()
            .is_none());
        fixture::wait(&app);
        app.poll_remote_live_with(true, |_| panic!("completed input resubmitted"))
            .unwrap();
        assert_eq!(
            app.surface_windows
                .iter_mut()
                .next()
                .unwrap()
                .snapshot_revision(),
            2
        );
        assert!(!IsWindow(Some(child)).as_bool());
        let receipt = app.remote_sessions.take_surface_batch_receipt().unwrap();
        assert!(matches!(receipt.presentation, Ok(Some(2))));
        assert_eq!(receipt.events.len(), 1);
        for _ in 0..3 {
            app.poll_remote_live_with(true, |_| panic!("idle refresh repeated request"))
                .unwrap();
        }
        assert_eq!(app.remote_sessions.wait_handles().count(), 0);
        app.surface_windows.close_all().unwrap();
        app.poll_remote_live_with(false, |o| fixture::config(o, files.root()))
            .unwrap();
        fixture::wait(&app);
        app.poll_remote_live_with(false, |_| panic!("inactive was requested twice"))
            .unwrap();
        assert!(!IsWindow(Some(parent)).as_bool());
        let owner = app
            .surface_windows
            .iter_mut()
            .next()
            .unwrap()
            .lifecycle()
            .unwrap();
        assert_eq!(
            owner.binding().lifecycle(),
            crate::surface_live_binding::Lifecycle::Inactive
        );
        assert!(!owner.disposed());
        let listener = peer.join().unwrap();
        assert!(
            matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
        app.authorization_owner.close();
        app.remote_sessions.close();
        assert!(app.remote_sessions.poll_surfaces().is_empty());
        assert!(app.remote_sessions.finished());
    }
}
