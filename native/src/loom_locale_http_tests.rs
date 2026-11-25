//! Real TLS requests prove locale transport authentication and response identity fences.
use super::*;
use crate::loom_locale_protocol::{Receipt, Report};
use serde_json::{json, Value};

fn response(request: Value) -> Value {
    json!({
        "loomId":"loom-fixture", "deviceId":"device-1",
        "locale":{
            "receiverGeneration":"generation-1", "language":"en", "revision":0,
            "supportedLanguages":["en","zh-Hans"], "active":true, "request":request
        }
    })
}

#[test]
fn locale_https_reports_and_acknowledges_with_device_auth_and_unique_nonces() {
    let (listener, client, config) = fixture();
    let credential = session(&client.origin, u64::MAX);
    let peer = std::thread::spawn(move || {
        let mut nonces = std::collections::HashSet::new();
        for index in 0..2 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            assert!(head.starts_with("POST /v1/otter/locale-report HTTP/1.1\r\n"));
            let headers = head.to_ascii_lowercase();
            assert!(headers.contains("authorization: device fixture-secret\r\n"));
            assert!(!headers.contains("bearer "));
            let nonce = headers
                .lines()
                .find_map(|line| line.strip_prefix("x-loom-device-nonce: "))
                .unwrap();
            assert!(nonces.insert(nonce.to_owned()));
            assert_eq!(body["receiverId"], "receiver-1");
            assert_eq!(body["supportedLanguages"], json!(["en", "zh-Hans"]));
            assert!(body.get("deviceId").is_none());
            let value = if index == 0 {
                assert!(body.get("receiverGeneration").is_none());
                response(json!({
                    "requestId":"request-1", "expectedRevision":0, "language":"zh-Hans",
                    "receiverGeneration":"generation-1", "expiresAt":15000, "validForMs":15000
                }))
            } else {
                assert_eq!(body["receiverGeneration"], "generation-1");
                assert_eq!(
                    body["result"],
                    json!({
                        "requestId":"request-1", "status":"applied", "code":"applied", "appliedRevision":1
                    })
                );
                let mut value = response(Value::Null);
                value["locale"]["language"] = json!("zh-Hans");
                value["locale"]["revision"] = json!(1);
                value
            };
            reply(&mut stream, 200, &serde_json::to_vec(&value).unwrap());
        }
    });
    let cancel = Cancel::new();
    let control = Control::new(&client, &credential, &cancel);
    let mut report = Report {
        receiver_id: "receiver-1",
        receiver_generation: None,
        language: "en",
        revision: 0,
        supported_languages: ["en", "zh-Hans"],
        result: None,
    };
    let snapshot = control.report_locale("loom-fixture", &report).unwrap();
    assert_eq!(
        snapshot.request.unwrap().language().unwrap().code(),
        "zh-Hans"
    );
    let receipt = Receipt {
        request_id: "request-1".into(),
        status: "applied",
        code: "applied",
        applied_revision: Some(1),
    };
    report.receiver_generation = Some("generation-1");
    report.language = "zh-Hans";
    report.revision = 1;
    report.result = Some(&receipt);
    assert!(control
        .report_locale("loom-fixture", &report)
        .unwrap()
        .request
        .is_none());
    peer.join().unwrap();
}

#[test]
fn locale_https_rejects_wrong_loom_device_or_receiver_before_delivery() {
    let (listener, client, config) = fixture();
    let credential = session(&client.origin, u64::MAX);
    let peer = std::thread::spawn(move || {
        for field in ["loomId", "deviceId", "receiverGeneration"] {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            request(&mut stream);
            let mut value = response(Value::Null);
            if field == "receiverGeneration" {
                value["locale"][field] = json!("foreign");
            } else {
                value[field] = json!("foreign");
            }
            reply(&mut stream, 200, &serde_json::to_vec(&value).unwrap());
        }
    });
    let cancel = Cancel::new();
    let control = Control::new(&client, &credential, &cancel);
    let report = Report {
        receiver_id: "receiver-1",
        receiver_generation: Some("generation-1"),
        language: "en",
        revision: 0,
        supported_languages: ["en", "zh-Hans"],
        result: None,
    };
    for _ in 0..3 {
        assert!(matches!(
            control.report_locale("loom-fixture", &report),
            Err(LiveError::Input)
        ));
    }
    peer.join().unwrap();
}
