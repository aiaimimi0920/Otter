//! TLS 实际分片/半帧跨短读超时仍须恢复；不连接真实 Loom。
use super::*;
use crate::{
    desktop::loom_execution_session::Session, graph_requests::Request, workspace_id::UnitId,
};
use std::{io::Write, sync::atomic::AtomicBool};
use tungstenite::protocol::frame::{
    coding::{Data, OpCode},
    Frame,
};

#[test]
fn fragmented_final_and_partial_frame_survive_repeated_read_timeouts() {
    let (listener, identity, config) = fixture();
    let worker = thread::spawn(move || {
        let stream = BridgeStream::new(
            listener.accept().unwrap().0,
            Instant::now() + Duration::from_secs(5),
        )
        .unwrap();
        let mut socket = tungstenite::accept(StreamOwned::new(
            ServerConnection::new(config).unwrap(),
            stream,
        ))
        .unwrap();
        socket.read().unwrap();
        let text=serde_json::json!({"protocolVersion":"loom.hook.v1","requestId":"request","status":"succeeded","data":{
            "protocolVersion":"loom.hook.v1","requestId":"request","nodeId":"unit","generation":1,"resultRevision":1,
            "outputs":{"output":{"kind":"value","value":"分片结果🦦"}}
        }}).to_string().into_bytes();
        let split = text.len() / 2;
        socket
            .send(tungstenite::Message::Frame(Frame::message(
                text[..split].to_vec(),
                OpCode::Data(Data::Text),
                false,
            )))
            .unwrap();
        thread::sleep(Duration::from_millis(400));
        let mut second = Vec::new();
        Frame::message(text[split..].to_vec(), OpCode::Data(Data::Continue), true)
            .format(&mut second)
            .unwrap();
        let middle = second.len() / 2;
        // 绕过 WebSocket 写封装，仅分拆测试帧的传输；TLS 仍为真实认证连接。
        socket.get_mut().write_all(&second[..middle]).unwrap();
        socket.get_mut().flush().unwrap();
        thread::sleep(Duration::from_millis(400));
        socket.get_mut().write_all(&second[middle..]).unwrap();
        socket.get_mut().flush().unwrap();
    });
    let unit = UnitId::parse("unit").unwrap();
    let request = Request {
        id: UnitId::parse("request").unwrap(),
        generation: 1,
    };
    let cancel = AtomicBool::new(false);
    let session = Session {
        identity: &identity,
        unit: &unit,
        request: &request,
        cancel: &cancel,
        timeout: Duration::from_secs(4),
    };
    let result = session
        .run("fixture".into(), |_| {}, |_| panic!("no interim messages"))
        .unwrap_or_else(|_| panic!("fragmented session failed"));
    let crate::desktop::loom_output_batch::Output::Value(value) =
        &result.delivery.result().unwrap().primary().value
    else {
        panic!("value");
    };
    assert_eq!(value, &serde_json::json!("分片结果🦦"));
    worker.join().unwrap();
}
