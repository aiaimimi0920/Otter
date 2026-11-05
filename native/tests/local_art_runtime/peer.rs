//! 有界回环 HTTPS peer；仅合成设备，证书不安装系统，停止后保留端口但拒绝接收。
use super::native;
use super::package;
#[path = "projection_peer.rs"]
pub mod projection;
use base64::{engine::general_purpose::STANDARD, Engine};
use ring::signature::KeyPair;
use rustls::{pki_types::PrivatePkcs8KeyDer, ServerConfig, ServerConnection, StreamOwned};
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
#[derive(Default)]
pub struct State {
    pub requests: Vec<String>,
    pub release: bool,
    pub sequence: u64,
    pub submit_status: Option<u16>,
    pub failed_owner: Option<Value>,
    pub image_output_owner: Option<Value>,
    pub projection: projection::State,
}
pub struct Peer {
    pub origin: String,
    pub state: Arc<Mutex<State>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<TcpListener>>,
    held: Option<TcpListener>,
}
impl Peer {
    pub fn new(root: &Path) -> Self {
        Self::with_source(root, package::SOURCE)
    }
    pub fn with_source(root: &Path, source: &str) -> Self {
        // 调用方可传公开SDK示例的已核字节；线程拥有这份有界测试源码，不泄漏static字符串。
        assert!(!source.is_empty() && source.len() <= 1024 * 1024);
        let source = source.to_owned();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let origin = format!(
            "https://127.0.0.1:{}",
            listener.local_addr().unwrap().port()
        );
        let rcgen::CertifiedKey { cert, key_pair } =
            rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
        std::fs::write(
            root.join("ca.pem"),
            format!(
                "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----\n",
                STANDARD.encode(cert.der())
            ),
        )
        .unwrap();
        let config =
            ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .unwrap()
                .with_no_client_auth()
                .with_single_cert(
                    vec![cert.der().clone()],
                    PrivatePkcs8KeyDer::from(key_pair.serialize_der()).into(),
                )
                .unwrap();
        let store = native::device_identity_store::Store::new(root).unwrap();
        let mut loaded = store.load_or_create().unwrap();
        store
            .bind_and_save(&mut loaded, "device-product-art")
            .unwrap();
        let device_key = loaded.identity().public_key().unwrap();
        let state = Arc::new(Mutex::new(State::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let (shared, stopped, address) = (state.clone(), stop.clone(), origin.clone());
        let thread = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(120);
            let config = Arc::new(config);
            let mut nonces = BTreeSet::new();
            while !stopped.load(Ordering::Acquire) {
                assert!(
                    Instant::now() < deadline,
                    "product TLS peer lifetime exceeded"
                );
                let tcp = match listener.accept() {
                    Ok((tcp, _)) => tcp,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("product TLS accept: {error}"),
                };
                // Windows accepted socket继承监听器的非阻塞模式；TLS read_exact需要明确恢复阻塞。
                tcp.set_nonblocking(false).unwrap();
                tcp.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                tcp.set_write_timeout(Some(Duration::from_secs(3))).unwrap();
                let mut stream =
                    StreamOwned::new(ServerConnection::new(config.clone()).unwrap(), tcp);
                let (path, headers, body) = request(&mut stream);
                eprintln!("art-product-peer: {path}");
                {
                    let mut state = shared.lock().unwrap();
                    assert!(state.requests.len() < 128, "bounded product request count");
                    state.requests.push(path.clone());
                }
                if path.starts_with("/v1/art/") || path.starts_with("/v1/projections/") {
                    assert!(headers.contains("authorization: device product-art-test-only\r\n"));
                    let nonce = headers
                        .lines()
                        .find(|s| s.starts_with("x-loom-device-nonce: "))
                        .unwrap()
                        .to_owned();
                    assert!(nonces.insert(nonce));
                }
                let projection = (path.starts_with("/v1/projections/")
                    && path != "/v1/projections/inbox")
                    .then(|| shared.lock().unwrap().projection.respond(&path, &body));
                let status = if let Some((status, _)) = &projection {
                    *status
                } else if path == "/v1/art/local/service/submit" {
                    let mut state = shared.lock().unwrap();
                    let status = state.submit_status.take().unwrap_or(200);
                    if status != 200 {
                        state.failed_owner = Some(body["owner"].clone());
                    }
                    status
                } else {
                    200
                };
                let data = if let Some((_, data)) = projection {
                    data
                } else if status == 200 {
                    response(&path, &body, &address, &device_key, &shared, &source)
                } else {
                    json!({"error":"temporary test-only refusal"})
                };
                let bytes = serde_json::to_vec(&data).unwrap();
                write!(stream,"HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len()).unwrap();
                stream.write_all(&bytes).unwrap();
                stream.flush().unwrap();
            }
            listener
        });
        Self {
            origin,
            state,
            stop,
            thread: Some(thread),
            held: None,
        }
    }
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            self.held = Some(thread.join().expect("product TLS peer failed"));
        }
    }
    pub fn assert_no_offline_connections(&self) {
        let listener = self.held.as_ref().expect("online peer must be stopped");
        assert!(
            matches!(listener.accept(),Err(error) if error.kind()==std::io::ErrorKind::WouldBlock),
            "offline product attempted a connection to the held test port"
        );
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            state.release = true;
        }
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            if std::thread::panicking() {
                let _ = thread.join();
            } else {
                self.held = Some(thread.join().unwrap());
            }
        }
    }
}
fn request(stream: &mut StreamOwned<ServerConnection, TcpStream>) -> (String, String, Value) {
    let mut bytes = Vec::new();
    let mut byte = [0];
    while !bytes.ends_with(b"\r\n\r\n") {
        assert!(bytes.len() < 16 * 1024);
        stream.read_exact(&mut byte).unwrap();
        bytes.push(byte[0]);
    }
    let headers = String::from_utf8(bytes).unwrap().to_ascii_lowercase();
    let length: usize = headers
        .lines()
        .find_map(|s| s.strip_prefix("content-length: "))
        .unwrap()
        .parse()
        .unwrap();
    assert!(length <= 32 * 1024);
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).unwrap();
    (
        headers.split_whitespace().nth(1).unwrap().into(),
        headers,
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        },
    )
}
fn response(
    path: &str,
    body: &Value,
    origin: &str,
    key: &str,
    state: &Mutex<State>,
    source: &str,
) -> Value {
    match path {
        "/v1/art/local/issuer" => {
            json!({"algorithm":"ed25519", "keyId":"product-issuer", "publicKey":STANDARD.encode(package::key().public_key().as_ref())})
        }
        "/v1/projections/inbox" => {
            assert_eq!(body, &json!({"policy":"confirm"}));
            json!({"invitations":[]})
        }
        "/v1/device-sessions/challenges" => {
            json!({"deviceId":"device-product-art","challengeId":"id","challenge":"value"})
        }
        "/v1/device-sessions" => {
            json!({"deviceId":"device-product-art","token":"product-art-test-only","expiresAtMs":9007199254740991u64})
        }
        "/v1/art/local/describe" => {
            assert_eq!(*body, json!({"artId":package::ART}));
            json!({"protocolVersion":"loom.art.local.describe.v1","descriptor":{"artId":package::ART,"artVersion":"1.0.0",
                "packageDigest":"b".repeat(64),"sdkVersion":"otter.art.v1","permissions":package::permissions(),"services":[package::SERVICE]}})
        }
        "/v1/art/local/prepare-package" => package::prepare_source(body, origin, key, source),
        "/v1/art/local/resource" => {
            assert_eq!(body["protocolVersion"], "loom.art.local.resource.v1");
            let id = body["resourceId"].as_str().unwrap();
            let bytes = package::resource(id);
            json!({"protocolVersion":"loom.art.local.resource.v1","grantId":"grant:product-art","resourceId":id,"digest":package::digest(&bytes),"bytes":STANDARD.encode(bytes)})
        }
        "/v1/art/local/service/submit-image-output" => {
            assert_eq!(
                body["protocolVersion"],
                "loom.art.local.service.image-output.v1"
            );
            assert_eq!(body["call"]["input"], json!({"returnImage":true}));
            assert_eq!(body["images"].as_array().unwrap().len(), 1);
            state.lock().unwrap().image_output_owner = Some(body["call"]["owner"].clone());
            let bytes = package::resource("picture");
            json!({"protocolVersion":"loom.art.local.service.image-output.v1","reply":{
                "protocolVersion":"loom.art.local.service.v1","owner":body["call"]["owner"],"sequence":1,"status":"succeeded",
                "output":{"answer":42},"error":null,"progress":null},
                "image":{"width":2,"height":2,"digest":package::digest(&bytes),"png":STANDARD.encode(bytes)}})
        }
        "/v1/art/local/service/submit" | "/v1/art/local/service/poll" => {
            // 首次submit发布可见进度；下一次poll由测试领取进度后放行，避免刷轮询抢时序。
            if path.ends_with("/poll") {
                let deadline = Instant::now() + Duration::from_secs(8);
                while !state.lock().unwrap().release {
                    assert!(
                        Instant::now() < deadline,
                        "product progress observation not released"
                    );
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
            let mut state = state.lock().unwrap();
            state.sequence += 1;
            if path.ends_with("/submit") {
                assert_eq!(body["input"], json!({"answer":42}));
            }
            let mut value = json!({"protocolVersion":"loom.art.local.service.v1","owner":body["owner"],"sequence":state.sequence,
                "status":if state.release {"succeeded"} else {"running"},"error":null,"progress":{"percent":37}});
            if state.release {
                value["output"] = Value::Null;
            }
            value
        }
        "/v1/art/local/service/cancel" => {
            let mut state = state.lock().unwrap();
            if state.image_output_owner.as_ref() == Some(&body["owner"]) {
                state.image_output_owner = None;
                return json!({"protocolVersion":"loom.art.local.service.v1","owner":body["owner"],"sequence":1,
                    "status":"succeeded","output":{"answer":42},"error":null,"progress":null});
            }
            assert_eq!(Some(&body["owner"]), state.failed_owner.as_ref());
            assert!(body.get("input").is_none());
            json!({"protocolVersion":"loom.art.local.service.v1","owner":body["owner"],"sequence":1,
                "status":"cancelled","error":null,"progress":null})
        }
        other => panic!("unexpected product endpoint: {other}"),
    }
}
