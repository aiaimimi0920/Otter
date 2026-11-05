use super::*;
use crate::{
    live_relay_frame::{Codec, ColorSpace, Metadata},
    live_relay_viewer_frames::Cursor,
    live_relay_viewer_identity::Snapshot,
    live_relay_viewer_session::ViewerSession,
};
use std::{
    net::{TcpListener, TcpStream},
    time::{Duration, Instant},
};
fn session() -> Arc<ViewerSession> {
    let bytes=serde_json::to_vec(&serde_json::json!({"session":{"protocolVersion":"loom.live.v1","sessionId":"live:test","sourceDeviceId":"source:test","sourceHookId":"hook:test","revision":1,"viewerDevices":["viewer:test"]},"epoch":1,"closed":false})).unwrap();
    let snapshot = Snapshot::parse(&bytes, "live:test").unwrap();
    let s =
        Arc::new(ViewerSession::new(snapshot.identity().clone(), "viewer:test".into()).unwrap());
    s.activate(&snapshot).unwrap();
    s
}
fn raw() -> Vec<u8> {
    let m = Metadata {
        epoch: 1,
        frame_id: 1,
        capture_timestamp_ms: 1,
        encode_timestamp_ms: 1,
        width: 1,
        height: 1,
        dropped_frames: 0,
        keyframe: true,
        color_space: ColorSpace::Srgb,
        codec: Codec::RawBgra,
    };
    let mut bytes = m.encode_header(4).unwrap().to_vec();
    bytes.extend_from_slice(&[10, 20, 30, 255]);
    bytes
}
#[test]
fn adopted_message_keeps_payload_pointer_and_accounts_full_capacity() {
    let mut bytes = raw();
    bytes.reserve(512);
    let capacity = bytes.capacity();
    let pointer = bytes[HEADER_LEN..].as_ptr();
    let budget = Budget::new(capacity);
    let frame = Frame::from_message(bytes, Profile::Legacy, &budget).unwrap();
    assert_eq!(frame.payload().as_ptr(), pointer);
    assert_eq!(budget.used(), capacity);
    drop(frame);
    assert_eq!(budget.used(), 0);
    assert!(Frame::from_message(raw(), Profile::Legacy, &Budget::new(4)).is_err());
}
#[test]
fn actual_loopback_websocket_publishes_to_worker_and_joins_after_peer_close() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (release, hold) = std::sync::mpsc::channel();
    listener.set_nonblocking(true).unwrap();
    let peer = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let (tcp, _) = loop {
            match listener.accept() {
                Ok(pair) => break pair,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline);
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(e) => panic!("fixture accept: {e}"),
            }
        };
        // Windows 的 accepted socket 可能继承 listener 的非阻塞模式。
        tcp.set_nonblocking(false).unwrap();
        tcp.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        tcp.set_write_timeout(Some(Duration::from_secs(3))).unwrap();
        let mut socket = tungstenite::accept(tcp).unwrap();
        let Message::Ping(payload) = socket.read().unwrap() else {
            panic!("expected active ping");
        };
        assert_eq!(payload.len(), 8);
        socket.send(Message::Pong(payload)).unwrap();
        socket.send(Message::Binary(raw())).unwrap();
        hold.recv_timeout(Duration::from_secs(5)).unwrap();
        socket.close(None).unwrap();
    });
    let tcp = TcpStream::connect(address).unwrap();
    tcp.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    tcp.set_write_timeout(Some(Duration::from_secs(3))).unwrap();
    let (mut socket, _) = tungstenite::client::client_with_config(
        format!("ws://{address}/media"),
        tcp,
        Some(config()),
    )
    .unwrap();
    socket
        .get_mut()
        .set_read_timeout(Some(Duration::from_millis(250)))
        .unwrap();
    socket
        .get_mut()
        .set_write_timeout(Some(Duration::from_millis(250)))
        .unwrap();
    let session = session();
    let mut worker = Worker::start(session.clone(), session.admit(1).unwrap()).unwrap();
    let budget = Budget::new(1024);
    let mut liveness = Liveness::new(Instant::now() - Duration::from_secs(2));
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let step = service(
            &mut socket,
            Profile::Legacy,
            Authorization::Other,
            &budget,
            &worker,
            &mut liveness,
        )
        .unwrap();
        if step == Step::Queued {
            break;
        }
        assert_eq!(step, Step::Idle);
        assert!(Instant::now() < deadline);
    }
    let output = loop {
        if let Some(output) = session
            .take(Cursor {
                epoch: 1,
                frame_id: 1,
            })
            .unwrap()
        {
            break output;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(&**output.pixels(), &[10, 20, 30, 255]);
    assert_eq!(budget.used(), 0);
    assert!(liveness.rtt().is_some());
    release.send(()).unwrap();
    loop {
        let step = service(
            &mut socket,
            Profile::Legacy,
            Authorization::Other,
            &budget,
            &worker,
            &mut liveness,
        )
        .unwrap();
        if step == Step::Closed {
            break;
        }
        assert_eq!(step, Step::Idle);
        assert!(Instant::now() < deadline);
    }
    while worker.try_join().is_none() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    peer.join().unwrap();
}
#[test]
fn only_exact_device_policy_close_is_terminal_and_unbounded_socket_is_rejected() {
    use tungstenite::protocol::Role;
    for (auth, code, reason, terminal) in [
        (
            Authorization::Device,
            1008u16,
            "live_media_device_revoked",
            true,
        ),
        (
            Authorization::Other,
            1008,
            "live_media_device_revoked",
            false,
        ),
        (
            Authorization::Device,
            1000,
            "live_media_device_revoked",
            false,
        ),
        (
            Authorization::Device,
            1008,
            "live_media_device_revoked ",
            false,
        ),
    ] {
        let s = session();
        let worker = Worker::start(s.clone(), s.admit(1).unwrap()).unwrap();
        let mut bytes = vec![0x88, (2 + reason.len()) as u8];
        bytes.extend_from_slice(&code.to_be_bytes());
        bytes.extend_from_slice(reason.as_bytes());
        let mut socket =
            WebSocket::from_raw_socket(std::io::Cursor::new(bytes), Role::Client, Some(config()));
        assert_eq!(
            service(
                &mut socket,
                Profile::Legacy,
                auth,
                &Budget::new(1024),
                &worker,
                &mut Liveness::new(Instant::now())
            ),
            Ok(if terminal {
                Step::DeviceRevoked
            } else {
                Step::Closed
            })
        );
        assert!(matches!(
            (s.admit(1), terminal),
            (Err(crate::live_relay_viewer_session::Error::Closed), true)
                | (
                    Err(crate::live_relay_viewer_session::Error::Suspended),
                    false
                )
        ));
    }
    let s = session();
    let worker = Worker::start(s.clone(), s.admit(1).unwrap()).unwrap();
    let mut limits = config();
    limits.max_message_size = None;
    let mut socket = WebSocket::from_raw_socket(
        std::io::Cursor::new(Vec::<u8>::new()),
        Role::Client,
        Some(limits),
    );
    assert_eq!(
        service(
            &mut socket,
            Profile::Legacy,
            Authorization::Other,
            &Budget::new(1024),
            &worker,
            &mut Liveness::new(Instant::now())
        ),
        Err(Error::Limits)
    );
}
