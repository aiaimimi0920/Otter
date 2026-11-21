//! 扩展协议共用的真实回环 TLS/WS 测试端；不接触发现文件、实际插件或用户资源。
use super::*;
use crate::loom_bridge_stream::BridgeStream;
pub(super) use crate::loom_extension_snapshot::tests::fixture;
use rustls::{ServerConnection, StreamOwned};
use serde_json::{json, Value};
use std::thread;
use tungstenite::{Message, WebSocket};
pub(crate) type Peer = WebSocket<StreamOwned<ServerConnection, BridgeStream>>;
pub(super) const COMMAND: &str = "neuro.official/ocr.toggle-overlay";
pub(super) fn context() -> Context<'static> {
    Context {
        kind: Some(crate::extension_when::UnitKind::Sticker),
        has_image: true,
        attachment_types: &[],
    }
}
pub(crate) fn read(peer: &mut Peer) -> Value {
    serde_json::from_str(&peer.read().unwrap().into_text().unwrap()).unwrap()
}
pub(crate) fn send(peer: &mut Peer, value: Value) {
    peer.send(Message::Text(value.to_string())).unwrap();
}
pub(crate) fn response(id: &Value, data: Value) -> Value {
    json!({"protocol":catalog::PROTOCOL, "apiVersion":catalog::API, "requestId":id, "status":"succeeded", "data":data})
}
pub(crate) fn handshake_data() -> Value {
    json!({"sessionId":"extension:test", "features":["contribution.snapshot", "command.invoke", "resource.authorization.v1"], "snapshot":fixture()})
}
pub(crate) fn handshake(peer: &mut Peer, data: Value) {
    let request = read(peer);
    assert_eq!(request["method"], "loom.extension.handshake");
    let params = &request["params"];
    assert_eq!(params["hookSessionId"], "fixture-session");
    assert_eq!(params["protocol"], catalog::PROTOCOL);
    assert_eq!(params["apiVersion"], catalog::API);
    assert_eq!(
        params["requiredFeatures"],
        json!([
            "contribution.snapshot",
            "command.invoke",
            "resource.authorization.v1"
        ])
    );
    send(peer, response(&params["requestId"], data));
}
pub(crate) fn peer(
    run: impl FnOnce(&mut Peer) + Send + 'static,
) -> (Identity, thread::JoinHandle<()>) {
    let (listener, identity, config) = crate::loom_bridge_client::tests::fixture();
    let worker = thread::spawn(move || {
        let stream = BridgeStream::new(
            listener.accept().unwrap().0,
            Instant::now() + Duration::from_secs(5),
        )
        .unwrap();
        let mut peer = tungstenite::accept(StreamOwned::new(
            ServerConnection::new(config).unwrap(),
            stream,
        ))
        .unwrap();
        assert_eq!(read(&mut peer)["method"], "loom.hook.handshake");
        send(&mut peer, crate::loom_handshake::tests::fixture());
        run(&mut peer);
    });
    (identity, worker)
}
pub(super) fn authority_request(peer: &mut Peer) -> Value {
    let request = read(peer);
    assert_eq!(request["method"], "loom.extension.command.authorize");
    let params = &request["params"];
    assert_eq!(params["sessionId"], "extension:test");
    assert_eq!(params["pluginId"], "neuro.official/ocr");
    assert_eq!(params["commandId"], COMMAND);
    assert_eq!(params["snapshotGeneration"], 7);
    assert_eq!(
        params["target"],
        json!({"unitId":"unit:test", "revision":12})
    );
    assert_eq!(params["checkOnly"], true);
    assert_eq!(
        params.as_object().unwrap().len(),
        7,
        "no images, attachments, invoke or resource lease"
    );
    params["requestId"].clone()
}
pub(super) fn target() -> Target {
    Target::new("unit:test", 12).unwrap()
}
pub(crate) fn updated(snapshot: Value) -> Value {
    json!({"protocol":catalog::PROTOCOL, "apiVersion":catalog::API,
        "method":"loom.extension.snapshot.updated", "params":{"snapshot":snapshot}})
}
