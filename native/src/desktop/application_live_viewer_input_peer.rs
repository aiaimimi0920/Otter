//! 独立TLS wire oracle，只记录自有viewer消息；无真实设备/配置或全局输入。
use super::*;
use std::{
    sync::{atomic::AtomicUsize, Mutex},
    time::{Duration, Instant},
};
fn state_event(sequence: u64, revision: u64, owned: bool) -> Value {
    json!({"protocolVersion":"loom.live.v1","sessionId":"live:viewer","epoch":1,"sequence":sequence,
        "messageType":"session_state","payload":{"revision":revision,"visibility":"visible","viewers":["device-old"],
            "controllerDeviceId":if owned{json!("device-old")}else{Value::Null}}})
}
pub(super) fn serve(
    listener: TcpListener,
    tls: Arc<ServerConfig>,
    stop: Arc<AtomicBool>,
    opened: Arc<AtomicUsize>,
    controls: Arc<AtomicUsize>,
    inputs: Arc<Mutex<Vec<Value>>>,
    mode: (u8, bool, Arc<AtomicUsize>),
) {
    let (mode, local_art, semantic) = mode;
    let mut events = Vec::new();
    let mut sequence = 7;
    let mut revision = 2;
    let mut actor = 1;
    let mut input_sequence = 0;
    let mut owned = false;
    let mut socket = None;
    let mut saw_events = false;
    let mut requests = 0;
    while !stop.load(Ordering::Acquire) {
        let tcp = match listener.accept() {
            Ok((tcp, _)) => tcp,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(2));
                continue;
            }
            Err(e) => panic!("input fixture accept: {e}"),
        };
        requests += 1;
        assert!(requests <= 256, "unbounded viewer fixture requests");
        let tcp = crate::loom_bridge_stream::BridgeStream::new(
            tcp,
            Instant::now() + Duration::from_secs(5),
        )
        .unwrap();
        let mut stream = StreamOwned::new(ServerConnection::new(tls.clone()).unwrap(), tcp);
        let (head, body) = match crate::device_session_client::tests::request_result(&mut stream) {
            Ok(r) => r,
            Err(e)
                if stop.load(Ordering::Acquire)
                    && matches!(
                        e.kind(),
                        std::io::ErrorKind::ConnectionAborted
                            | std::io::ErrorKind::ConnectionReset
                            | std::io::ErrorKind::UnexpectedEof
                    ) =>
            {
                break
            }
            Err(e) => panic!("input fixture request: {e}"),
        };
        assert!(head
            .to_ascii_lowercase()
            .contains("authorization: device configured-token"));
        if head.starts_with("GET /v1/live/media?") {
            assert!(socket.is_none());
            opened.fetch_add(1, Ordering::AcqRel);
            let key = head
                .lines()
                .find_map(|l| {
                    l.split_once(':')
                        .filter(|(k, _)| k.eq_ignore_ascii_case("sec-websocket-key"))
                        .map(|(_, v)| v.trim())
                })
                .unwrap();
            let accept = tungstenite::handshake::derive_accept_key(key.as_bytes());
            write!(stream,"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Accept: {accept}\r\nSec-WebSocket-Protocol: loom.live.v1\r\n\r\n").unwrap();
            stream.flush().unwrap();
            let stopped = stop.clone();
            socket = Some(std::thread::spawn(move || {
                let mut socket = tungstenite::WebSocket::from_raw_socket(
                    stream,
                    tungstenite::protocol::Role::Server,
                    None,
                );
                socket.send(tungstenite::Message::Binary(frame())).unwrap();
                while !stopped.load(Ordering::Acquire) {
                    std::thread::sleep(Duration::from_millis(5));
                }
            }));
            continue;
        }
        let mut is_input = false;
        let response = if head.starts_with("GET /v1/live/sessions/live:viewer/events?") {
            if !saw_events {
                opened.fetch_add(1, Ordering::AcqRel);
                saw_events = true;
            }
            let after = head
                .split("after=")
                .nth(1)
                .unwrap()
                .split('&')
                .next()
                .unwrap()
                .parse::<u64>()
                .unwrap();
            if mode == 9 && sequence == 7 && semantic.load(Ordering::Acquire) == 1 {
                events.extend(observation::delta());
                sequence = 9;
            }
            let batch: Vec<_> = events
                .iter()
                .filter(|event: &&Value| event["sequence"].as_u64().unwrap() > after)
                .cloned()
                .collect();
            std::thread::sleep(Duration::from_millis(5));
            json!({"protocolVersion":"loom.live.v1","reset":false,"next":sequence,"events":batch})
        } else {
            let body = if local_art {
                assert!(head.starts_with("POST /v1/art/local/live-viewer/control "));
                art::check_control(&body);
                let op = &body["operation"];
                if op["kind"] == "input" {
                    json!({"envelope":op["envelope"]})
                } else {
                    assert_eq!(op["sessionId"], "live:viewer");
                    json!({"action":op["kind"],"sequence":op["sequence"],"epoch":op["epoch"]})
                }
            } else {
                assert_eq!(body["surfaceInstanceId"], "instance:test");
                assert_eq!(body["attachmentId"], "attachment:test");
                assert!(
                    head.starts_with("POST /v1/live/sessions/live:viewer/control ")
                        || head.starts_with("POST /v1/live/sessions/live:viewer/input ")
                );
                body
            };
            if body.get("action").is_some() {
                actor += 1;
                assert_eq!(body["sequence"], actor);
                assert_eq!(body["epoch"], 1);
                owned = match body["action"].as_str().unwrap() {
                    "acquire" => true,
                    "release" => false,
                    other => panic!("unexpected action {other}"),
                };
                revision += 1;
                sequence += 1;
                events.push(state_event(sequence, revision, owned));
                controls.fetch_add(1, Ordering::AcqRel);
                let mut snapshot = snapshot(true);
                snapshot["session"]["revision"] = json!(revision);
                snapshot["session"]["controllerDevice"] = if owned {
                    json!("device-old")
                } else {
                    Value::Null
                };
                snapshot
            } else {
                is_input = true;
                assert!(owned);
                actor += 1;
                input_sequence += 1;
                let envelope = &body["envelope"];
                assert_eq!(envelope["sequence"], actor);
                assert_eq!(envelope["epoch"], 1);
                assert_eq!(envelope["sessionId"], "live:viewer");
                assert_eq!(envelope["messageType"], "input_event");
                assert_eq!(envelope["payload"]["inputSequence"], input_sequence);
                assert_eq!(envelope["payload"]["sourceDeviceId"], "device-old");
                assert!(envelope["payload"]["issuedAtMs"].as_u64().unwrap() > 0);
                inputs
                    .lock()
                    .unwrap()
                    .push(envelope["payload"]["kind"].clone());
                if mode == 7 {
                    reply(&mut stream, 500, br#"{"error":"fixture_input_failed"}"#);
                    continue;
                }
                sequence += 1;
                let mut forwarded = envelope.clone();
                forwarded["sequence"] = json!(sequence);
                events.push(forwarded);
                let accepted = sequence;
                if mode == 6 && input_sequence == 2 {
                    owned = false;
                    revision += 1;
                    sequence += 1;
                    events.push(state_event(sequence, revision, false));
                }
                json!({"protocolVersion":"loom.live.v1","acceptedSequence":accepted})
            }
        };
        let status = if is_input { 202 } else { 200 };
        if let Err(e) = crate::device_session_client::tests::reply_result(
            &mut stream,
            status,
            &serde_json::to_vec(&response).unwrap(),
        ) {
            assert!(
                stop.load(Ordering::Acquire)
                    && matches!(
                        e.kind(),
                        std::io::ErrorKind::ConnectionAborted
                            | std::io::ErrorKind::ConnectionReset
                            | std::io::ErrorKind::BrokenPipe
                    ),
                "input fixture reply: {e}"
            );
        }
    }
    if let Some(socket) = socket {
        socket.join().unwrap();
    }
}
