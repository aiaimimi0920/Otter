//! 实际菜单/原Job/TLS/默认Checkpoint夹具；仅使用隔离profile，不操作用户输入或设备。
use super::*;
pub(super) unsafe fn accepted_two_way(offline: bool) -> Fixture {
    let mut fixture = Fixture::with_request(
        if offline {
            "/v1/offline-projections/accept"
        } else {
            "/v1/projections/accept"
        },
        move |stream, origin| {
            let mut value = wire(origin, 1, offline);
            value["editing"]["mode"] = json!("two_way");
            reply(stream, 200, &serde_json::to_vec(&value).unwrap());
        },
    );
    saved(&mut fixture);
    fixture
}
pub(super) fn journal(fixture: &Fixture) -> Value {
    serde_json::from_slice(
        &std::fs::read(fixture.files.root().join("projection-edit.json")).unwrap(),
    )
    .unwrap()
}
pub(super) unsafe fn choose(fixture: &mut Fixture, cancel: bool) {
    let doc = fixture
        .app
        .documents
        .iter()
        .find(|doc| doc.state.unit_id.as_str() == fixture.ticket().unit())
        .unwrap();
    crate::desktop::device_authorization_ui::execute(
        doc.window.0,
        &doc.state,
        if cancel {
            crate::desktop::device_authorization_ui::editing::CANCEL
        } else {
            crate::desktop::device_authorization_ui::editing::SUBMIT
        },
    )
    .unwrap();
    requests(fixture);
}
pub(super) unsafe fn requests(fixture: &mut Fixture) {
    fixture
        .app
        .projection_edit_requests(&mut |origin| config(origin, fixture.files.root()));
}
pub(super) unsafe fn poll_edit(fixture: &mut Fixture) {
    fixture
        .app
        .poll_projection_edit(&mut |origin| config(origin, fixture.files.root()));
}
pub(super) unsafe fn manual_save(fixture: &mut Fixture, cancelled: bool) {
    let path = fixture.files.root().join("manual-collaboration.json");
    fixture.app.request_save_new(path.clone()).unwrap();
    let end = Instant::now() + Duration::from_secs(10);
    let prepared = loop {
        if let Some(prepared) = fixture.app.take_save_candidate().unwrap() {
            break prepared;
        }
        assert!(Instant::now() < end, "collaboration manual save deadline");
        std::thread::yield_now();
    };
    let result = prepared.commit_ready(
        &mut fixture.app,
        &std::sync::atomic::AtomicBool::new(cancelled),
    );
    assert_eq!(result.is_err(), cancelled);
    assert_eq!(path.is_file(), !cancelled);
}
pub(super) unsafe fn completed(fixture: &Fixture) {
    let handle = fixture.app.projection_edit.wait_handle().unwrap();
    assert_eq!(
        WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000),
        WAIT_OBJECT_0
    );
}
pub(super) unsafe fn shift(fixture: &Fixture, fraction: f64) {
    let mut canvas = state(fixture).canvas.borrow_mut();
    let width = canvas.editor.full_logical_frame().unwrap().width;
    assert!(canvas
        .editor
        .translate(
            0,
            Point {
                x: (width * fraction) as f32,
                y: 0.0
            }
        )
        .unwrap());
}
pub(super) fn server(fixture: &mut Fixture, replies: Vec<(u16, f64)>) -> mpsc::Receiver<Value> {
    server_with_renew(fixture, replies, false)
}
pub(super) fn server_with_renew(
    fixture: &mut Fixture,
    replies: Vec<(u16, f64)>,
    renew: bool,
) -> mpsc::Receiver<Value> {
    let listener = fixture.listener.take().unwrap();
    let tls = fixture.tls.clone();
    let mut document = serde_json::to_value(snapshot(fixture).document()).unwrap();
    let offline = fixture.offline;
    let (tx, rx) = mpsc::sync_channel(4);
    fixture.server = Some(std::thread::spawn(move || {
        if renew {
            for index in 0..2 {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(tls.clone()).unwrap(),
                    accept(&listener),
                );
                let (head, _) = request(&mut stream);
                if index == 0 {
                    assert!(head.starts_with("POST /v1/device-sessions/challenges "));
                    reply(
                        &mut stream,
                        200,
                        br#"{"deviceId":"device-old","challengeId":"id","challenge":"value"}"#,
                    );
                } else {
                    assert!(head.starts_with("POST /v1/device-sessions "));
                    let now = crate::desktop::surface_windows::now_ms().unwrap();
                    reply(&mut stream,200,&serde_json::to_vec(&json!({"deviceId":"device-old","token":"receive-owner-token","expiresAtMs":now+60_000})).unwrap());
                }
            }
        }
        for (status, x) in replies {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            assert!(head.starts_with(if offline {
                "POST /v1/offline-projections/edit "
            } else {
                "POST /v1/projections/edit "
            }));
            assert!(head
                .to_ascii_lowercase()
                .contains("authorization: device receive-owner-token\r\n"));
            assert_eq!(body["operation"], "apply");
            assert_eq!(body["changes"].as_array().unwrap().len(), 1);
            assert_eq!(body["changes"][0]["objectId"], "remote");
            assert_eq!(body["changes"][0]["value"]["x"].as_f64(), Some(x));
            if status == 200 {
                let revision = body["baseRevision"].as_u64().unwrap() + 1;
                document["revision"] = json!(revision);
                document["receiptCount"] = json!(revision - 1);
                document["objects"]["remote"]["revision"] = json!(revision);
                document["objects"]["remote"]["value"]["x"] = json!(x);
                reply(&mut stream, status, &serde_json::to_vec(&document).unwrap());
            } else {
                reply(
                    &mut stream,
                    status,
                    br#"{"error":{"code":"temporarily_unavailable"}}"#,
                );
            }
            tx.send(body).unwrap();
        }
        listener
    }));
    rx
}
pub(super) unsafe fn finish(fixture: &mut Fixture) {
    let end = Instant::now() + Duration::from_secs(10);
    while fixture.app.projection_edit.busy() {
        poll_edit(fixture);
        save(fixture);
        assert!(
            Instant::now() < end,
            "collaboration submit completion deadline"
        );
        std::thread::yield_now();
    }
}
