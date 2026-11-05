//! 隔离TLS固定wire oracle；确认包含authorizer/完整配置/双游标，错误不盲重放。
use super::*;
use crate::live_relay_http::RequesterCursor;
use crate::{live_relay_trigger_command::Command, live_relay_viewer_identity::Snapshot};
use serde_json::{json, Value};
fn command() -> Command {
    Command::parse(r#"{"bindingId":"b:1","enabled":true,"target":{"surfaceInstanceId":"s:1","surfaceAttachmentId":"a:1","surfaceNodeId":"finish","surfaceEvent":"click","surfaceAction":"finish"},"condition":{"conditionId":"c:1","revision":1,"observationId":"uia:edit","operator":"equals","operand":{"path":"/value/text","value":"done"},"stableForMs":1000,"risingEdge":false,"rearm":true,"minimumConfidence":"exact"}}"#).unwrap()
}
fn snapshot(control: u64, registrations: Vec<Value>) -> Value {
    json!({"session":{"protocolVersion":"loom.live.v1","sessionId":"live:trigger",
        "sourceDeviceId":"source:1","sourceHookId":"hook:1","revision":3,"viewerDevices":["device-1"]},
        "epoch":1,"closed":false,"observations":[],"triggers":registrations,
        "requesterControl":{"deviceId":"device-1","epoch":1,"controlSequence":control,"inputSequence":4}})
}
fn registration() -> Value {
    json!({"binding":{"bindingId":"b:1","observationId":"uia:edit","conditionRevision":1,"authorizedBy":"device-1","enabled":true},
        "condition":{"conditionId":"c:1","revision":1,"observationId":"uia:edit","operator":"equals","operand":{"path":"/value/text","value":"done"},"stableForMs":1000,"risingEdge":false,"rearm":true,"minimumConfidence":"exact"},
        "target":{"surfaceInstanceId":"s:1","surfaceAttachmentId":"a:1","surfaceNodeId":"finish","surfaceEvent":"click","surfaceAction":"finish"},
        "armed":false,"lastMatch":true,"pendingOfflineMatch":false})
}
fn identity() -> crate::live_relay_viewer_identity::Identity {
    Snapshot::parse(
        &serde_json::to_vec(&snapshot(7, vec![])).unwrap(),
        "live:trigger",
    )
    .unwrap()
    .identity()
    .clone()
}
#[test]
fn exact_committed_trigger_recovers_lost_reply_and_requires_full_identity_and_cursors() {
    for scenario in 0..14 {
        let (listener, client, tls) = fixture();
        let session = session(&client.origin, u64::MAX);
        let peer = std::thread::spawn(move || {
            for stage in 0..3 {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(tls.clone()).unwrap(),
                    accept(&listener),
                );
                let (head, body) = request(&mut stream);
                assert!(head
                    .to_ascii_lowercase()
                    .contains("authorization: device fixture-secret"));
                if stage == 1 {
                    assert!(head.starts_with("POST /v1/live/sessions/live:trigger/triggers "));
                    assert_eq!(body["envelope"]["sequence"], 8);
                    assert_eq!(body["envelope"]["messageType"], "trigger_condition");
                    assert_eq!(
                        body["envelope"]["payload"]["operand"],
                        json!({"path":"/value/text","value":"done"})
                    );
                    assert_eq!(body["target"]["surfaceAttachmentId"], "a:1");
                    if scenario == 1 {
                        continue;
                    } // 真正丢失已提交请求的回复，后续只有只读GET。
                    reply(
                        &mut stream,
                        if matches!(scenario, 10 | 11) {
                            500
                        } else if matches!(scenario, 8 | 9) {
                            403
                        } else {
                            200
                        },
                        b"{}",
                    );
                } else {
                    assert!(head.starts_with("GET /v1/live/sessions/live:trigger "));
                    let mut value = if stage == 0 {
                        snapshot(7, vec![])
                    } else {
                        snapshot(8, vec![registration()])
                    };
                    if stage == 2 {
                        match scenario {
                            2 => {
                                value["triggers"][0]["binding"]["authorizedBy"] =
                                    json!("viewer:other")
                            }
                            3 => value["triggers"][0]["target"]["surfaceNodeId"] = json!("other"),
                            4 => {
                                value["triggers"][0]["condition"]["operand"]["value"] =
                                    json!("wrong")
                            }
                            5 => value["requesterControl"]["controlSequence"] = json!(9),
                            6 => value["requesterControl"]["inputSequence"] = json!(5),
                            7 => value = snapshot(7, vec![]),
                            8 => value = snapshot(7, vec![]),
                            9 => value["requesterControl"]["deviceId"] = json!("viewer:other"),
                            10 => value = snapshot(7, vec![]),
                            12 => value["epoch"] = json!(2),
                            13 => value["closed"] = json!(true),
                            _ => {}
                        }
                    }
                    reply(&mut stream, 200, &serde_json::to_vec(&value).unwrap());
                }
            }
        });
        let cancel = Cancel::default();
        let control = Control::new(&client, &session, &cancel);
        let result = control.configure_trigger(
            &identity(),
            RequesterCursor {
                control: 7,
                input: 4,
            },
            &command(),
        );
        match scenario {
            0 | 1 | 11 => assert!(result.unwrap().accepted),
            8 => assert!(!result.unwrap().accepted),
            _ => assert!(result.is_err(), "scenario {scenario}"),
        }
        peer.join().unwrap();
    }
}
#[test]
fn stale_revision_rejects_without_post_and_duplicate_registration_is_not_authority() {
    let (listener, client, tls) = fixture();
    let session = session(&client.origin, u64::MAX);
    let peer = std::thread::spawn(move || {
        let mut stream = StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
        let (head, _) = request(&mut stream);
        assert!(head.starts_with("GET "));
        reply(
            &mut stream,
            200,
            &serde_json::to_vec(&snapshot(7, vec![registration()])).unwrap(),
        );
    });
    let cancel = Cancel::default();
    let control = Control::new(&client, &session, &cancel);
    assert!(
        !control
            .configure_trigger(
                &identity(),
                RequesterCursor {
                    control: 7,
                    input: 4
                },
                &command()
            )
            .unwrap()
            .accepted
    );
    peer.join().unwrap();
    assert!(crate::live_relay_trigger_registration::from_snapshot(
        &serde_json::to_vec(&snapshot(7, vec![registration(), registration()])).unwrap()
    )
    .is_err());
}
