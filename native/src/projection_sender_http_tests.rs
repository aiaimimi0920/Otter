//! 实际 loopback TLS 的发送创建/恢复；复用原 CA/HTTP/身份夹具，不发在线请求。
use super::request as http_request;
use super::*;
use crate::{
    device_authorization_lease::Grant,
    projection_delivery_runtime::{Error as JobError, Job, Operation, Output},
};
use delivery::sender::{
    self as sender,
    tests::{frame, Directory},
    Request as SendRequest,
};
use serde_json::{json, Value};
use std::{os::windows::io::AsRawHandle, sync::atomic::AtomicBool};
#[path = "projection_sender_flow_tests.rs"]
mod flow;
#[path = "projection_sender_lifecycle_http_tests.rs"]
mod lifecycle;
#[path = "projection_sender_offline_http_tests.rs"]
mod offline;
#[path = "projection_sender_preparation_http_tests.rs"]
mod preparation;
fn prepared(client: &Client) -> (Directory, sender::Store, SendRequest) {
    let directory = Directory::new();
    directory.install_identity();
    let store = sender::Store::open(&directory.0, &AtomicBool::new(false)).unwrap();
    let selection =
        sender::Selection::new(&"a".repeat(64), client.origin(), "unit-1", "target-1").unwrap();
    let intent = store
        .prepare(
            sender::Preparation {
                selection: &selection,
                frame: frame(),
                identity_root: &directory.identity(),
                device: "device-1",
                now_ms: 1_000,
            },
            &AtomicBool::new(false),
        )
        .unwrap();
    let request = SendRequest::new(intent, directory.identity());
    (directory, store, request)
}
fn response(request: &SendRequest) -> Value {
    let intent = request.intent();
    json!({"envelope":intent.envelope(),"revision":1,"digest":intent.digest(),"linked":true,
        "receiverDeviceId":null,"receiverUnitId":null,"snapshot":null,"delivery":{"targetDeviceId":intent.target(),"status":"awaiting_confirmation"}})
}
fn grant(client: &Client) -> Grant {
    Grant::new(session(client.origin(), u64::MAX))
}
fn wait(job: &Job) {
    use windows::Win32::{
        Foundation::{HANDLE, WAIT_OBJECT_0},
        System::Threading::WaitForSingleObject,
    };
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(job.wait_handle().unwrap().as_raw_handle()), 5000) },
        WAIT_OBJECT_0
    );
}
fn start(client: &Client, grant: &mut Grant, request: SendRequest) -> Job {
    Job::start_prepared(
        client.clone(),
        grant.lease(client.origin(), 0).unwrap(),
        Operation::Send(Box::new(request)),
    )
    .unwrap()
}
fn receive(
    listener: &TcpListener,
    tls: Arc<ServerConfig>,
) -> StreamOwned<ServerConnection, BridgeStream> {
    StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(listener))
}
fn no_request(listener: &TcpListener) {
    assert_eq!(
        listener.accept().err().unwrap().kind(),
        io::ErrorKind::WouldBlock
    );
}
const MISSING: &[u8] =
    br#"{"error":{"code":"projection_not_found","message":"projection_not_found"}}"#;
#[test]
fn sender_read_then_create_uses_durable_exact_identity_target_and_distinct_nonces() {
    let (listener, client, tls) = fixture();
    let (directory, store, request) = prepared(&client);
    let expected = response(&request);
    let original = std::fs::read(directory.path()).unwrap();
    let saved = original.clone();
    let path = directory.path();
    let envelope = serde_json::to_value(request.intent().envelope()).unwrap();
    let peer = std::thread::spawn(move || {
        let mut nonces = std::collections::BTreeSet::new();
        for index in 0..2 {
            let mut stream = receive(&listener, tls.clone());
            let (head, body) = http_request(&mut stream);
            let lower = head.to_ascii_lowercase();
            assert!(lower.contains("authorization: device fixture-secret\r\n"));
            let nonce = lower
                .lines()
                .find_map(|line| line.strip_prefix("x-loom-device-nonce: "))
                .unwrap();
            assert!(nonces.insert(nonce.to_owned()));
            assert_eq!(std::fs::read(&path).unwrap(), saved);
            if index == 0 {
                assert!(head.starts_with("POST /v1/projections/read "));
                assert_eq!(
                    body,
                    json!({"projectionId":envelope["projectionId"],"knownRevision":0})
                );
                reply(&mut stream, 404, MISSING);
            } else {
                assert!(head.starts_with("POST /v1/projections/create "));
                assert_eq!(body["envelope"], envelope);
                assert_eq!(body["targetDeviceId"], "target-1");
                assert_eq!(body["snapshot"]["width"], 2);
                reply(&mut stream, 200, &serde_json::to_vec(&expected).unwrap());
            }
        }
        listener
    });
    let mut grant = grant(&client);
    let mut job = start(&client, &mut grant, request);
    wait(&job);
    let Output::Sent(ready) = job.try_join().unwrap().unwrap() else {
        panic!("missing sender result")
    };
    assert_eq!(ready.response().revision(), 1);
    assert_eq!(ready.response().receiver_device(), None);
    assert!(ready
        .validate(&grant.lease(client.origin(), 0).unwrap(), 0)
        .is_ok());
    let source = sender::binding::tests::source("unit-1", &"a".repeat(64));
    let bound = ready
        .prepare_binding(
            &source,
            "Loopback Target",
            &grant.lease(client.origin(), 0).unwrap(),
            0,
        )
        .unwrap();
    assert!(bound.expected().same_storage(&source));
    assert!(bound.index().includes(bound.binding()));
    assert!(bound.binding().matches_intent(ready.request().intent()));
    assert!(!bound.binding().linked());
    let mut other = Grant::new(session(client.origin(), u64::MAX));
    assert!(ready
        .validate(&other.lease(client.origin(), 0).unwrap(), 0)
        .is_err());
    assert!(ready
        .prepare_binding(
            &source,
            "Loopback Target",
            &other.lease(client.origin(), 0).unwrap(),
            0
        )
        .is_err());
    assert_eq!(std::fs::read(directory.path()).unwrap(), original);
    assert_eq!(store.pending(&AtomicBool::new(false)).unwrap().len(), 1);
    no_request(&peer.join().unwrap());
}
#[test]
fn sender_existing_read_resumes_without_create_even_when_current_revision_is_newer() {
    let (listener, client, tls) = fixture();
    let (_, _, request) = prepared(&client);
    let mut expected = response(&request);
    expected["revision"] = 3.into();
    expected["digest"] = "b".repeat(64).into();
    let peer = std::thread::spawn(move || {
        let mut stream = receive(&listener, tls);
        assert!(http_request(&mut stream)
            .0
            .starts_with("POST /v1/projections/read "));
        reply(&mut stream, 200, &serde_json::to_vec(&expected).unwrap());
        listener
    });
    let mut grant = grant(&client);
    let mut job = start(&client, &mut grant, request);
    wait(&job);
    let Output::Sent(ready) = job.try_join().unwrap().unwrap() else {
        panic!("missing resumed sender")
    };
    assert_eq!(ready.response().revision(), 3);
    no_request(&peer.join().unwrap());
}
#[test]
fn sender_only_explicit_404_not_found_allows_create_not_proxy_errors_or_other_statuses() {
    for (status, bytes) in [
        (404, b"not found".as_slice()),
        (404, br#"{"error":{"code":"projection_route_missing"}}"#),
        (403, MISSING),
        (410, MISSING),
        (200, MISSING),
        (
            404,
            br#"{"error":{"code":"projection_not_found","code":"projection_not_found"}}"#,
        ),
    ] {
        let (listener, client, tls) = fixture();
        let (directory, _, request) = prepared(&client);
        let original = std::fs::read(directory.path()).unwrap();
        let bytes = bytes.to_vec();
        let peer = std::thread::spawn(move || {
            let mut stream = receive(&listener, tls);
            http_request(&mut stream);
            reply(&mut stream, status, &bytes);
            listener
        });
        let mut grant = grant(&client);
        let mut job = start(&client, &mut grant, request);
        wait(&job);
        assert!(job.try_join().unwrap().is_err());
        no_request(&peer.join().unwrap());
        assert_eq!(std::fs::read(directory.path()).unwrap(), original);
    }
}
#[test]
fn sender_lost_create_reply_reconciles_same_id_once_without_repeating_create() {
    let (listener, client, tls) = fixture();
    let (directory, _, request) = prepared(&client);
    let expected = response(&request);
    let envelope = serde_json::to_value(request.intent().envelope()).unwrap();
    let original = std::fs::read(directory.path()).unwrap();
    let peer = std::thread::spawn(move || {
        for index in 0..3 {
            let mut stream = receive(&listener, tls.clone());
            let (head, body) = http_request(&mut stream);
            if index == 1 {
                assert!(head.starts_with("POST /v1/projections/create "));
                assert_eq!(body["envelope"], envelope);
            } else {
                assert!(head.starts_with("POST /v1/projections/read "));
                assert_eq!(body["projectionId"], envelope["projectionId"]);
                if index == 0 {
                    reply(&mut stream, 404, MISSING);
                } else {
                    reply(&mut stream, 200, &serde_json::to_vec(&expected).unwrap());
                }
            }
        }
        listener
    });
    let mut grant = grant(&client);
    let mut job = start(&client, &mut grant, request);
    wait(&job);
    assert!(matches!(job.try_join(), Some(Ok(Output::Sent(_)))));
    no_request(&peer.join().unwrap());
    assert_eq!(std::fs::read(directory.path()).unwrap(), original);
}
#[test]
fn sender_uncertain_create_error_keeps_outbox_and_never_creates_again_after_reconcile_missing() {
    let (listener, client, tls) = fixture();
    let (directory, store, request) = prepared(&client);
    let original = std::fs::read(directory.path()).unwrap();
    let peer = std::thread::spawn(move || {
        for index in 0..3 {
            let mut stream = receive(&listener, tls.clone());
            let (head, _) = http_request(&mut stream);
            assert!(head.starts_with(if index == 1 {
                "POST /v1/projections/create "
            } else {
                "POST /v1/projections/read "
            }));
            reply(&mut stream, if index == 1 { 403 } else { 404 }, MISSING);
        }
        listener
    });
    let mut grant = grant(&client);
    let mut job = start(&client, &mut grant, request);
    wait(&job);
    assert!(matches!(
        job.try_join(),
        Some(Err(JobError::Http(LiveError::Status(403))))
    ));
    no_request(&peer.join().unwrap());
    assert_eq!(std::fs::read(directory.path()).unwrap(), original);
    assert!(!store.pending(&AtomicBool::new(false)).unwrap()[0].cancelled());
}
