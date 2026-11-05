//! 完整控制加入 + Device WSS + 原生像素发布 + 撤权回收的真实回环测试。
use super::*;
use crate::{
    byte_budget::Budget,
    live_relay_frame::{Codec, ColorSpace, Metadata},
    live_relay_viewer_frames::Cursor,
};

fn frame() -> Vec<u8> {
    let mut bytes = Metadata {
        epoch: 1,
        frame_id: 1,
        capture_timestamp_ms: 0,
        encode_timestamp_ms: 0,
        width: 1,
        height: 1,
        dropped_frames: 0,
        keyframe: true,
        color_space: ColorSpace::Srgb,
        codec: Codec::RawBgra,
    }
    .encode_header(4)
    .unwrap()
    .to_vec();
    bytes.extend_from_slice(&[1, 2, 3, 255]);
    bytes
}
pub(super) fn upgrade_media(
    stream: StreamOwned<ServerConnection, BridgeStream>,
    head: &str,
) -> tungstenite::WebSocket<StreamOwned<ServerConnection, BridgeStream>> {
    upgrade_media_at(stream, head, 0)
}
pub(super) fn upgrade_media_at(
    mut stream: StreamOwned<ServerConnection, BridgeStream>,
    head: &str,
    after: u64,
) -> tungstenite::WebSocket<StreamOwned<ServerConnection, BridgeStream>> {
    assert!(head.contains("sessionId=live%3Atest%2Fpart"));
    assert!(head.contains(&format!("afterEpoch=1&afterFrameId={after}")));
    assert!(!head.contains("deviceId="));
    let header = |name: &str| {
        head.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case(name).then(|| value.trim())
        })
    };
    assert_eq!(header("authorization"), Some("Device fixture-secret"));
    assert_eq!(header("x-loom-device-nonce").unwrap().len(), 32);
    let accept =
        tungstenite::handshake::derive_accept_key(header("sec-websocket-key").unwrap().as_bytes());
    write!(stream, "HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Accept: {accept}\r\nSec-WebSocket-Protocol: loom.live.v1\r\n\r\n").unwrap();
    stream.flush().unwrap();
    tungstenite::WebSocket::from_raw_socket(stream, tungstenite::protocol::Role::Server, None)
}
fn serve_media(stream: StreamOwned<ServerConnection, BridgeStream>, head: &str) {
    let mut socket = upgrade_media(stream, head);
    socket.send(tungstenite::Message::Binary(frame())).unwrap();
    while socket.read().is_ok() {}
}
#[test]
fn authenticated_control_and_media_publish_pixels_and_revoke_as_one_lifecycle() {
    let (listener, client, tls) = fixture();
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    let (finish_tx, finish_rx) = std::sync::mpsc::channel();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let finish = Arc::new(std::sync::Mutex::new(finish_rx));
    let peer = std::thread::spawn(move || {
        joined(&listener, &tls);
        let mut children = Vec::new();
        for _ in 0..2 {
            let stream = accept(&listener);
            let tls = tls.clone();
            let finish = finish.clone();
            let started = started_tx.clone();
            children.push(std::thread::spawn(move || {
                let mut stream = StreamOwned::new(ServerConnection::new(tls).unwrap(), stream);
                let (head, _) = request(&mut stream);
                started.send(()).unwrap();
                if head.starts_with("GET /v1/live/media?") {
                    serve_media(stream, &head);
                } else {
                    assert!(head.contains("after=7&timeoutMs=750"));
                    let _ = finish
                        .lock()
                        .unwrap()
                        .recv_timeout(std::time::Duration::from_secs(8));
                }
            }));
        }
        for child in children {
            child.join().unwrap();
        }
    });
    let budget = Budget::new(1024 * 1024);
    let mut runtime =
        Runtime::start_viewer(client, lease, target(), budget.clone(), |_| Ok(())).unwrap();
    wait(runtime.ready_handle());
    let ready = runtime.try_ready().unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let frame = loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        assert_eq!(
            unsafe {
                WaitForSingleObject(
                    HANDLE(ready.media.change_handle().as_raw_handle()),
                    remaining.as_millis().min(2000) as u32,
                )
            },
            WAIT_OBJECT_0,
            "media notification missing"
        );
        if let Some(frame) = ready
            .media
            .poll_frame(Cursor {
                epoch: 1,
                frame_id: 0,
            })
            .unwrap()
        {
            break frame;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "media was not published: {:?}",
            runtime.try_join()
        );
    };
    assert_eq!(frame.metadata().frame_id, 1);
    drop(frame);
    // 媒体先产帧不代表控制请求头已到达；先完成夹具握手再测试撤权，避免把
    // 合法取消中的 EOF 当作服务端 request() 必须成功读取的前提。
    for _ in 0..2 {
        started_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
    }
    drop(grant);
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(ready.media.change_handle().as_raw_handle()), 0) },
        WAIT_OBJECT_0
    );
    assert!(ready
        .media
        .poll_frame(Cursor {
            epoch: 1,
            frame_id: 1
        })
        .is_err());
    assert!(ready.media.validate(&ready.permit).is_err());
    wait(runtime.wait_handle().unwrap());
    assert_eq!(runtime.try_join(), Some(Exit::Stopped));
    assert_eq!(budget.used(), 0);
    let _ = finish_tx.send(());
    peer.join().unwrap();
}

#[test]
fn media_start_requires_bound_owner_same_origin_and_live_lease() {
    use crate::{
        device_authorization_lease::MediaError, live_relay_viewer_identity::Snapshot,
        live_relay_viewer_session::ViewerSession,
    };
    let (_listener, client, _) = fixture();
    let (other_listener, other, _) = fixture();
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    let snapshot = Snapshot::parse(&snapshot(), "live:test/part").unwrap();
    let media =
        Arc::new(ViewerSession::new(snapshot.identity().clone(), "device-1".into()).unwrap());
    media.activate(&snapshot).unwrap();
    let permit = media.admit(1).unwrap();
    let start = |client: &Client| {
        lease.start_media(
            client,
            "live:test/part",
            media.clone(),
            permit.clone(),
            Budget::new(1024),
            |_| {},
        )
    };
    assert!(matches!(start(&client), Err(MediaError::Unbound)));
    lease.bind_media(&media).unwrap();
    assert!(matches!(start(&other), Err(MediaError::Credential)));
    lease.revoke();
    assert!(matches!(start(&client), Err(MediaError::Stopped)));
    assert!(
        matches!(other_listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
    );
}
