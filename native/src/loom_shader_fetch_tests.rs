//! 真实认证 TLS 取码→同份 PNG 输入/LUT→原任务槽→正式端口；没有用户 Loom 或测试后门入口。
use super::*;
use crate::{
    desktop::{
        loom_execution_prepare::Configured,
        loom_execution_tasks::{CommitError, Tasks},
        loom_result_state::Results,
        shader_prepare::{self, tests as staging},
    },
    graph_requests::Requests,
    loom_art_wire::{Configuration, OutputTransport},
    shader_angle::Backend,
};
use std::sync::atomic::AtomicBool;
#[path = "loom_shader_fetch_cleanup_tests.rs"]
mod cleanup;

fn configuration() -> Configuration {
    let raw = json!({"id":"shader-art","parameters":[
        {"id":"mode","default":"cpu"},{"id":"output_mode","default":"cpu"},
        {"id":"gain","default":1},{"id":"secret","secret":true,"default":"never send"},
        {"id":"disabled","default":"__DISABLED__"}]});
    let parameters = crate::loom_parameter_state::Parameters::new(
        &crate::loom_capability::Capability::new(&raw).unwrap(),
    )
    .unwrap();
    Configuration::shader_from_parameters(&parameters, &["input_path", "reference_path"]).unwrap()
}
// tungstenite Callback 固定返回 ErrorResponse；只对此第三方回调签名豁免，不放宽产品错误类型。
#[allow(clippy::result_large_err)]
pub(crate) fn authenticated_socket(
    listener: &std::net::TcpListener,
    config: Arc<rustls::ServerConfig>,
    identity: &crate::loom_bridge_identity::Identity,
) -> tungstenite::WebSocket<StreamOwned<ServerConnection, BridgeStream>> {
    let mut stream = BridgeStream::new(
        listener.accept().unwrap().0,
        Instant::now() + Duration::from_secs(10),
    )
    .unwrap();
    let mut connection = ServerConnection::new(config).unwrap();
    while connection.is_handshaking() {
        connection.complete_io(&mut stream).unwrap();
    }
    let context = identity.exporter_context();
    let proof = connection
        .export_keying_material(
            [0u8; 32],
            b"EXPORTER-Loom-Local-Bridge-v1",
            Some(context.as_bytes()),
        )
        .unwrap();
    let expected = format!(
        "LoomBridgeProof {}",
        proof
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    tungstenite::accept_hdr(
        StreamOwned::new(connection, stream),
        |request: &tungstenite::handshake::server::Request, response| {
            assert_eq!(
                request.headers().get("authorization").unwrap(),
                expected.as_str()
            );
            assert_eq!(
                request.headers().get("x-loom-bridge-instance").unwrap(),
                identity.instance()
            );
            Ok(response)
        },
    )
    .unwrap()
}
fn wait(tasks: &mut Tasks, registry: &Requests) -> crate::desktop::loom_execution_tasks::Report {
    let deadline = Instant::now() + Duration::from_secs(25);
    loop {
        tasks.poll();
        if let Some(report) = tasks.take_settled(registry) {
            return report;
        }
        assert!(Instant::now() < deadline, "shader fetch did not settle");
        thread::sleep(Duration::from_millis(1));
    }
}
fn execute(
    payload: serde_json::Value,
    runtime: shader_prepare::Runtime,
    rgba: &[u8],
    size: (u32, u32),
    stale: bool,
) -> Option<crate::desktop::loom_image_value::ImageValue> {
    let editor = Editor::default();
    let inputs = staging::prepared(&editor, size.0, size.1, rgba);
    let (listener, identity, config) = fixture();
    let server_identity = identity.clone();
    let expected = rgba.to_vec();
    let server = thread::spawn(move || {
        let mut socket = authenticated_socket(&listener, config, &server_identity);
        let message = socket.read().unwrap();
        let value: serde_json::Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
        assert!(!message.to_text().unwrap().contains("never send"));
        assert_eq!(value["method"], "loom.hook.art.execute");
        let params = &value["params"];
        assert_eq!(params["parameters"]["mode"], "shader");
        assert_eq!(params["parameters"]["output_mode"], "shader");
        assert_eq!(params["disabledParameters"], json!(["disabled"]));
        assert_eq!(params["outputTransports"], json!(["websocket"]));
        let encoded = params["inputs"]["input_path"]["dataBase64"]
            .as_str()
            .unwrap();
        let decoded = staging::decode_png(encoded);
        assert_eq!(&*decoded.rgba, expected);
        assert!(params["inputs"]["reference_path"]["dataBase64"]
            .as_str()
            .is_some());
        socket.send(tungstenite::Message::Text(json!({"protocolVersion":"loom.hook.v1","requestId":params["requestId"],"status":"succeeded","data":{
            "protocolVersion":"loom.hook.v1","requestId":params["requestId"],"nodeId":params["nodeId"],"generation":params["generation"],"resultRevision":1,
            "outputs":{"output":{"kind":"value","value":payload}}}}).to_string())).unwrap();
    });
    let mut registry = Requests::default();
    let mut execution = Configured {
        identity,
        request_id: staging::id("shader-fetch"),
        configuration: configuration(),
        timeout: Duration::from_secs(5),
        transport: OutputTransport::Websocket,
    }
    .stage_checked(&mut registry, inputs, |_, _| true)
    .unwrap();
    execution.shader = Some(runtime);
    let request = execution.request.clone();
    let (sent, received) = std::sync::mpsc::channel();
    let mut tasks = Tasks::with_notify(Arc::new(move || {
        let _ = sent.send(());
    }));
    tasks.start(execution, &registry).unwrap();
    let mut results = Results::new(staging::id("target"));
    results.begin(request.clone());
    received.recv_timeout(Duration::from_secs(25)).unwrap();
    let mut report = wait(&mut tasks, &registry);
    assert_eq!(report.unit, staging::id("target"));
    let completion = match &report.completion {
        crate::desktop::loom_execution_tasks::Completion::Remote(Ok(completion)) => completion,
        _ => panic!("fetch must retain its actual remote identity"),
    };
    assert!(
        completion.remote_cancel.is_none(),
        "local failure must not cancel a successful remote terminal"
    );
    assert!(!completion.cleanup_pending());
    let successful = completion.result.is_ok();
    if stale {
        assert!(
            successful,
            "stale proof requires a genuinely rendered result"
        );
        let changed = Editor::default();
        assert!(matches!(
            report.commit(&mut registry, &mut results, |_| Some(&changed)),
            Err(CommitError::Stale)
        ));
        assert!(report.reject(&mut registry, &mut results, "stale"));
    } else if successful {
        report
            .commit_checked(&mut registry, &mut results, |_, _| true)
            .unwrap();
    } else {
        assert!(matches!(
            report.commit_checked(&mut registry, &mut results, |_, _| true),
            Err(CommitError::ExecutionFailed)
        ));
        assert!(results.image_source("output").is_none());
        assert!(report.reject(&mut registry, &mut results, "invalid shader"));
    }
    assert!(tasks.is_empty());
    assert!(registry.is_idle());
    server.join().unwrap();
    results.image_source("output").map(|source| source.image)
}
#[test]
fn shader_fetch_invalid_payload_and_texture_fail_after_authenticated_terminal_without_cancel() {
    let _serial = SERIAL.lock().unwrap();
    let _input = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    for payload in [json!({"type":"other"}), {
        let mut value = staging::payload();
        value["textures"]["lut"] = json!("https://example.test/not-read");
        value
    }] {
        assert!(execute(
            payload,
            staging::runtime(Backend::D3d11, false),
            &[64, 32, 16, 128],
            (1, 1),
            false
        )
        .is_none());
    }
}
#[test]
#[ignore = "requires verified ANGLE fixture and actual browser cases; authenticated fetch plus real child"]
fn shader_fetch_real_loom_sources_lut_and_png_inputs_commit_through_shared_tasks() {
    let _serial = SERIAL.lock().unwrap();
    let _input = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    for backend in [Backend::D3d11, Backend::Warp] {
        for case in crate::shader_angle::contract_tests::cases() {
            let input = case
                .textures
                .iter()
                .find(|texture| texture.name == "input")
                .unwrap();
            let mut payload = json!({"type":"shader","vertex_shader":case.vertex,"fragment_shader":case.fragment,"uniforms":case.uniforms,"textures":{}});
            for texture in case
                .textures
                .iter()
                .filter(|texture| texture.name != "input")
            {
                payload["textures"][&texture.name] = json!(format!(
                    "data:image/png;base64,{}",
                    staging::encoded(&staging::png(texture.width, texture.height, &texture.rgba))
                ));
            }
            if case.name == "loom-exposure-alpha" {
                payload = json!({"content":[{"text":payload.to_string()}]});
            }
            let image = execute(
                payload,
                staging::runtime(backend, true),
                &input.rgba,
                (case.width, case.height),
                false,
            )
            .unwrap();
            let expected = crate::desktop::shader_image::prepare(
                crate::shader_angle::Frame {
                    width: case.width,
                    height: case.height,
                    rgba: case.expected_raw_rgba.into(),
                },
                &AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!(image.pixels, expected.image.pixels);
            assert_eq!(
                crate::desktop::shader_image::tests::decode_rgba(
                    image.encoded_png().unwrap(),
                    case.width,
                    case.height
                ),
                crate::desktop::shader_image::tests::decode_rgba(
                    &expected.png,
                    case.width,
                    case.height
                )
            );
            eprintln!("native-shader-fetch: backend={backend:?} case={} tls=true same-input=true exact-png=true exact-display=true",case.name);
        }
        assert!(execute(
            staging::payload(),
            staging::runtime(backend, true),
            &[64, 32, 16, 128],
            (1, 1),
            true
        )
        .is_none());
        let matrix = crate::desktop::shader_image::tests::matrix();
        let image = execute(
            staging::payload(),
            staging::runtime(backend, true),
            &matrix.rgba,
            (256, 256),
            false,
        )
        .unwrap();
        let expected =
            crate::desktop::shader_image::prepare(matrix, &AtomicBool::new(false)).unwrap();
        assert_eq!(image.pixels, expected.image.pixels);
        assert_eq!(
            crate::desktop::shader_image::tests::decode_rgba(
                image.encoded_png().unwrap(),
                256,
                256
            ),
            crate::desktop::shader_image::tests::decode_rgba(&expected.png, 256, 256)
        );
        eprintln!("native-shader-fetch-matrix: backend={backend:?} tls=true exact-png=true exact-display=true");
    }
}
