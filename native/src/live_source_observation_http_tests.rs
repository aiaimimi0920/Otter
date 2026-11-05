//! 真实隔离TLS；固定wire oracle核双序号、错ack、未知提交只读对账与最终close。
use super::*;
use crate::{
    live_relay_http::source::{ObservationSubmission, Publication},
    live_relay_observation::Observation,
};
use serde_json::{json, Value};
#[path = "live_source_observation_reconcile_tests.rs"]
mod reconcile;
fn observation() -> Observation {
    Observation::parse(br#"{"observationId":"uia:edit","sequence":1,"state":"observing","source":"ui_automation","confidence":"exact","observedAtMs":100,"locator":{"automationId":"101","controlType":"Edit","ancestorPath":[]},"value":{"value":{"text":"seven","readOnly":false}}}"#).unwrap()
}
#[test]
fn source_observation_ack_loss_reconciles_exact_payload_and_close_uses_current_actor() {
    let (listener, client, tls) = fixture();
    let session = session(&client.origin, u64::MAX);
    let mut publication =
        Publication::new("hook:test", [1, 1], Some(("window:test".into(), 1))).unwrap();
    publication.allow_observation().unwrap();
    let expected = serde_json::to_value(observation()).unwrap();
    let peer = std::thread::spawn(move || {
        let mut snapshot = Value::Null;
        for stage in 0..5 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            match stage {
                0 => {
                    snapshot = json!({"session":body["envelope"]["payload"]["session"],"epoch":1,"closed":false,"observations":[],"requesterControl":{"deviceId":"device-1","epoch":1,"controlSequence":1}});
                    assert_eq!(
                        snapshot["session"]["observationCapabilities"],
                        json!(["uia_tree"])
                    );
                    reply(&mut stream, 201, &serde_json::to_vec(&snapshot).unwrap());
                }
                1 => {
                    assert!(head.contains("/observations HTTP/1.1"));
                    assert_eq!(body["envelope"]["sequence"], 2);
                    assert_eq!(body["envelope"]["payload"], expected);
                    snapshot["observations"] = json!([expected]);
                    snapshot["requesterControl"]["controlSequence"] = json!(2);
                    reply(&mut stream, 500, b"{}");
                }
                2 | 3 => {
                    assert!(head.starts_with("GET /v1/live/sessions/"));
                    assert!(!head.contains("/observations"));
                    reply(&mut stream, 200, &serde_json::to_vec(&snapshot).unwrap());
                }
                _ => {
                    assert!(head.contains("/close HTTP/1.1"));
                    assert_eq!(body["envelope"]["sequence"], 3);
                    snapshot["closed"] = json!(true);
                    reply(&mut stream, 200, &serde_json::to_vec(&snapshot).unwrap());
                }
            }
        }
    });
    let cancel = Cancel::default();
    let control = Control::new(&client, &session, &cancel);
    control.publish_source(&publication).unwrap();
    let value = observation();
    let mut pending = ObservationSubmission::new(&publication, &value).unwrap();
    assert_eq!(
        control.publish_source_observation(&publication, &mut pending),
        Ok(true)
    );
    control.close_source(&publication).unwrap();
    peer.join().unwrap();
}
#[test]
fn source_observation_unknown_submission_only_reads_until_exact_ack_and_wrong_ack_never_advances() {
    for wrong_ack in [false, true] {
        let (listener, client, tls) = fixture();
        let session = session(&client.origin, u64::MAX);
        let mut publication =
            Publication::new("hook:test", [1, 1], Some(("window:test".into(), 1))).unwrap();
        publication.allow_observation().unwrap();
        let expected = serde_json::to_value(observation()).unwrap();
        let peer = std::thread::spawn(move || {
            let mut snapshot = Value::Null;
            for stage in 0..if wrong_ack { 2 } else { 5 } {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(tls.clone()).unwrap(),
                    accept(&listener),
                );
                let (head, body) = request(&mut stream);
                if stage == 0 {
                    assert!(head.starts_with("POST /v1/live/sessions "));
                    snapshot = json!({"session":body["envelope"]["payload"]["session"],"epoch":1,"closed":false,"observations":[],"requesterControl":{"deviceId":"device-1","epoch":1,"controlSequence":1}});
                    reply(&mut stream, 201, &serde_json::to_vec(&snapshot).unwrap());
                } else if stage >= 2 {
                    assert!(head.starts_with("GET "));
                    if stage == 3 {
                        reply(&mut stream, 503, b"{}");
                        continue;
                    }
                    if stage == 4 {
                        snapshot["requesterControl"]["controlSequence"] = json!(2);
                        snapshot["observations"] = json!([expected]);
                    }
                    reply(&mut stream, 200, &serde_json::to_vec(&snapshot).unwrap());
                } else {
                    assert_eq!(body["envelope"]["sequence"], 2);
                    assert_eq!(body["envelope"]["payload"], expected);
                    if !wrong_ack {
                        reply(&mut stream, 500, b"{}");
                    } else {
                        reply(&mut stream,202,&serde_json::to_vec(&json!({"protocolVersion":"loom.live.v1","acceptedSequence":if wrong_ack{9}else{2},"acceptedObservationSequence":1})).unwrap());
                    }
                }
            }
        });
        let cancel = Cancel::default();
        let control = Control::new(&client, &session, &cancel);
        control.publish_source(&publication).unwrap();
        let value = observation();
        let mut pending = ObservationSubmission::new(&publication, &value).unwrap();
        let first = control.publish_source_observation(&publication, &mut pending);
        if wrong_ack {
            assert_eq!(first, Err(crate::live_relay_http::Error::Cursor));
        } else {
            assert_eq!(first, Ok(false));
            assert_eq!(
                control.publish_source_observation(&publication, &mut pending),
                Err(crate::live_relay_http::Error::Status(503))
            );
            assert_eq!(
                control.publish_source_observation(&publication, &mut pending),
                Ok(true)
            );
            assert_eq!(
                control.publish_source_observation(&publication, &mut pending),
                Err(crate::live_relay_http::Error::Cursor)
            );
        }
        peer.join().unwrap();
    }
}
