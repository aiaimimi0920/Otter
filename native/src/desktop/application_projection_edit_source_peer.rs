//! 有界源端协作TLS夹具；独立blue像素与固定rect坐标oracle，不复用产品合并/初始化实现。
use super::*;
use base64::Engine;
use sha2::Digest;
use std::sync::{
    atomic::{AtomicU16, Ordering},
    Arc, Mutex,
};
struct Projection {
    envelope: Value,
    snapshot: Value,
    digest: String,
    revision: u64,
    attached: bool,
    target: String,
}
pub(super) struct Peer {
    pub trace: Arc<Mutex<Vec<Value>>>,
    pub probe_status: Arc<AtomicU16>,
    pub lose_initialize: Arc<AtomicBool>,
    pub omit_editing: Arc<AtomicBool>,
    pub lose_mode: Arc<AtomicBool>,
    pub fail_apply: Arc<AtomicBool>,
    pub receipts: Arc<AtomicU16>,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
fn png(snapshot: &Value, basis: bool) -> String {
    let png = base64::engine::general_purpose::STANDARD
        .decode(snapshot["imageBase64"].as_str().unwrap())
        .unwrap();
    let image = crate::desktop::export::decode_png(png.clone());
    assert_eq!((image.width, image.height), (32, 32));
    assert_eq!(snapshot["width"], 32);
    assert_eq!(snapshot["height"], 32);
    if basis {
        assert_eq!(&*image.pixels, [255, 0, 0, 255].repeat(1024));
    } else {
        assert!(image.pixels.chunks_exact(4).any(|p| p[2] > 0));
    }
    format!("{:x}", sha2::Sha256::digest(png))
}
impl Peer {
    pub(super) fn start(
        listener: std::net::TcpListener,
        tls: Arc<rustls::ServerConfig>,
        profile: std::path::PathBuf,
    ) -> Self {
        let trace = Arc::new(Mutex::new(Vec::new()));
        let out = trace.clone();
        let probe_status = Arc::new(AtomicU16::new(404));
        let probe = probe_status.clone();
        let lose_initialize = Arc::new(AtomicBool::new(false));
        let lose = lose_initialize.clone();
        let omit_editing = Arc::new(AtomicBool::new(false));
        let omit = omit_editing.clone();
        let lose_mode = Arc::new(AtomicBool::new(false));
        let mode_lost = lose_mode.clone();
        let fail_apply = Arc::new(AtomicBool::new(false));
        let failed_apply = fail_apply.clone();
        let receipts = Arc::new(AtomicU16::new(0));
        let receipts_next = receipts.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let worker = std::thread::spawn(move || {
            let _com = Apartment::new().unwrap();
            let mut projections: Vec<Projection> = Vec::new();
            let mut document = Value::Null;
            let mut original_initialize = Value::Null;
            let mut last_mode = Value::Null;
            let mut last_apply = Value::Null;
            let mut last = Instant::now();
            loop {
                let socket = match listener.accept() {
                    Ok((tcp, _)) => tcp,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if stopped.load(Ordering::Acquire) {
                            break;
                        }
                        assert!(
                            last.elapsed() < Duration::from_secs(5),
                            "source peer idle deadline"
                        );
                        std::thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                    Err(error) => panic!("source peer accept: {error}"),
                };
                let tcp = crate::loom_bridge_stream::BridgeStream::new(
                    socket,
                    Instant::now() + Duration::from_secs(5),
                )
                .unwrap();
                let mut stream = StreamOwned::new(ServerConnection::new(tls.clone()).unwrap(), tcp);
                let (head, body) = request(&mut stream);
                let path = head.split_whitespace().nth(1).unwrap();
                {
                    let mut trace = out.lock().unwrap();
                    assert!(trace.len() < 64);
                    trace.push(json!({"path":path,"body":body}));
                }
                last = Instant::now();
                if path == "/v1/device-sessions/challenges" {
                    reply(
                        &mut stream,
                        200,
                        br#"{"deviceId":"device-old","challengeId":"id","challenge":"value"}"#,
                    );
                    continue;
                }
                if path == "/v1/device-sessions" {
                    reply(&mut stream,200,&serde_json::to_vec(&json!({"deviceId":"device-old","token":"sender-token","expiresAtMs":now().unwrap()+60_000})).unwrap());
                    continue;
                }
                assert!(head
                    .to_ascii_lowercase()
                    .contains("authorization: device sender-token\r\n"));
                if path == "/v1/projections/create" {
                    let basis = !document.is_null();
                    let digest = png(&body["snapshot"], basis);
                    assert_eq!(body["envelope"]["content"]["digest"], digest);
                    projections.push(Projection {
                        envelope: body["envelope"].clone(),
                        snapshot: body["snapshot"].clone(),
                        digest,
                        revision: 1,
                        attached: false,
                        target: body["targetDeviceId"].as_str().unwrap().into(),
                    });
                }
                let id = if path.ends_with("/create") {
                    body["envelope"]["projectionId"].as_str().unwrap()
                } else {
                    body["projectionId"].as_str().unwrap()
                };
                let Some(p) = projections
                    .iter_mut()
                    .find(|p| p.envelope["projectionId"] == id)
                else {
                    assert!(path.ends_with("/read"));
                    reply(
                        &mut stream,
                        404,
                        br#"{"error":{"code":"projection_not_found"}}"#,
                    );
                    continue;
                };
                if path == "/v1/projections/edit" {
                    let operation = body["operation"].as_str().unwrap();
                    match operation {
                        "read" if !p.attached => {
                            let status = probe.load(Ordering::Acquire);
                            let code = if status == 404 {
                                "projection_edit_not_found"
                            } else {
                                "unsupported_edit_service"
                            };
                            reply(
                                &mut stream,
                                status,
                                &serde_json::to_vec(&json!({"error":{"code":code}})).unwrap(),
                            );
                            continue;
                        }
                        "read" => {}
                        "initialize" => {
                            let disk: Value = serde_json::from_slice(
                                &std::fs::read(profile.join("projection-edit.json")).unwrap(),
                            )
                            .unwrap();
                            assert_eq!(
                                disk["entries"][0]["pending"]["request"], body,
                                "write before HTTP"
                            );
                            assert_eq!(body["expectedDigest"], p.digest);
                            png(&p.snapshot, true);
                            if document.is_null() {
                                let objects = body["objects"].as_object().unwrap();
                                assert_eq!(objects.len(), 1);
                                let value = objects.values().next().unwrap();
                                assert_eq!(value["type"], "rect");
                                for (key, expected) in
                                    [("x", 4.0), ("y", 4.0), ("w", 8.0), ("h", 8.0)]
                                {
                                    assert_eq!(value[key].as_f64(), Some(expected));
                                }
                                document = json!({"schema":"neuro.projection-edit.v1","sessionId":body["sessionId"],"basis":{"digest":p.digest,"width":32,"height":32},
                                    "revision":1,"modeRevision":1,"checkpointRevision":1,"receiptCount":0,"mode":"one_way",
                                    "objects":objects.iter().map(|(id,value)|(id.clone(),json!({"revision":1,"value":value}))).collect::<serde_json::Map<_,_>>()});
                                original_initialize = body.clone();
                            } else {
                                assert_eq!(
                                    body, original_initialize,
                                    "ambiguous initialization reuses exact initial request"
                                );
                            }
                            p.attached = true;
                            if lose.swap(false, Ordering::AcqRel) {
                                drop(stream);
                                continue;
                            }
                        }
                        "attach" => {
                            let disk: Value = serde_json::from_slice(
                                &std::fs::read(profile.join("projection-edit.json")).unwrap(),
                            )
                            .unwrap();
                            assert_eq!(disk["entries"][0]["pending"]["request"], body);
                            assert_eq!(body["sessionId"], document["sessionId"]);
                            assert_eq!(p.digest, document["basis"]["digest"]);
                            png(&p.snapshot, true);
                            p.attached = true;
                        }
                        "apply" => {
                            if failed_apply.swap(false, Ordering::AcqRel) {
                                reply(
                                    &mut stream,
                                    503,
                                    br#"{"error":{"code":"temporarily_unavailable"}}"#,
                                );
                                continue;
                            }
                            if last_apply == body {
                                reply(&mut stream, 200, &serde_json::to_vec(&document).unwrap());
                                continue;
                            }
                            assert_eq!(body["sessionId"], document["sessionId"]);
                            assert_eq!(body["baseRevision"], document["revision"]);
                            assert_eq!(body["modeRevision"], document["modeRevision"]);
                            let revision = document["revision"].as_u64().unwrap() + 1;
                            document["revision"] = json!(revision);
                            document["receiptCount"] =
                                json!(document["receiptCount"].as_u64().unwrap() + 1);
                            for change in body["changes"].as_array().unwrap() {
                                document["objects"][change["objectId"].as_str().unwrap()] =
                                    json!({"revision":revision,"value":change["value"]});
                            }
                            last_apply = body.clone();
                        }
                        "mode" => {
                            let disk: Value = serde_json::from_slice(
                                &std::fs::read(profile.join("projection-edit.json")).unwrap(),
                            )
                            .unwrap();
                            assert_eq!(disk["entries"][0]["pending"]["request"], body);
                            if last_mode != body {
                                assert_eq!(body["baseModeRevision"], document["modeRevision"]);
                                let revision = document["revision"].as_u64().unwrap() + 1;
                                document["revision"] = json!(revision);
                                document["modeRevision"] = json!(revision);
                                document["mode"] = body["mode"].clone();
                                document["receiptCount"] =
                                    json!(document["receiptCount"].as_u64().unwrap() + 1);
                                last_mode = body.clone();
                            }
                            if mode_lost.swap(false, Ordering::AcqRel) {
                                drop(stream);
                                continue;
                            }
                        }
                        "checkpoint" => {
                            assert_eq!(body["expectedRevision"], document["revision"]);
                            let revision = document["revision"].as_u64().unwrap() + 1;
                            document["revision"] = json!(revision);
                            document["modeRevision"] = json!(revision);
                            document["checkpointRevision"] = json!(revision);
                            document["receiptCount"] = json!(0);
                        }
                        _ => panic!("unexpected edit operation: {body}"),
                    }
                    reply(&mut stream, 200, &serde_json::to_vec(&document).unwrap());
                    continue;
                }
                if path == "/v1/projections/update" {
                    assert!(
                        !p.attached,
                        "collaboration cannot flatten vectors with PNG update"
                    );
                    assert_eq!(body["priorRevision"], p.revision);
                    assert_eq!(body["revision"], p.revision + 1);
                    assert_eq!(body["sourceSessionId"], p.envelope["source"]["sessionId"]);
                    p.digest = png(&body["snapshot"], true);
                    assert_eq!(body["digest"], p.digest);
                    p.snapshot = body["snapshot"].clone();
                    p.revision += 1;
                } else {
                    assert!(
                        path.ends_with("/read") || path.ends_with("/create"),
                        "{head}"
                    );
                }
                let mut response = json!({"envelope":p.envelope,"revision":p.revision,"digest":p.digest,"linked":true,"receiverDeviceId":p.target,
                    "receiverUnitId":format!("remote-{}",p.target),"snapshot":null,"delivery":{"targetDeviceId":p.target,"status":"accepted"}});
                if p.attached && !omit.load(Ordering::Acquire) {
                    let receipts = receipts_next.swap(0, Ordering::AcqRel);
                    if receipts != 0 {
                        document["revision"] = json!(receipts as u64 + 1);
                        document["receiptCount"] = json!(receipts);
                    }
                    response["editing"] = document.clone();
                }
                reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
            }
        });
        Self {
            trace,
            probe_status,
            lose_initialize,
            omit_editing,
            lose_mode,
            fail_apply,
            receipts,
            stop,
            worker: Some(worker),
        }
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            if let Err(error) = worker.join() {
                if !std::thread::panicking() {
                    std::panic::resume_unwind(error);
                }
            }
        }
    }
}
