//! Source专属TLS服务：固定授权、create/WSS/close，独立核完整帧；不修改真实配置。
use super::*;
use serde_json::{json, Value};
use std::io::Write;
use std::{
    net::TcpListener,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread::JoinHandle,
};
pub(super) struct Peer {
    pub observations: Arc<std::sync::Mutex<Vec<Value>>>,
    pub created: Arc<AtomicBool>,
    pub release: Arc<AtomicBool>,
    pub received: Arc<AtomicBool>,
    pub closed: Arc<AtomicBool>,
    pub polls: Arc<AtomicUsize>,
    done: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
impl Peer {
    pub fn start(
        listener: TcpListener,
        tls: Arc<rustls::ServerConfig>,
        hold: bool,
        close_fail: bool,
    ) -> Self {
        Self::start_inner(listener, tls, hold, close_fail, None, false)
    }
    pub fn input(
        listener: TcpListener,
        tls: Arc<rustls::ServerConfig>,
        batches: Vec<Vec<(&'static str, Value)>>,
    ) -> Self {
        Self::start_inner(listener, tls, false, false, Some(batches), false)
    }
    pub fn observation(listener: TcpListener, tls: Arc<rustls::ServerConfig>) -> Self {
        Self::start_inner(listener, tls, false, false, None, true)
    }
    fn start_inner(
        listener: TcpListener,
        tls: Arc<rustls::ServerConfig>,
        hold: bool,
        close_fail: bool,
        input: Option<Vec<Vec<(&'static str, Value)>>>,
        observing: bool,
    ) -> Self {
        let created = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(!hold));
        let received = Arc::new(AtomicBool::new(false));
        let closed = Arc::new(AtomicBool::new(false));
        let done = Arc::new(AtomicBool::new(false));
        let polls = Arc::new(AtomicUsize::new(0));
        let poll_count = polls.clone();
        let observations = Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
        let observed = observations.clone();
        let (notice, ready, frames, terminal, stop) = (
            created.clone(),
            release.clone(),
            received.clone(),
            closed.clone(),
            done.clone(),
        );
        let thread = std::thread::spawn(move || {
            for stage in 0..2 {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(tls.clone()).unwrap(),
                    accept(&listener),
                );
                let (head, _) = request(&mut stream);
                assert!(head.starts_with(if stage == 0 {
                    "POST /v1/device-sessions/challenges "
                } else {
                    "POST /v1/device-sessions "
                }));
                reply(&mut stream,200,&serde_json::to_vec(&if stage==0 {json!({"deviceId":"device-old","challengeId":"id","challenge":"value"})}
                    else {json!({"deviceId":"device-old","token":"configured-token","expiresAtMs":u64::MAX})}).unwrap());
            }
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            assert!(head.starts_with("POST /v1/live/sessions "));
            let mut snapshot =
                json!({"session":body["envelope"]["payload"]["session"],"epoch":1,"closed":false});
            assert_eq!(snapshot["session"]["sourceDeviceId"], "device-old");
            assert_eq!(
                !snapshot["session"]["interactionCapabilities"]
                    .as_array()
                    .unwrap()
                    .is_empty(),
                input.is_some()
            );
            assert_eq!(
                snapshot["session"]["observationCapabilities"],
                if observing {
                    json!(["uia_tree"])
                } else {
                    json!([])
                }
            );
            snapshot["observations"] = json!([]);
            snapshot["requesterControl"] =
                json!({"deviceId":"device-old","epoch":1,"controlSequence":1});
            notice.store(true, Ordering::Release);
            let deadline = Instant::now() + Duration::from_secs(5);
            while !ready.load(Ordering::Acquire) {
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
            }
            reply(&mut stream, 201, &serde_json::to_vec(&snapshot).unwrap());
            drop(stream);
            let mut media = None;
            let require_frame = input.is_none();
            let mut cursor = 0u64;
            let mut batches = input.unwrap_or_default().into_iter();
            while !stop.load(Ordering::Acquire) {
                assert!(Instant::now() < deadline, "source peer deadline");
                let tcp = match listener.accept() {
                    Ok((tcp, _)) => tcp,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                    Err(e) => panic!("{e}"),
                };
                let mut stream = StreamOwned::new(
                    ServerConnection::new(tls.clone()).unwrap(),
                    crate::loom_bridge_stream::BridgeStream::new(tcp, deadline).unwrap(),
                );
                let (head, body) =
                    match crate::device_session_client::tests::request_result(&mut stream) {
                        Ok(request) => request,
                        Err(error)
                            if (!require_frame || observing) && cancelled_connection(&error) =>
                        {
                            continue
                        }
                        Err(error) => panic!("source HTTP read: {error}"),
                    };
                assert!(head
                    .to_ascii_lowercase()
                    .contains("authorization: device configured-token"));
                if head.starts_with("GET /v1/live/media?") {
                    assert!(media.is_none());
                    assert!(head.contains("role=source"));
                    assert!(!head.contains("deviceId="));
                    let key = head
                        .lines()
                        .find_map(|line| {
                            line.split_once(':')
                                .filter(|(k, _)| k.eq_ignore_ascii_case("sec-websocket-key"))
                                .map(|(_, v)| v.trim())
                        })
                        .unwrap();
                    let accept = tungstenite::handshake::derive_accept_key(key.as_bytes());
                    write!(stream,"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Accept: {accept}\r\nSec-WebSocket-Protocol: loom.live.v1\r\n\r\n").unwrap();
                    stream.flush().unwrap();
                    let stop = stop.clone();
                    let frames = frames.clone();
                    media = Some(std::thread::spawn(move || {
                        let mut socket = tungstenite::WebSocket::from_raw_socket(
                            stream,
                            tungstenite::protocol::Role::Server,
                            None,
                        );
                        let bytes = match socket.read() {
                            Ok(message) => message.into_data(),
                            Err(_) if !require_frame => return,
                            Err(error) => panic!("{error}"),
                        };
                        assert_eq!(bytes.len(), 64 + 16);
                        assert_eq!(&bytes[..8], b"NLLV\x01\x01\x00\x40");
                        assert_eq!(&bytes[40..48], &[0, 0, 0, 2, 0, 0, 0, 2]);
                        assert_eq!(&bytes[64..], &[17, 83, 211, 255].repeat(4));
                        frames.store(true, Ordering::Release);
                        while !stop.load(Ordering::Acquire) {
                            socket
                                .get_mut()
                                .sock
                                .operation_deadline(Duration::from_millis(100))
                                .unwrap();
                            match socket.read() {
                                Ok(tungstenite::Message::Ping(bytes)) => {
                                    let _ = socket.send(tungstenite::Message::Pong(bytes));
                                }
                                Err(tungstenite::Error::Io(e))
                                    if matches!(
                                        e.kind(),
                                        std::io::ErrorKind::TimedOut
                                            | std::io::ErrorKind::WouldBlock
                                    ) => {}
                                Err(_) | Ok(tungstenite::Message::Close(_)) => break,
                                other => panic!("unexpected source message {other:?}"),
                            }
                        }
                    }));
                } else if observing && head.contains("/observations HTTP/1.1") {
                    let sequence = snapshot["requesterControl"]["controlSequence"]
                        .as_u64()
                        .unwrap()
                        + 1;
                    assert_eq!(body["envelope"]["sequence"], sequence);
                    let value = &body["envelope"]["payload"];
                    let value_sequence = value["sequence"].as_u64().unwrap();
                    let values = snapshot["observations"].as_array_mut().unwrap();
                    if let Some(old) = values
                        .iter_mut()
                        .find(|o| o["observationId"] == value["observationId"])
                    {
                        assert_eq!(value_sequence, old["sequence"].as_u64().unwrap() + 1);
                        *old = value.clone();
                    } else {
                        assert_eq!(value_sequence, 1);
                        values.push(value.clone());
                    }
                    observed.lock().unwrap().push(value.clone());
                    snapshot["requesterControl"]["controlSequence"] = json!(sequence);
                    let _=crate::device_session_client::tests::reply_result(&mut stream,202,&serde_json::to_vec(&json!({"protocolVersion":"loom.live.v1","acceptedSequence":sequence,"acceptedObservationSequence":value_sequence})).unwrap());
                } else if observing
                    && head.starts_with("GET /v1/live/sessions/")
                    && !head.contains("/events?")
                {
                    reply(&mut stream, 200, &serde_json::to_vec(&snapshot).unwrap());
                } else if head.starts_with("GET ") && head.contains("/events?") {
                    assert!(head.contains(&format!("after={cursor}&timeoutMs=750")));
                    poll_count.fetch_add(1, Ordering::AcqRel);
                    let mut rows = Vec::new();
                    if cursor == 0 {
                        rows.push(("session_state",json!({"revision":1,"visibility":"visible","viewers":[],"controllerDeviceId":null,"reason":"created"})));
                    }
                    rows.extend(batches.next().unwrap_or_default());
                    let events=rows.into_iter().map(|(kind,payload)|{cursor+=1;json!({"protocolVersion":"loom.live.v1","sessionId":snapshot["session"]["sessionId"],"epoch":1,"sequence":cursor,"messageType":kind,"payload":payload})}).collect::<Vec<_>>();
                    if events.is_empty() {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    if let Err(error)=crate::device_session_client::tests::reply_result(&mut stream,200,&serde_json::to_vec(&json!({"protocolVersion":"loom.live.v1","next":cursor,"reset":false,"events":events})).unwrap()) {
                        assert!(!require_frame&&cancelled_connection(&error),"source events response: {error}");
                    }
                } else {
                    assert!(head.contains("/close HTTP/1.1"));
                    assert_eq!(body["envelope"]["messageType"], "session_end");
                    assert_eq!(
                        body["envelope"]["sequence"],
                        snapshot["requesterControl"]["controlSequence"]
                            .as_u64()
                            .unwrap()
                            + 1
                    );
                    assert_eq!(
                        body["envelope"]["sessionId"],
                        snapshot["session"]["sessionId"]
                    );
                    snapshot["closed"] = Value::Bool(true);
                    reply(
                        &mut stream,
                        if close_fail { 500 } else { 200 },
                        &serde_json::to_vec(&snapshot).unwrap(),
                    );
                    terminal.store(true, Ordering::Release);
                    break;
                }
            }
            stop.store(true, Ordering::Release);
            if let Some(media) = media {
                media.join().unwrap();
            }
        });
        Self {
            observations,
            created,
            release,
            received,
            closed,
            polls,
            done,
            thread: Some(thread),
        }
    }
    pub fn finish(&mut self) {
        self.done.store(true, Ordering::Release);
        self.release.store(true, Ordering::Release);
        self.thread.take().unwrap().join().unwrap();
    }
}
fn cancelled_connection(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::UnexpectedEof
            | std::io::ErrorKind::NotConnected
    )
}
impl Drop for Peer {
    fn drop(&mut self) {
        self.done.store(true, Ordering::Release);
        self.release.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
