//! 真实固定-origin TLS、Device nonce、原 Permit/Lease 和服务 reply；不冒充 QuickJS/产品接线。
use super::*;
use crate::live_relay_http::art_prepare::service::{Call, Error as ServiceError, Status};
use serde_json::{json, Value};
use std::thread::JoinHandle;

const SERVICE: &str = "capability:vendor:service.run";
#[path = "art_service_canvas_tests.rs"]
mod canvas_tests;
#[path = "art_service_confirmation_tests.rs"]
mod confirmation_tests;
#[path = "art_service_failure_tests.rs"]
mod failure_tests;
#[path = "art_service_image_output_tests.rs"]
mod image_output_tests;
#[path = "art_service_images_tests.rs"]
mod images_tests;
#[path = "art_service_runtime_tests.rs"]
mod runtime_tests;
#[path = "art_service_signed_tests.rs"]
mod signed_tests;
fn service_response_source(origin: &str, source: &str) -> Value {
    let mut value = response(origin);
    let mut grant: wire::Grant = serde_json::from_slice(
        &STANDARD
            .decode(value["grant"]["payload"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    grant.permissions.push(wire::Permission::LoomService);
    grant.services.push(SERVICE.into());
    let digest = format!("{:x}", Sha256::digest(source));
    grant.source = wire::Source {
        resource_id: format!("sha256:{digest}"),
        digest,
        size: source.len() as u64,
    };
    value["source"] = json!(STANDARD.encode(source));
    let payload = serde_json::to_vec(&grant).unwrap();
    let signature = key().sign(&[wire::DOMAIN, payload.as_slice()].concat());
    value["grant"]["payload"] = json!(STANDARD.encode(payload));
    value["grant"]["signature"] = json!(STANDARD.encode(signature.as_ref()));
    value
}
fn setup(
    count: usize,
    change: impl FnMut(usize, &str, &Value, &mut Value) + Send + 'static,
) -> (Prepared, Client, DeviceGrant, Arc<Lease>, JoinHandle<()>) {
    setup_source(count, SOURCE, change)
}
fn setup_source(
    count: usize,
    source: &'static str,
    mut change: impl FnMut(usize, &str, &Value, &mut Value) + Send + 'static,
) -> (Prepared, Client, DeviceGrant, Arc<Lease>, JoinHandle<()>) {
    setup_source_status(count, source, move |index, head, body, response| {
        change(index, head, body, response);
        200
    })
}
fn setup_source_status(
    count: usize,
    source: &'static str,
    mut change: impl FnMut(usize, &str, &Value, &mut Value) -> u16 + Send + 'static,
) -> (Prepared, Client, DeviceGrant, Arc<Lease>, JoinHandle<()>) {
    let (listener, client, config) = fixture();
    let origin = client.origin.clone();
    let (device, lease) = lease(&origin);
    let peer = std::thread::spawn(move || {
        let mut stream = StreamOwned::new(
            ServerConnection::new(config.clone()).unwrap(),
            accept(&listener),
        );
        let (head, _) = request(&mut stream);
        assert!(head.starts_with("POST /v1/art/local/prepare HTTP/1.1"));
        reply(
            &mut stream,
            200,
            &serde_json::to_vec(&service_response_source(&origin, source)).unwrap(),
        );
        drop(stream);
        let mut nonces = std::collections::BTreeSet::new();
        for index in 0..count {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            let lower = head.to_ascii_lowercase();
            assert!(lower.contains("authorization: device fixture-secret\r\n"));
            let nonce = lower
                .lines()
                .find(|line| line.starts_with("x-loom-device-nonce: "))
                .unwrap()
                .to_owned();
            assert!(nonces.insert(nonce));
            let call = if body["protocolVersion"] == "loom.art.local.service.confirmation.v1" {
                if body.get("call").is_some() {
                    &body["call"]
                } else {
                    &body["control"]
                }
            } else if head.starts_with("POST /v1/art/local/service/submit-images ")
                || head.starts_with("POST /v1/art/local/service/submit-image-output ")
            {
                assert!(matches!(
                    body["protocolVersion"].as_str(),
                    Some(
                        "loom.art.local.service.images.v1"
                            | "loom.art.local.service.image-output.v1"
                    )
                ));
                &body["call"]
            } else {
                &body
            };
            assert_eq!(call["protocolVersion"], "loom.art.local.service.v1");
            assert!(call["grant"]["signature"].is_string());
            let mut response = json!({"protocolVersion":"loom.art.local.service.v1","owner":call["owner"],"sequence":index+1,
                "status":"accepted","error":null,"progress":null});
            let status = change(index, &head, &body, &mut response);
            if status != 0 {
                reply(&mut stream, status, &serde_json::to_vec(&response).unwrap());
            }
        }
    });
    let mut input = input(&client.origin);
    input.permissions.push(wire::Permission::LoomService);
    input.services.push(SERVICE.into());
    let prepared = Prepared::fetch(&client, lease.clone(), input, pin()).unwrap();
    (prepared, client, device, lease, peer)
}
#[test]
fn art_service_tls_submit_progress_null_and_terminal_are_identity_bound() {
    let (mut prepared, client, _device, _lease, peer) = setup(3, |index, head, body, response| {
        if index == 0 {
            assert!(head.starts_with("POST /v1/art/local/service/submit HTTP/1.1"));
            assert_eq!(body["input"], json!({"text":"hello"}));
            assert_eq!(body["timeoutMs"], 5000);
        } else {
            assert!(head.starts_with("POST /v1/art/local/service/poll HTTP/1.1"));
            assert!(body.get("input").is_none());
        }
        if index == 1 {
            response["status"] = json!("running");
            response["progress"] = json!({"step":1});
        }
        if index == 2 {
            response["status"] = json!("succeeded");
            response["output"] = Value::Null;
        }
    });
    let mut call = Call::new(
        &mut prepared,
        &client,
        SERVICE,
        json!({"text":"hello"}),
        5000,
    )
    .unwrap();
    assert_eq!(call.owner().binding.device_id, "device-1");
    assert_eq!(call.submit().unwrap().status, Status::Accepted);
    assert_eq!(call.poll().unwrap().progress, Some(json!({"step":1})));
    let result = call.poll().unwrap();
    assert_eq!(result.output, Some(Value::Null));
    assert_eq!(result.status, Status::Succeeded);
    assert_eq!(call.cancel().unwrap(), result); // no network after terminal
    assert_eq!(call.submit(), Err(ServiceError::Sequence));
    peer.join().unwrap();
}
#[test]
fn art_service_tls_cancel_uses_original_owner_without_resubmitting_input() {
    let (mut prepared, client, _device, _lease, peer) = setup(2, |index, head, body, response| {
        if index == 1 {
            assert!(head.starts_with("POST /v1/art/local/service/cancel HTTP/1.1"));
            assert!(body.get("input").is_none());
            response["status"] = json!("cancelled");
        }
    });
    let mut call = Call::new(&mut prepared, &client, SERVICE, json!({}), 5000).unwrap();
    call.submit().unwrap();
    assert_eq!(call.cancel().unwrap().status, Status::Cancelled);
    peer.join().unwrap();
}
#[test]
fn art_service_tls_cross_instance_reply_poisoned_chain_cannot_poll_again() {
    let (mut prepared, client, _device, _lease, peer) = setup(1, |_, _, _, response| {
        response["owner"]["binding"]["generation"] = json!(2);
    });
    let mut call = Call::new(&mut prepared, &client, SERVICE, json!({}), 5000).unwrap();
    assert_eq!(call.submit(), Err(ServiceError::Identity));
    assert_eq!(call.poll(), Err(ServiceError::Stopped));
    peer.join().unwrap();
}
#[test]
fn art_service_tls_same_sequence_cannot_change_payload() {
    let (mut prepared, client, _device, _lease, peer) = setup(2, |index, _, _, response| {
        if index == 1 {
            response["sequence"] = json!(1);
            response["status"] = json!("running");
        }
    });
    let mut call = Call::new(&mut prepared, &client, SERVICE, json!({}), 5000).unwrap();
    call.submit().unwrap();
    assert_eq!(call.poll(), Err(ServiceError::Sequence));
    peer.join().unwrap();
}
#[test]
fn art_service_tls_revoked_original_lease_cannot_use_cached_terminal() {
    let (mut prepared, client, _device, lease, peer) = setup(1, |_, _, _, response| {
        response["status"] = json!("succeeded");
        response["output"] = json!({"ok":true});
    });
    let mut call = Call::new(&mut prepared, &client, SERVICE, json!({}), 5000).unwrap();
    call.submit().unwrap();
    lease.revoke();
    assert_eq!(call.poll(), Err(ServiceError::Lease));
    assert_eq!(call.cancel(), Err(ServiceError::Stopped));
    peer.join().unwrap();
}
#[test]
fn art_service_tls_scope_and_input_limits_reject_before_dispatch() {
    let (mut prepared, client, _device, _lease, peer) = setup(0, |_, _, _, _| {});
    assert!(matches!(
        Call::new(
            &mut prepared,
            &client,
            "capability:vendor:admin.run",
            json!({}),
            5000
        ),
        Err(ServiceError::Input)
    ));
    assert!(matches!(
        Call::new(
            &mut prepared,
            &client,
            SERVICE,
            json!("x".repeat(65537)),
            5000
        ),
        Err(ServiceError::Limit)
    ));
    assert!(matches!(
        Call::new(&mut prepared, &client, SERVICE, json!({}), 0),
        Err(ServiceError::Input)
    ));
    peer.join().unwrap();
}
