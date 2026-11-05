//! 真实Loom LAN/WSS和公开SDK原字节的自动回调→Application正式结果/Save/ACK/停止。
use super::*;
use crate::desktop::composition::relay::window::trigger::{
    local_registration_test, local_stop_test, local_submit_test,
};
use crate::live_relay_viewer_trigger_queue::Phase;
unsafe fn spin(
    c: &context::Context,
    app: &mut Application,
    ready: impl Fn(&mut Application) -> bool,
) {
    c.spin(app, |app| {
        app.process_local_arts(false);
        ready(app)
    });
}
fn value(app: &Application) -> Option<serde_json::Value> {
    app.documents[0]
        .state
        .execution_results
        .borrow()
        .save_value_outputs()?
        .get("data")
        .cloned()
}
#[test]
#[ignore = "explicit real Loom local automatic trigger, signed child and original Results ACK"]
fn local_art_automatic_trigger_real_loom_lan_result_and_stop() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let c = context::Context::load();
    let _com = Apartment::new().unwrap();
    unsafe {
        let (_class, mut app) = fixture::application();
        c.authorize(&mut app);
        app.art_issuers =
            crate::art_issuer_trust::Authority::new(crate::art_issuer_trust::Document {
                issuers: vec![crate::art_issuer_trust::Entry {
                    origin: c.origin.clone(),
                    key_id: c.data["issuer"]["keyId"].as_str().unwrap().into(),
                    public_key: c.data["issuer"]["publicKey"].as_str().unwrap().into(),
                }],
            })
            .unwrap();
        app.request_restore_input(crate::desktop::session_job::Input::Bytes(
            br#"{"documentSchemaVersion":1,"documentRevision":1,"otterNativeCoordinateSpace":{"version":1,"origin":[0,0],"scale":1},"stickers":[{"id":"trigger-art","type":"art","artId":"publisher.test/local-counter","x":0,"y":0,"w":200,"h":120}],"links":[]}"#.to_vec()),None,None).unwrap();
        let restore_deadline = Instant::now() + Duration::from_secs(10);
        while app.restore_job.is_some() {
            app.process_restore(false).unwrap();
            assert!(Instant::now() < restore_deadline);
        }
        let unit = app.documents[0].state.unit_id.to_string();
        let editor = app.documents[0]
            .state
            .canvas
            .borrow()
            .editor
            .document_stamp();
        app.start_local_art(
            &c.origin,
            c.data["artId"].as_str().unwrap().into(),
            &unit,
            c.config(&c.origin).unwrap(),
        )
        .unwrap();
        spin(&c, &mut app, |app| app.local_arts.test_counts() == (1, 1));
        app.settle_live_viewers().unwrap();
        app.authorization.borrow_mut().request = Some(Request::Discover);
        app.process_authorization_with_projection(|origin| c.config(origin));
        spin(&c, &mut app, |app| {
            app.authorization.borrow().catalog.is_some()
        });
        let selected = {
            let view = app.authorization.borrow();
            let identity = view
                .catalog
                .as_ref()
                .unwrap()
                .entries
                .iter()
                .find(|e| e.identity.session() == c.data["session"].as_str().unwrap())
                .unwrap()
                .identity
                .clone();
            live::Selection {
                origin: c.origin.clone(),
                identity,
                binding: view.live.bindings[0].clone(),
            }
        };
        live::select(app.documents[0].window.0, &app.documents[0].state, selected).unwrap();
        app.process_authorization_with_projection(|origin| c.config(origin));
        spin(&c, &mut app, |app| {
            app.remote_sessions
                .viewers()
                .first()
                .is_some_and(|v| v.shown)
        });
        let key = app.remote_sessions.viewers()[0].key.clone();
        spin(&c, &mut app, |app| {
            app.remote_sessions.test_frame(&key).unwrap().is_some()
        });
        assert_eq!(
            app.remote_sessions.test_frame(&key).unwrap().unwrap().1,
            [17, 83, 211, 255]
        );
        let (hwnd, _) = app.remote_sessions.test_viewer_input(&key).unwrap();
        context::Context::offscreen(hwnd);
        let mut condition = crate::art_live_trigger::tests::condition();
        condition.condition_id = "condition:native:local".into();
        condition.observation_id = c.data["observationId"].as_str().unwrap().into();
        condition.operand = serde_json::json!({"path":"/value/text","value":"never"});
        condition.rising_edge = true;
        condition.rearm = false;
        assert!(local_submit_test(hwnd, condition.clone()));
        spin(&c, &mut app, |_| {
            local_registration_test(hwnd).is_some_and(|r| r.0 == Phase::Confirmed && r.3)
        });
        condition.revision = 2;
        condition.operand["value"] = serde_json::json!("value-eight");
        assert!(local_submit_test(hwnd, condition));
        spin(&c, &mut app, |_| {
            local_registration_test(hwnd)
                .is_some_and(|r| r.0 == Phase::Confirmed && r.2.revision == 2 && r.3)
        });
        let (_, scope, condition, _) = local_registration_test(hwnd).unwrap();
        let proofs =
            app.local_arts
                .viewer_proofs(&app.documents, &app.workspace, &app.live_sessions);
        assert_eq!(proofs.len(), 1);
        // 仅留在隔离TEMP供父门认证Inspect；不复制到公开/普通证据输出。
        c.record("local-trigger-private-inspect.json",serde_json::json!({"protocolVersion":crate::art_live_trigger::PROTOCOL,
            "binding":proofs[0].binding(),"grant":proofs[0].test_envelope(),"operation":{"kind":"inspect","scope":scope}}));
        c.record("local-trigger-updated.json",serde_json::json!({"scope":scope,"condition":condition,"signedPackage":true,"wssPixel":[17,83,211,255],"configurationSeam":true}));
        spin(&c, &mut app, |app| value(app).is_some());
        let data = value(&app).unwrap();
        assert_eq!(data["count"], 1);
        assert_eq!(data["observation"]["value"]["text"], "value-eight");
        assert_eq!(data["sessionId"], c.data["session"]);
        assert_eq!(data["epoch"], 1);
        let (_, snapshot) = app.capture_session_snapshot().unwrap();
        let stop = std::sync::atomic::AtomicBool::new(false);
        let bytes = snapshot.encode(&stop).unwrap().bytes.to_vec();
        let restored = crate::desktop::session_candidate::prepare(bytes, None, &stop)
            .unwrap()
            .prepare_results(&stop)
            .unwrap();
        assert_eq!(restored[0].save_value_outputs().unwrap()["data"], data);
        assert!(editor.matches(&app.documents[0].state.canvas.borrow().editor));
        c.record("local-trigger-result.json",serde_json::json!({"data":data,"formalResult":true,"saveRestore":true,"editorUnchanged":true}));
        spin(&c, &mut app, |_| c.root.join("local-trigger-stop").exists());
        assert!(local_stop_test(hwnd));
        spin(&c, &mut app, |_| {
            local_registration_test(hwnd).is_some_and(|r| r.0 == Phase::Confirmed && !r.3)
        });
        c.record(
            "local-trigger-stopped.json",
            serde_json::json!({"stopped":true,"preservedFormal":value(&app).unwrap()==data}),
        );
        spin(&c, &mut app, |_| c.root.join("viewer-stop").exists());
        app.authorization.borrow_mut().request = Some(Request::StopLive(key));
        app.process_authorization_with_projection(|origin| c.config(origin));
        spin(&c, &mut app, |app| app.remote_sessions.viewers().is_empty());
        app.shutdown();
        assert!(app.remote_sessions.finished());
        assert!(!app.local_arts.busy());
        c.record("viewer-complete.json",serde_json::json!({"ownersReclaimed":true,"ordinaryVisibleProduct":false,"physicalCrossDevice":false}));
    }
}
