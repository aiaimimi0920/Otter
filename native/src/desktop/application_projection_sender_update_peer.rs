//! 持续投送隔离TLS接收端：独立solid/crop像素oracle，不接真实配置或真实设备。
use super::*;
use base64::Engine;
use serde_json::{json, Value};
use sha2::Digest;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{mpsc, Arc},
    thread::JoinHandle,
};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Normal,
    Lost,
    Malformed,
    Stale,
    Stopped,
    Offline,
    Revoked,
    Workspace,
    Profile,
    DurableStop,
    WrongPeer,
    PeerRevoked,
}
fn metadata(envelope: &Value, revision: u64, digest: &str) -> Value {
    json!({"envelope":envelope,"revision":revision,"digest":digest,"linked":true,
        "receiverDeviceId":"receiver-one","receiverUnitId":"remote-unit","snapshot":null,
        "delivery":{"targetDeviceId":"receiver-one","status":"accepted"}})
}
fn pixels(body: &Value, width: u32, height: u32, color: [u8; 4], stage: &str) -> String {
    let png = base64::engine::general_purpose::STANDARD
        .decode(body["snapshot"]["imageBase64"].as_str().unwrap())
        .unwrap();
    assert_eq!(body["snapshot"]["width"], width);
    assert_eq!(body["snapshot"]["height"], height);
    let image = crate::desktop::export::decode_png(png.clone());
    assert_eq!((image.width, image.height), (width, height));
    assert_eq!(&*image.pixels, color.repeat((width * height) as usize));
    let digest = format!("{:x}", sha2::Sha256::digest(&png));
    if let Some(root) = std::env::var_os("OTTER_PNG_EVIDENCE_DIR") {
        let root = std::path::PathBuf::from(root);
        std::fs::create_dir_all(&root).unwrap();
        let test = std::thread::current().name().unwrap().to_owned();
        let key = format!("{:x}", sha2::Sha256::digest(test.as_bytes()));
        let path = root.join(format!("{}-{stage}", &key[..16]));
        for (path, bytes) in [
            (path.with_extension("png"), png),
            (
                path.with_extension("json"),
                serde_json::to_vec_pretty(&json!({
                    "test":test,"stage":stage,"metadataWidth":width,"metadataHeight":height,
                    "pngWidth":image.width,"pngHeight":image.height,"digest":digest,
                    "independentPixelsMatch":true,"maxPixelDifference":0
                }))
                .unwrap(),
            ),
        ] {
            std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(path)
                .unwrap()
                .write_all(&bytes)
                .unwrap();
        }
    }
    digest
}
pub(super) fn start(
    listener: TcpListener,
    tls: Arc<rustls::ServerConfig>,
    mode: Mode,
) -> JoinHandle<TcpListener> {
    start_with_signal(listener, tls, mode, None, false)
}
pub(super) fn start_with_signal(
    listener: TcpListener,
    tls: Arc<rustls::ServerConfig>,
    mode: Mode,
    update_received: Option<mpsc::Sender<()>>,
    offline: bool,
) -> JoinHandle<TcpListener> {
    let name = std::thread::current().name().unwrap().to_owned();
    std::thread::Builder::new().name(name).spawn(move || {
        let _com = Apartment::new().unwrap();
        let mut envelope = Value::Null;
        let mut digest = String::new();
        let mut revision = 1;
        let count = match mode { Mode::Normal => 10, Mode::Lost | Mode::Malformed | Mode::WrongPeer => 7,
            Mode::Stale => 8, Mode::Stopped | Mode::Revoked | Mode::Workspace => 6, Mode::Offline => 5,
            Mode::Profile => 4, Mode::DurableStop => 7, Mode::PeerRevoked => 5 };
        let peer = format!("loom-{}", "b".repeat(64));
        let target = if offline { format!("peer-target:{:x}", sha2::Sha256::digest(format!("{peer}\nreceiver-one"))) } else { "receiver-one".into() };
        for step in 0..count {
            let mut stream = StreamOwned::new(ServerConnection::new(tls.clone()).unwrap(), accept(&listener));
            let (head,body) = request(&mut stream);
            eprintln!("continuous-peer step={step} path={}", head.lines().next().unwrap());
            if step == 0 {
                assert!(head.starts_with("POST /v1/device-sessions/challenges "));
                reply(&mut stream,200,br#"{"deviceId":"device-old","challengeId":"id","challenge":"value"}"#);
                continue;
            }
            if step == 1 {
                assert!(head.starts_with("POST /v1/device-sessions "));
                reply(&mut stream,200,&serde_json::to_vec(&json!({"deviceId":"device-old","token":"sender-token","expiresAtMs":now().unwrap()+60_000})).unwrap());
                continue;
            }
            assert!(head.to_ascii_lowercase().contains("authorization: device sender-token\r\n"));
            let head = if offline {
                assert!(head.starts_with("POST /v1/offline-projections/"));
                head.replacen("/v1/offline-projections/", "/v1/projections/", 1)
            } else { head };
            if mode == Mode::DurableStop && step == 6 {
                assert!(head.starts_with("POST /v1/projections/unlink "));
                reply(&mut stream,200,br#"{"unlinked":true}"#);
                continue;
            }
            if step == 2 {
                assert!(head.starts_with("POST /v1/projections/read "));
                reply(&mut stream,404,br#"{"error":{"code":"projection_not_found"}}"#);
                continue;
            }
            if step == 3 {
                assert!(head.starts_with("POST /v1/projections/create "));
                envelope = body["envelope"].clone();
                assert_eq!(body["targetDeviceId"],target);
                if offline {
                    assert_eq!(body["peerId"], peer);
                    assert_eq!(body["remoteDeviceId"], "receiver-one");
                } else { assert!(body.get("peerId").is_none()); }
                digest = pixels(&body,32,32,[255,0,0,255],"create");
                assert_eq!(envelope["content"]["digest"],digest);
            } else {
                assert_eq!(body["projectionId"],envelope["projectionId"]);
                let is_update = step == 5 || (mode == Mode::Normal && step == 9) || (mode == Mode::Stale && step == 7);
                if is_update {
                    assert!(head.starts_with("POST /v1/projections/update "));
                    assert_eq!(body["sourceSessionId"],envelope["source"]["sessionId"]);
                    assert_eq!(body["priorRevision"],revision);
                    assert_eq!(body["revision"],revision+1);
                    assert!(body.get("envelope").is_none());
                    let (size,color) = if step == 9 { ((16,8),[0,255,0,255]) }
                        else if step == 7 { ((32,32),[0,0,255,255]) }
                        else { ((32,32),[0,255,0,255]) };
                    digest = pixels(&body,size.0,size.1,color,if step==5 {"update-1"} else {"update-2"});
                    assert_eq!(body["digest"], digest);
                    revision += 1;
                    if mode == Mode::DurableStop {
                        update_received.as_ref().unwrap().send(()).unwrap();
                        // 无响应的在途更新；只有宿主取消原socket才结束，绝不注入用户输入。
                        let mut byte = [0];
                        assert!(stream.read(&mut byte).map_or(true, |count| count == 0));
                        continue;
                    }
                    if mode == Mode::Lost { drop(stream); continue; }
                } else {
                    assert!(head.starts_with("POST /v1/projections/read "));
                    // 断言非Create重试：读的是本地canonical已接受版本，允许远端领先一次。
                    assert!(body["knownRevision"].as_u64().unwrap() >= 1);
                    if mode == Mode::Offline { reply(&mut stream,503,b"temporary"); continue; }
                    if mode == Mode::PeerRevoked { reply(&mut stream,403,br#"{"error":{"code":"projection_peer_revoked"}}"#); continue; }
                }
            }
            let mut value = if mode == Mode::Malformed && step >= 5 {
                metadata(&envelope,revision,&"c".repeat(64))
            } else { metadata(&envelope,revision,&digest) };
            if offline {
                value["route"] = "offline_peer".into();
                value["peerId"] = if mode == Mode::WrongPeer && step >= 5 {format!("loom-{}", "c".repeat(64))} else {peer.clone()}.into();
                value["delivery"]["targetDeviceId"] = target.clone().into();
            }
            reply(&mut stream,200,&serde_json::to_vec(&value).unwrap());
        }
        listener
    }).unwrap()
}
