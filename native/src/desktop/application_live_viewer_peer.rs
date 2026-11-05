//! 专属有界TLS/WSS夹具：独立构造目录/控制流/固定像素；不是实际Loom服务。
use super::*;
use rustls::ServerConfig;
use serde_json::{json, Value};
use std::{
    io::Write,
    net::TcpListener,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};
#[path = "application_live_viewer_art_peer.rs"]
mod art;
#[path = "application_live_viewer_input_peer.rs"]
mod input;
#[path = "application_live_viewer_observation_peer.rs"]
mod observation;
pub(super) struct Peer {
    done: Arc<AtomicBool>,
    pub attached: Arc<AtomicBool>,
    pub streams: Arc<std::sync::atomic::AtomicUsize>,
    pub controls: Arc<std::sync::atomic::AtomicUsize>,
    pub inputs: Arc<std::sync::Mutex<Vec<Value>>>,
    pub semantic: Arc<std::sync::atomic::AtomicUsize>,
    thread: Option<JoinHandle<()>>,
}
fn snapshot(member: bool) -> Value {
    json!({"session":{"protocolVersion":"loom.live.v1","sessionId":"live:viewer","sourceDeviceId":"source:1","sourceHookId":"hook:1","revision":2,"viewerDevices":if member {vec!["device-old"]} else {vec![]},"observationCapabilities":[]},"epoch":1,"closed":false,"observations":[]})
}
fn history(reset: bool) -> Value {
    json!({"protocolVersion":"loom.live.v1","next":7,"reset":reset,"events":[{"protocolVersion":"loom.live.v1","sessionId":"live:viewer","epoch":1,"sequence":7,"messageType":"session_state","payload":{"revision":2,"reason":"viewer_joined","visibility":"visible","viewers":["device-old"]}}]})
}
fn frame() -> Vec<u8> {
    use crate::live_relay_frame::*;
    let mut frame = Metadata {
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
    frame.extend([17, 83, 211, 255]);
    frame
}
impl Peer {
    pub fn start(listener: TcpListener, tls: Arc<ServerConfig>, hold_attach: bool) -> Self {
        Self::start_mode(listener, tls, hold_attach, false, None)
    }
    pub fn start_art(listener: TcpListener, tls: Arc<ServerConfig>, hold_attach: bool) -> Self {
        Self::start_mode(listener, tls, hold_attach, true, None)
    }
    pub fn start_input(listener: TcpListener, tls: Arc<ServerConfig>, mode: u8) -> Self {
        Self::start_mode(listener, tls, false, false, Some(mode))
    }
    pub fn start_art_input(listener: TcpListener, tls: Arc<ServerConfig>, mode: u8) -> Self {
        Self::start_mode(listener, tls, false, true, Some(mode))
    }
    pub fn art_authority(origin: &str) -> crate::art_issuer_trust::Authority {
        art::authority(origin)
    }
    fn start_mode(
        listener: TcpListener,
        tls: Arc<ServerConfig>,
        hold_attach: bool,
        local_art: bool,
        interactive: Option<u8>,
    ) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let stop = done.clone();
        let attached = Arc::new(AtomicBool::new(false));
        let notice = attached.clone();
        let streams = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let opened = streams.clone();
        let controls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let inputs = Arc::new(std::sync::Mutex::new(Vec::new()));
        let changed = controls.clone();
        let received = inputs.clone();
        let semantic = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let semantic_step = semantic.clone();
        let thread = std::thread::spawn(move || {
            for stage in 0..7 {
                if local_art && stage == 2 {
                    art::mount(&listener, &tls);
                }
                let mut stream = StreamOwned::new(
                    ServerConnection::new(tls.clone()).unwrap(),
                    accept(&listener),
                );
                let (head, body) = request(&mut stream);
                if stage >= 2 {
                    assert!(head
                        .to_ascii_lowercase()
                        .contains("authorization: device configured-token"));
                }
                let mut response = match stage {
                    0 => {
                        assert!(head.starts_with("POST /v1/device-sessions/challenges "));
                        json!({"deviceId":"device-old","challengeId":"id","challenge":"value"})
                    }
                    1 => {
                        assert!(head.starts_with("POST /v1/device-sessions "));
                        json!({"deviceId":"device-old","token":"configured-token","expiresAtMs":u64::MAX})
                    }
                    2 | 3 => {
                        assert!(head.starts_with("GET /v1/live/sessions "));
                        json!({"protocolVersion":"loom.live.v1","sessions":[snapshot(false)]})
                    }
                    4 => {
                        if local_art {
                            assert!(head.starts_with("POST /v1/art/local/live-viewer/attach "));
                            art::check_join(&body);
                        } else {
                            assert!(head.starts_with("POST /v1/live/sessions/live:viewer/viewers "));
                            assert_eq!(body["surfaceInstanceId"], "instance:test");
                            assert_eq!(body["attachmentId"], "attachment:test");
                            assert!(body.get("requireExistingMembership").is_none());
                        }
                        assert_eq!(
                            body["envelope"]["payload"]["responderDeviceId"],
                            "device-old"
                        );
                        notice.store(true, Ordering::Release);
                        if hold_attach {
                            while !stop.load(Ordering::Acquire) {
                                std::thread::sleep(std::time::Duration::from_millis(5));
                            }
                            return;
                        }
                        snapshot(true)
                    }
                    5 => {
                        assert!(head.contains("after=0&timeoutMs=0"));
                        history(true)
                    }
                    _ => {
                        assert!(head.contains("after=6&timeoutMs=750"));
                        history(false)
                    }
                };
                if interactive.is_some() {
                    let caps = json!([
                        "pointer_move",
                        "pointer_button",
                        "double_click",
                        "drag",
                        "wheel",
                        "keyboard",
                        "cancel"
                    ]);
                    if stage == 2 || stage == 3 {
                        response["sessions"][0]["session"]["interactionCapabilities"] = caps;
                    } else if stage == 4 {
                        response["session"]["interactionCapabilities"] = caps;
                    }
                }
                if interactive == Some(9) {
                    observation::decorate(&mut response, stage);
                }
                reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
            }
            if let Some(mode) = interactive {
                input::serve(
                    listener,
                    tls,
                    stop,
                    opened,
                    changed,
                    received,
                    (mode, local_art, semantic_step),
                );
                return;
            }
            let mut children = Vec::new();
            for _ in 0..2 {
                let stream = accept(&listener);
                let tls = tls.clone();
                let stop = stop.clone();
                let opened = opened.clone();
                children.push(std::thread::spawn(move || {
                    let mut stream=StreamOwned::new(ServerConnection::new(tls).unwrap(),stream);
                    let (head,_)=request(&mut stream);
                    assert!(head.to_ascii_lowercase().contains("authorization: device configured-token"));
                    opened.fetch_add(1,Ordering::AcqRel);
                    if head.starts_with("GET /v1/live/media?") {
                        assert!(head.contains("sessionId=live%3Aviewer")); assert!(!head.contains("deviceId="));
                        let nonce=head.lines().find_map(|line|line.split_once(':').filter(|(key,_)|key.eq_ignore_ascii_case("x-loom-device-nonce")).map(|(_,value)|value.trim())).unwrap();
                        assert_eq!(nonce.len(),32);
                        let key=head.lines().find_map(|line|line.split_once(':').filter(|(key,_)|key.eq_ignore_ascii_case("sec-websocket-key")).map(|(_,value)|value.trim())).unwrap();
                        let accept=tungstenite::handshake::derive_accept_key(key.as_bytes());
                        write!(stream,"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Accept: {accept}\r\nSec-WebSocket-Protocol: loom.live.v1\r\n\r\n").unwrap(); stream.flush().unwrap();
                        let mut socket=tungstenite::WebSocket::from_raw_socket(stream,tungstenite::protocol::Role::Server,None);
                        socket.send(tungstenite::Message::Binary(frame())).unwrap();
                        while !stop.load(Ordering::Acquire) { std::thread::sleep(std::time::Duration::from_millis(5)); }
                    } else {
                        assert!(head.contains("after=7&timeoutMs=750"));
                        while !stop.load(Ordering::Acquire) { std::thread::sleep(std::time::Duration::from_millis(5)); }
                    }
                }));
            }
            for child in children {
                child.join().unwrap();
            }
        });
        Self {
            done,
            attached,
            streams,
            controls,
            inputs,
            semantic,
            thread: Some(thread),
        }
    }
    pub fn finish(&mut self) {
        self.done.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        self.done.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
