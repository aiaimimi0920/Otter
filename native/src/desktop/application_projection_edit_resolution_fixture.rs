//! 冲突恢复的真实TLS对端；每步独立核固定路由、认证、操作和像素坐标，不使用生产合并生成预期。
use super::*;
pub(super) fn document(fixture: &Fixture, revision: u64, two_way: bool, x: f64) -> Value {
    let mut value = serde_json::to_value(snapshot(fixture).document()).unwrap();
    value["revision"] = json!(revision);
    value["receiptCount"] = json!(revision - 1);
    if (snapshot(fixture).document().mode() == edit::Mode::TwoWay) != two_way {
        value["modeRevision"] = json!(revision);
    }
    value["mode"] = json!(if two_way { "two_way" } else { "one_way" });
    value["objects"]["remote"]["revision"] = json!(revision);
    value["objects"]["remote"]["value"]["x"] = json!(x);
    value
}
pub(super) struct Step {
    operation: &'static str,
    status: u16,
    value: Value,
    x: Option<f64>,
}
pub(super) fn read(value: Value) -> Step {
    Step {
        operation: "read",
        status: 200,
        value,
        x: None,
    }
}
pub(super) fn apply(value: Value, x: f64) -> Step {
    Step {
        operation: "apply",
        status: 200,
        value,
        x: Some(x),
    }
}
pub(super) fn rejected(status: u16, code: &str, operation: &'static str) -> Step {
    Step {
        operation,
        status,
        value: json!({"error":{"code":code}}),
        x: None,
    }
}
pub(super) fn server(
    fixture: &mut Fixture,
    steps: Vec<Step>,
    renew: bool,
) -> mpsc::Receiver<Value> {
    let listener = fixture.listener.take().unwrap();
    let tls = fixture.tls.clone();
    let offline = fixture.offline;
    let (tx, rx) = mpsc::sync_channel(8);
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
                    reply(&mut stream, 200, &serde_json::to_vec(&json!({"deviceId":"device-old","token":"receive-owner-token","expiresAtMs":now+60_000})).unwrap());
                }
            }
        }
        for step in steps {
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
            assert_eq!(body["operation"], step.operation);
            if let Some(x) = step.x {
                assert_eq!(body["changes"].as_array().unwrap().len(), 1);
                assert_eq!(body["changes"][0]["objectId"], "remote");
                assert_eq!(body["changes"][0]["value"]["x"].as_f64(), Some(x));
            }
            reply(
                &mut stream,
                step.status,
                &serde_json::to_vec(&step.value).unwrap(),
            );
            tx.send(body).unwrap();
        }
        listener
    }));
    rx
}
pub(super) unsafe fn choose_resolution(
    fixture: &mut Fixture,
    keep_local: bool,
) -> windows::core::Result<()> {
    let doc = fixture
        .app
        .documents
        .iter()
        .find(|doc| doc.state.unit_id.as_str() == fixture.ticket().unit())
        .unwrap();
    crate::desktop::device_authorization_ui::execute(
        doc.window.0,
        &doc.state,
        if keep_local {
            crate::desktop::device_authorization_ui::editing::KEEP_LOCAL
        } else {
            crate::desktop::device_authorization_ui::editing::USE_REMOTE
        },
    )?;
    submit_fixture::requests(fixture);
    Ok(())
}
pub(super) unsafe fn conflicted(offline: bool, two_way: bool) -> Fixture {
    let mut fixture = if two_way {
        accepted_two_way(offline)
    } else {
        accepted(offline)
    };
    shift(&fixture, 0.25);
    let mut response = wire(&fixture.origin, 2, offline);
    response["snapshot"] = Value::Null;
    response["editing"]["mode"] = json!(if two_way { "two_way" } else { "one_way" });
    response["editing"]["objects"]["remote"]["value"]["x"] = json!(0.25);
    let rx = peer(&mut fixture, vec![response]);
    cycle(&mut fixture, &rx);
    joined(&mut fixture);
    assert!(snapshot(&fixture).conflict().is_some());
    fixture
}
pub(super) fn x(fixture: &Fixture) -> f64 {
    snapshot(fixture).editor_view().unwrap().basis_objects()["remote"]["x"]
        .as_f64()
        .unwrap()
}
