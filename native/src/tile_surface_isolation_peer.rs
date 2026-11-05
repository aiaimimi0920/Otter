//! 独立TLS/WSS oracle：故意挂住Art图片回复，媒体持续发包并记录真实Ping，不放宽产品期限。
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
pub(super) struct Signals {
    pub stop: AtomicBool,
    pub painted: AtomicBool,
    pub image_started: AtomicBool,
    pub release: AtomicBool,
    pub pings: AtomicUsize,
    pub connections: AtomicUsize,
    pub opens: AtomicUsize,
}
impl Signals {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            stop: AtomicBool::new(false),
            painted: AtomicBool::new(false),
            image_started: AtomicBool::new(false),
            release: AtomicBool::new(false),
            pings: AtomicUsize::new(0),
            connections: AtomicUsize::new(0),
            opens: AtomicUsize::new(0),
        })
    }
}
fn resource(pixel: [u8; 4]) -> Value {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use sha2::{Digest, Sha256};
    let bytes = pixel.repeat(4);
    json!({"protocolVersion":"loom.wall.v1","resource":{"resourceId":format!("sha256:{:x}",Sha256::digest(&bytes)),"kind":"image","mime":"application/x-neuro-rgba8","size":16,"width":2,"height":2},"dataBase64":STANDARD.encode(bytes)})
}
fn header<'a>(head: &'a str, name: &str) -> &'a str {
    head.lines()
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case(name).then_some(value.trim())
        })
        .unwrap()
}
fn media(mut stream: StreamOwned<ServerConnection, BridgeStream>, head: &str, signals: &Signals) {
    use tungstenite::{protocol::Role, Message, WebSocket};
    assert!(head.starts_with("GET /v1/walls/live/media?"));
    assert_eq!(header(head, "Sec-WebSocket-Protocol"), "loom.wall.media.v1");
    let accept =
        tungstenite::handshake::derive_accept_key(header(head, "Sec-WebSocket-Key").as_bytes());
    write!(stream,"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Accept: {accept}\r\nSec-WebSocket-Protocol: loom.wall.media.v1\r\n\r\n").unwrap();
    stream.flush().unwrap();
    signals.connections.fetch_add(1, Ordering::SeqCst);
    let mut socket = WebSocket::from_raw_socket(stream, Role::Server, None);
    let end = Instant::now() + Duration::from_secs(15);
    let mut id = 0u64;
    while !signals.stop.load(Ordering::Acquire) && Instant::now() < end {
        id += 1;
        let now = crate::tile_runtime::now_ms();
        let mut bytes = vec![0u8; 84];
        bytes[..8].copy_from_slice(b"NLWM\x01\x01\x00\x50");
        for (offset, value) in [(8, 1), (16, id), (24, now), (32, now), (64, now), (72, now)] {
            bytes[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
        }
        for (offset, value) in [(40, 1u32), (44, 1), (52, 4)] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
        }
        bytes[56..58].copy_from_slice(&[1, 1]);
        bytes[80..].copy_from_slice(&[17, 83, 211, 255]);
        socket
            .get_mut()
            .sock
            .operation_deadline(Duration::from_millis(20))
            .unwrap();
        if socket.send(Message::Binary(bytes)).is_err() {
            break;
        }
        match socket.read() {
            Ok(Message::Ping(payload)) => {
                signals.pings.fetch_add(1, Ordering::SeqCst);
                if socket.send(Message::Pong(payload)).is_err() {
                    break;
                }
            }
            Ok(Message::Close(_)) => break,
            Err(tungstenite::Error::Io(e))
                if matches!(
                    e.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) => {}
            Ok(_) => {}
            _ => break,
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
pub(super) fn serve(
    listener: TcpListener,
    config: Arc<ServerConfig>,
    endpoint: crate::tile_wire::Endpoint,
    signals: Arc<Signals>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut threads = Vec::new();
        let end = Instant::now() + Duration::from_secs(18);
        let began = crate::tile_runtime::now_ms();
        while !signals.stop.load(Ordering::Acquire) {
            assert!(Instant::now() < end, "isolated peer deadline");
            let (tcp, _) = match listener.accept() {
                Ok(value) => value,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(e) => panic!("isolated peer: {e}"),
            };
            assert!(threads.len() < 128, "fixture connection bound");
            let (config, endpoint, flags) = (config.clone(), endpoint.clone(), signals.clone());
            threads.push(std::thread::spawn(move || {
                let mut stream = StreamOwned::new(ServerConnection::new(config).unwrap(), BridgeStream::new(tcp, Instant::now()+Duration::from_secs(8)).unwrap());
                let Ok((head, body)) = request_result(&mut stream) else { return; };
                assert_eq!(header(&head, "Authorization"), "Device fixture-secret");
                if head.starts_with("GET /v1/walls/live/media?") { media(stream, &head, &flags); return; }
                let art_image = resource([0, 255, 0, 255]);
                let picture = resource([255, 0, 0, 255]);
                let response = if head.starts_with("GET /v1/walls/state ") {
                    let mut catalog = crate::tile_tests::catalog(&endpoint, 2);
                    catalog["layouts"][0]["bounds"]["width"] = 6.into();
                    catalog["layouts"][0]["tiles"][0]["rect"]["width"] = 6.into();
                    catalog["layouts"][0]["placements"] = json!([
                        {"placementId":"p","source":{"kind":"image","id":picture["resource"]["resourceId"]},"rect":{"x":0,"y":0,"width":2,"height":2},"sourceCrop":{"x":0,"y":0,"width":1,"height":1},"zIndex":0,"interactive":false},
                        {"placementId":"l","source":{"kind":"live","id":"live:one"},"rect":{"x":2,"y":0,"width":2,"height":2},"sourceCrop":{"x":0,"y":0,"width":1,"height":1},"zIndex":1,"interactive":false},
                        {"placementId":"a","source":{"kind":"surface","id":"art"},"rect":{"x":4,"y":0,"width":2,"height":2},"sourceCrop":{"x":0,"y":0,"width":1,"height":1},"zIndex":2,"interactive":false}]);
                    catalog["timing"]["serverTimeMs"] = crate::tile_runtime::now_ms().into();
                    catalog["timing"]["scenes"][0]["preparedAtMs"] = began.into();
                    catalog["timing"]["scenes"][0]["activateAtMs"] = began.into();
                    catalog
                } else if head.starts_with("POST /v1/walls/connect ") {
                    json!({"protocolVersion":"loom.wall.v1","leaseId":"lease","leaseTtlMs":15000})
                } else if head.starts_with("POST /v1/walls/images/read ") {
                    assert_eq!(body["resourceId"], picture["resource"]["resourceId"]); picture
                } else if head.starts_with("POST /v1/walls/surfaces/image ") {
                    flags.image_started.store(true, Ordering::Release);
                    let end = Instant::now()+Duration::from_secs(7);
                    while !flags.release.load(Ordering::Acquire) { assert!(Instant::now() < end, "test must release held response"); std::thread::sleep(Duration::from_millis(5)); }
                    assert_eq!(body["resourceId"], art_image["resource"]["resourceId"]); art_image
                } else if head.starts_with("POST /v1/walls/surfaces/open ") || head.starts_with("POST /v1/walls/surfaces/state ") {
                    let open = head.starts_with("POST /v1/walls/surfaces/open ");
                    if open { assert_eq!(flags.opens.fetch_add(1, Ordering::SeqCst), 0); }
                    let view = if open { json!({"binding":body["binding"],"instanceId":"art","attachmentId":"view"}) } else { body["view"].clone() };
                    let update = !open && flags.painted.load(Ordering::Acquire);
                    let mut snapshot = json!({"protocolVersion":"loom.surface.v1","instanceId":"art","attachmentId":"view","artId":"fixture","artVersion":"1","revision":if update{2}else{1},"runtime":"declarative","scene":{"id":"root","type":"view","layout":{"width":2,"height":2}},"resources":[],"resourceLeases":[]});
                    if update {
                        snapshot["resources"] = json!([art_image["resource"]]);
                        snapshot["scene"] = json!({"id":"root","type":"image","props":{"resourceId":art_image["resource"]["resourceId"]},"layout":{"width":2,"height":2}});
                    }
                    if body["snapshotRevision"] == 2 { snapshot = Value::Null; }
                    json!({"protocolVersion":"loom.wall.v1","view":view,"width":2,"height":2,"generation":1,"sequence":0,"snapshot":snapshot,"preview":null,"result":null,"confirmations":[],"pending":[],"failure":null})
                } else {
                    assert!(head.starts_with("POST /v1/walls/heartbeat ") || head.starts_with("POST /v1/walls/disconnect ") || head.starts_with("POST /v1/walls/surfaces/close "), "unexpected {head}");
                    json!({"protocolVersion":"loom.wall.v1","accepted":true})
                };
                // 取消用例允许客户端已断开；服务端仍尝试交付迟到内容。
                let _ = reply_result(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
    })
}
