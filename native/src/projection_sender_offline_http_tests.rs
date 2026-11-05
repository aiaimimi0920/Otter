//! 原Job在隔离TLS的首发不确定/信任拒绝；复用原nonce、预算和取消，不允许在线回退。
use super::*;
use sha2::{Digest, Sha256};
fn prepared(client: &Client) -> (Directory, sender::Store, SendRequest) {
    let directory = Directory::new();
    directory.install_identity();
    let store = sender::Store::open(&directory.0, &AtomicBool::new(false)).unwrap();
    let peer = format!("loom-{}", "b".repeat(64));
    let id = format!(
        "peer-target:{:x}",
        Sha256::digest(format!("{peer}\ntarget-1"))
    );
    let targets = delivery::targets::Directory::parse(&serde_json::to_vec(&json!({"targets":[{
        "deviceId":id,"route":"offline_peer","peerId":peer,"peerName":"Loom B","remoteDeviceId":"target-1",
        "name":"Remote","policy":"confirm","deliveryAvailable":true,"transferProtocol":"loom.offline-transfer.v1"}]})).unwrap(),client.origin()).unwrap();
    let selected = sender::Selection::for_target(
        &"a".repeat(64),
        client.origin(),
        "unit-1",
        &targets.targets()[0],
    )
    .unwrap();
    let intent = store
        .prepare(
            sender::Preparation {
                selection: &selected,
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
fn run(lost: bool) {
    let (listener, client, tls) = fixture();
    let (directory, store, request) = prepared(&client);
    let original = std::fs::read(directory.path()).unwrap();
    let envelope = serde_json::to_value(request.intent().envelope()).unwrap();
    let target = request.intent().target().to_owned();
    let mut expected = response(&request);
    expected["route"] = "offline_peer".into();
    expected["peerId"] = format!("loom-{}", "b".repeat(64)).into();
    let peer = std::thread::spawn(move || {
        let mut nonces = std::collections::BTreeSet::new();
        for step in 0..if lost { 3 } else { 1 } {
            let mut stream = receive(&listener, tls.clone());
            let (head, body) = http_request(&mut stream);
            assert!(head
                .to_ascii_lowercase()
                .contains("authorization: device fixture-secret\r\n"));
            let lower = head.to_ascii_lowercase();
            assert!(nonces.insert(
                lower
                    .lines()
                    .find_map(|line| line.strip_prefix("x-loom-device-nonce: "))
                    .unwrap()
                    .to_owned()
            ));
            if step == 1 {
                assert!(head.starts_with("POST /v1/offline-projections/create "));
                assert_eq!(body["envelope"], envelope);
                assert_eq!(body["targetDeviceId"], target);
                assert_eq!(body["remoteDeviceId"], "target-1");
                assert_eq!(body["peerId"], format!("loom-{}", "b".repeat(64)));
                // 模拟create已被对端接受但回复丢失；下一步只能读原ID，绝不重发create。
                continue;
            }
            assert!(head.starts_with("POST /v1/offline-projections/read "));
            assert_eq!(body["projectionId"], envelope["projectionId"]);
            if !lost {
                reply(
                    &mut stream,
                    403,
                    br#"{"error":{"code":"projection_peer_revoked"}}"#,
                );
            } else if step == 0 {
                reply(&mut stream, 404, MISSING);
            } else {
                reply(&mut stream, 200, &serde_json::to_vec(&expected).unwrap());
            }
        }
        listener
    });
    let mut grant = grant(&client);
    let mut job = start(&client, &mut grant, request);
    wait(&job);
    let result = job.try_join().unwrap();
    if lost {
        let Output::Sent(ready) = result.unwrap() else {
            panic!("offline result missing")
        };
        assert_eq!(
            ready.response().status(),
            sender::Status::AwaitingConfirmation
        );
        assert!(ready
            .request()
            .intent()
            .target()
            .starts_with("peer-target:"));
    } else {
        assert!(result.is_err());
    }
    assert_eq!(std::fs::read(directory.path()).unwrap(), original);
    assert_eq!(store.pending(&AtomicBool::new(false)).unwrap().len(), 1);
    no_request(&peer.join().unwrap());
}
#[test]
fn offline_lost_create_response_reads_same_id_once_without_resign_or_shared_fallback() {
    run(true);
}
#[test]
fn offline_revoked_trust_response_never_authorizes_create_or_changes_durable_intent() {
    run(false);
}
