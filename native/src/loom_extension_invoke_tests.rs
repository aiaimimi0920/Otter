//! 单次许可、资源读取顺序与真实 TLS 命令结果；不执行实际 OCR/译文模型或系统副作用。
use super::super::test_peer::*;
use super::*;
use std::sync::{atomic::AtomicUsize, Arc};
const RUN: &str = "neuro.official/ocr.recognize-selected-unit";
const LEASE: &str = "extension-auth:01234567-89ab-4cde-8123-0123456789ab";
fn data(image: bool) -> Value {
    let mut data = handshake_data();
    let permissions = if image {
        json!(["hook.unit.image.read", "hook.unit.attachments.read"])
    } else {
        json!([])
    };
    data["snapshot"]["plugins"][0]["effectivePermissions"] = permissions.clone();
    let command = &mut data["snapshot"]["contributions"]["commands"][0];
    command["id"] = json!(RUN);
    command["commandId"] = json!(RUN);
    command["payload"] = json!({"permissions":permissions, "timeoutMs":60_000});
    data
}
fn authorize(peer: &mut Peer, lease: &str) {
    let id = authorization(peer);
    send(peer, response(&id, json!({"authorizationId":lease})));
}
fn authorization(peer: &mut Peer) -> Value {
    let request = read(peer);
    assert_eq!(request["method"], "loom.extension.command.authorize");
    let params = &request["params"];
    assert_eq!(params["sessionId"], "extension:test");
    assert_eq!(params["commandId"], RUN);
    assert_eq!(params["pluginId"], "neuro.official/ocr");
    assert_eq!(params["snapshotGeneration"], 7);
    assert_eq!(
        params["target"],
        json!({"unitId":"unit:test", "revision":12})
    );
    assert_eq!(params["checkOnly"], false);
    params["requestId"].clone()
}
fn invocation(peer: &mut Peer, resource: bool) -> Value {
    let request = read(peer);
    assert_eq!(request["method"], "loom.extension.command.invoke");
    assert_eq!(request["params"]["sessionId"], "extension:test");
    let call = &request["params"]["invocation"];
    assert_eq!(call["protocol"], catalog::PROTOCOL);
    assert_eq!(call["apiVersion"], catalog::API);
    assert_eq!(call["commandId"], RUN);
    assert_eq!(call["pluginId"], "neuro.official/ocr");
    assert_eq!(call["snapshotGeneration"], 7);
    assert_eq!(call["target"], json!({"unitId":"unit:test", "revision":12}));
    assert_eq!(call["resourceRefs"], json!([]));
    if resource {
        assert_eq!(request["params"]["authorizationId"], LEASE);
    } else {
        assert!(request["params"].get("authorizationId").is_none());
    }
    request
}
fn complete(peer: &mut Peer, id: &Value) {
    send(
        peer,
        response(
            id,
            json!({"protocol":catalog::PROTOCOL, "apiVersion":catalog::API,
        "requestId":id, "status":"succeeded", "output":{"text":"合成识别结果"},
        "effects":[{"type":"attachment.upsert", "payload":{"attachmentId":"neuro.official/ocr.result",
            "typeId":"neuro.official/ocr.result.v1", "schemaVersion":"1", "priorRevision":0, "revision":1,
            "payload":{"fullText":"合成识别结果"}, "resourceRefs":[]}}]}),
        ),
    );
}
fn prepared() -> Result<Prepared, Error> {
    Ok(Prepared::new(json!({"fixture":true}))?.image(Image::new("image/png", &[1, 2, 3])?))
}
#[test]
fn real_single_use_authorization_precedes_image_read_and_complete_result_delivery() {
    let seen = Arc::new(AtomicUsize::new(0));
    let signal = seen.clone();
    let (identity, server) = peer(move |peer| {
        handshake(peer, data(true));
        let id = authorization(peer);
        signal.store(1, Ordering::Release);
        send(peer, response(&id, json!({"authorizationId":LEASE})));
        let request = invocation(peer, true);
        assert_eq!(
            request["params"]["resourceUploads"],
            json!([{"kind":"image", "mime":"image/png", "dataBase64":"AQID"}])
        );
        assert_eq!(
            request["params"]["invocation"]["input"],
            json!({"fixture":true})
        );
        assert!(request["params"]["invocation"]["userGestureToken"]
            .as_str()
            .unwrap()
            .starts_with("otter-gesture:"));
        complete(peer, &request["params"]["invocation"]["requestId"]);
    });
    let cancel = AtomicBool::new(false);
    let mut session = Session::open(&identity, &cancel).unwrap();
    let result = session
        .invoke(
            RUN,
            &context(),
            &target(),
            |access| {
                assert_eq!(seen.load(Ordering::Acquire), 1);
                assert!(access.image_allowed() && access.attachments_allowed());
                assert_eq!(access.plugin(), "neuro.official/ocr");
                assert!(access.input_context().is_none());
                prepared()
            },
            &cancel,
        )
        .unwrap();
    assert_eq!(result.output()["text"], "合成识别结果");
    assert_eq!(result.effects()[0].kind(), &EffectKind::AttachmentUpsert);
    assert_eq!(result.scope(), "scope:ocr");
    assert!(session.snapshot().is_some());
    drop(session);
    server.join().unwrap();
}
#[test]
fn commands_without_resource_reads_or_gesture_requirement_do_not_acquire_tickets() {
    let (identity, server) = peer(|peer| {
        let mut data = data(false);
        data["snapshot"]["contributions"]["commands"][0]["payload"]["requiresUserGesture"] =
            json!(false);
        handshake(peer, data);
        let request = invocation(peer, false);
        assert!(request["params"].get("resourceUploads").is_none());
        assert!(request["params"]["invocation"]
            .get("userGestureToken")
            .is_none());
        complete(peer, &request["params"]["invocation"]["requestId"]);
    });
    let cancel = AtomicBool::new(false);
    let mut session = Session::open(&identity, &cancel).unwrap();
    session
        .invoke(
            RUN,
            &context(),
            &target(),
            |_| Prepared::new(json!({})),
            &cancel,
        )
        .unwrap();
    drop(session);
    server.join().unwrap();
}
#[test]
fn every_invocation_has_a_fresh_request_and_gesture_and_does_not_reuse_resource_authorization() {
    let (identity, server) = peer(|peer| {
        handshake(peer, data(true));
        let mut prior = None;
        for _ in 0..2 {
            // 即使 peer 故意复用相同测试票值，客户端仍必须每次重新请求，而不是缓存授权。
            authorize(peer, LEASE);
            let request = invocation(peer, true);
            let call = &request["params"]["invocation"];
            if let Some((id, gesture)) = prior {
                assert_ne!(call["requestId"], id);
                assert_ne!(call["userGestureToken"], gesture);
            }
            prior = Some((call["requestId"].clone(), call["userGestureToken"].clone()));
            complete(peer, &call["requestId"]);
        }
    });
    let cancel = AtomicBool::new(false);
    let mut session = Session::open(&identity, &cancel).unwrap();
    for _ in 0..2 {
        session
            .invoke(RUN, &context(), &target(), |_| prepared(), &cancel)
            .unwrap();
    }
    drop(session);
    server.join().unwrap();
}
#[test]
fn main_command_timeout_margin_gesture_default_and_ocr_feature_contract_are_preserved() {
    for (declared, expected) in [
        (Value::Null, 20_000),
        (json!(60_000), 70_000),
        (json!(1.9), 10_001),
        (json!(200_000), 180_000),
        (json!(-1), 20_000),
    ] {
        let mut data = data(true);
        data["snapshot"]["contributions"]["commands"][0]["payload"]["timeoutMs"] = declared;
        let snapshot = Snapshot::parse(&serde_json::to_vec(&data["snapshot"]).unwrap()).unwrap();
        let policy = Policy::new(snapshot.command(RUN, &context()).unwrap(), false).unwrap();
        assert_eq!(policy.timeout, Duration::from_millis(expected));
        assert!(policy.gesture);
    }
    let mut data = data(true);
    data["snapshot"]["contributions"]["commands"][0]["payload"]["inputContext"] =
        json!("ocr-text.v1");
    let snapshot = Snapshot::parse(&serde_json::to_vec(&data["snapshot"]).unwrap()).unwrap();
    assert!(matches!(
        Policy::new(snapshot.command(RUN, &context()).unwrap(), false),
        Err(Error::Unsupported)
    ));
    let policy = Policy::new(snapshot.command(RUN, &context()).unwrap(), true).unwrap();
    assert_eq!(policy.access.input_context(), Some("ocr-text.v1"));
    assert!(valid_lease(LEASE));
    for bad in [
        "extension-auth:invalid",
        "extension-auth:0123456789ab4cde81230123456789ab",
        "checkOnly:true",
    ] {
        assert!(!valid_lease(bad));
    }
}
#[path = "loom_extension_invoke_abort_tests.rs"]
mod abort;
