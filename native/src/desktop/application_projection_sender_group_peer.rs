//! 合成TLS接收端独立验证每份PNG尺寸/像素/digest；只识别测试目录中的设备，不用真实服务。
use super::*;
use base64::Engine;
use serde_json::{json, Value};
use sha2::Digest;
use std::{net::TcpListener, path::PathBuf, sync::Arc, thread::JoinHandle};
pub(super) fn start(
    listener: TcpListener,
    tls: Arc<rustls::ServerConfig>,
    root: PathBuf,
    count: usize,
    fail_second: bool,
) -> JoinHandle<Vec<Value>> {
    serve(listener, tls, root, count, fail_second, false, false)
}
pub(super) fn start_stopping(
    listener: TcpListener,
    tls: Arc<rustls::ServerConfig>,
    root: PathBuf,
    count: usize,
    fail_first_stop: bool,
) -> JoinHandle<Vec<Value>> {
    serve(listener, tls, root, count, false, true, fail_first_stop)
}
fn serve(
    listener: TcpListener,
    tls: Arc<rustls::ServerConfig>,
    root: PathBuf,
    count: usize,
    fail_second: bool,
    stopping: bool,
    fail_first_stop: bool,
) -> JoinHandle<Vec<Value>> {
    std::thread::spawn(move || {
        let mut received = Vec::new();
        let mut second_failed = false;
        let mut failed_id = None;
        let mut stops = 0;
        for step in 0..count {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            if step == 0 {
                assert!(head.starts_with("POST /v1/device-sessions/challenges "));
                reply(
                    &mut stream,
                    200,
                    br#"{"deviceId":"device-old","challengeId":"id","challenge":"value"}"#,
                );
            } else if step == 1 {
                assert!(head.starts_with("POST /v1/device-sessions "));
                reply(&mut stream,200,&serde_json::to_vec(&json!({"deviceId":"device-old","token":"group-token","expiresAtMs":now().unwrap()+60_000})).unwrap());
            } else {
                assert!(head
                    .to_ascii_lowercase()
                    .contains("authorization: device group-token\r\n"));
                let journal: Value = serde_json::from_slice(
                    &std::fs::read(root.join("projection-sender.json")).unwrap(),
                )
                .unwrap();
                let id = if head.starts_with("POST /v1/projections/create ") {
                    &body["envelope"]["projectionId"]
                } else {
                    &body["projectionId"]
                };
                let entry = journal["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|entry| &entry["envelope"]["projectionId"] == id)
                    .unwrap();
                let envelope = entry["envelope"].clone();
                if head.starts_with("POST /v1/projections/read ") {
                    if fail_second && received.len() == 1 && !second_failed {
                        second_failed = true;
                        failed_id = Some(envelope["projectionId"].clone());
                        reply(&mut stream, 503, b"temporary");
                    } else {
                        if let Some(failed) = &failed_id {
                            assert_eq!(failed, &envelope["projectionId"]);
                        }
                        reply(
                            &mut stream,
                            404,
                            br#"{"error":{"code":"projection_not_found"}}"#,
                        );
                    }
                } else if head.starts_with("POST /v1/projections/create ") {
                    assert_eq!(body["envelope"], envelope);
                    assert_eq!(
                        body["targetDeviceId"],
                        format!("receiver-{}", received.len())
                    );
                    let png = base64::engine::general_purpose::STANDARD
                        .decode(body["snapshot"]["imageBase64"].as_str().unwrap())
                        .unwrap();
                    assert_eq!(
                        format!("{:x}", sha2::Sha256::digest(&png)),
                        envelope["content"]["digest"]
                    );
                    let reader = png::Decoder::new(std::io::Cursor::new(&png))
                        .read_info()
                        .unwrap();
                    assert_eq!((reader.info().width, reader.info().height), (32, 32));
                    assert_eq!(body["snapshot"]["width"], 32);
                    assert_eq!(body["snapshot"]["height"], 32);
                    let mut reader = reader;
                    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
                    let info = reader.next_frame(&mut pixels).unwrap();
                    assert_eq!(info.color_type, png::ColorType::Rgba);
                    assert!(pixels[..info.buffer_size()]
                        .chunks_exact(4)
                        .all(|pixel| pixel == [0, 0, 255, 255]));
                    let target = body["targetDeviceId"].clone();
                    reply(&mut stream,200,&serde_json::to_vec(&json!({"envelope":envelope,"revision":1,"digest":envelope["content"]["digest"],"linked":true,"receiverDeviceId":null,"receiverUnitId":null,"snapshot":null,"delivery":{"targetDeviceId":target,"status":"awaiting_confirmation"}})).unwrap());
                    received.push(body);
                } else {
                    assert!(head.starts_with("POST /v1/projections/unlink "));
                    assert_eq!(entry["cancelled"], true);
                    if stopping {
                        assert_eq!(entry["stop"], "requested");
                        let saved: Value = serde_json::from_slice(
                            &std::fs::read(root.join("workspace.json")).unwrap(),
                        )
                        .unwrap();
                        let binding = saved["stickers"][0]["projectionSenders"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|binding| binding["link"]["envelope"]["projectionId"] == *id)
                            .unwrap();
                        assert_eq!(
                            binding["link"]["stopPending"], true,
                            "unlink requires real canonical pending"
                        );
                        let sent = received
                            .iter()
                            .find(|sent| sent["envelope"]["projectionId"] == *id)
                            .unwrap();
                        assert_eq!(
                            sent["envelope"], envelope,
                            "stop must keep original signed identity"
                        );
                    }
                    if stopping && fail_first_stop && stops == 0 {
                        reply(&mut stream, 503, b"temporary");
                    } else {
                        reply(&mut stream, 200, br#"{"unlinked":true}"#);
                    }
                    stops += 1;
                }
            }
        }
        assert!(
            matches!(listener.accept(),Err(error) if error.kind()==std::io::ErrorKind::WouldBlock)
        );
        received
    })
}
