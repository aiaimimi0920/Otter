//! Independent TLS1.3/exporter peer; certificates stay in memory without installation in the certificate store or access to the user's Loom.
use base64::Engine;
use otter_native::{loom_bridge_identity::Identity, loom_bridge_stream::BridgeStream};
use rustls::{pki_types::PrivatePkcs8KeyDer, ServerConfig, ServerConnection, StreamOwned};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    net::TcpListener,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
const INSTANCE: &str = "d2f4f738-09d9-4f46-829e-262381e63387";
pub struct Peer {
    listener: TcpListener,
    config: Arc<ServerConfig>,
    identity: Identity,
    pub manifest: Value,
}
impl Peer {
    pub fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let rcgen::CertifiedKey { cert, key_pair } =
            rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let endpoint = format!("wss://127.0.0.1:{}/", listener.local_addr().unwrap().port());
        let certificate = base64::engine::general_purpose::STANDARD.encode(cert.der());
        let pin = format!("{:x}", Sha256::digest(cert.der()));
        let token = "a1".repeat(32);
        let identity = Identity::validate(
            "loom.local-bridge.v1",
            INSTANCE,
            &endpoint,
            &certificate,
            &pin,
            &token,
        )
        .unwrap();
        let config =
            ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_protocol_versions(&[&rustls::version::TLS13])
                .unwrap()
                .with_no_client_auth()
                .with_single_cert(
                    vec![cert.der().clone()],
                    PrivatePkcs8KeyDer::from(key_pair.serialize_der()).into(),
                )
                .unwrap();
        let manifest = json!({"schemaVersion":1,"appId":"loom","displayName":"Shader regression peer","version":"1","pid":std::process::id(),"startedAt":"fixture",
            "transport":{"type":"http","baseUrl":"http://127.0.0.1:1/"},"capabilities":["brain.plan"],
            "hookBridge":{"protocol":"loom.local-bridge.v1","instanceId":INSTANCE,"endpoint":endpoint,"certificateDerBase64":certificate,"certificateSha256":pin,"authToken":token}});
        Self {
            listener,
            config: Arc::new(config),
            identity,
            manifest,
        }
    }
    // tungstenite Callback requires the large ErrorResponse error type; do not change that third-party interface.
    #[allow(clippy::result_large_err)]
    fn socket(&self) -> tungstenite::WebSocket<StreamOwned<ServerConnection, BridgeStream>> {
        self.listener.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let tcp = loop {
            match self.listener.accept() {
                Ok((tcp, _)) => break tcp,
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(1))
                }
                Err(e) => panic!("bounded peer accept failed: {e}"),
            }
        };
        let mut stream = BridgeStream::new(tcp, deadline).unwrap();
        let mut tls = ServerConnection::new(self.config.clone()).unwrap();
        while tls.is_handshaking() {
            tls.complete_io(&mut stream).unwrap();
        }
        assert_eq!(
            tls.protocol_version(),
            Some(rustls::ProtocolVersion::TLSv1_3)
        );
        let proof = tls
            .export_keying_material(
                [0u8; 32],
                b"EXPORTER-Loom-Local-Bridge-v1",
                Some(self.identity.exporter_context().as_bytes()),
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
            StreamOwned::new(tls, stream),
            |request: &tungstenite::handshake::server::Request, response| {
                assert!(request.headers()["authorization"] == expected);
                assert!(request.headers()["x-loom-bridge-instance"] == INSTANCE);
                Ok(response)
            },
        )
        .unwrap()
    }
    pub fn assert_unused(self) {
        self.listener.set_nonblocking(true).unwrap();
        assert_eq!(
            self.listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
    pub fn serve_native(self) -> TcpListener {
        let mut socket = self.socket();
        let message: Value =
            serde_json::from_str(socket.read().unwrap().to_text().unwrap()).unwrap();
        assert_eq!(message["method"], "loom.hook.handshake");
        socket.send(tungstenite::Message::Text(json!({"protocolVersion":"loom.hook.v1","serverName":"fixture","serverVersion":"1","sessionId":"fixture-session","transport":"websocket","capabilities":{
            "artDefinitions":[{"id":"shader-startup","label":"Shader startup regression","metadata":{"capabilities":{"preview":"shader","shaderInput":"input"}},"parameters":[{"id":"gain","default":0.5,"widget":"number","min":0,"max":1}],"inputs":[{"name":"input","type":"image","required":true}],"outputs":[]}],
            "operations":["loom.hook.art.execute"],"surface":{"apiVersion":"1","runtimes":[],"nodes":[],"transports":[],"capabilities":[],"input":{"pointer":false,"hover":false,"touch":false,"keyboard":false}}}}).to_string())).unwrap();
        drop(socket);
        let mut socket = self.socket();
        let message: Value =
            serde_json::from_str(socket.read().unwrap().to_text().unwrap()).unwrap();
        assert_eq!(message["method"], "loom.hook.art.execute");
        let p = &message["params"];
        assert_eq!(p["parameters"]["mode"], "shader");
        assert_eq!(p["parameters"]["output_mode"], "shader");
        assert_eq!(p["parameters"]["gain"], 0.5);
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(p["inputs"]["input"]["dataBase64"].as_str().unwrap())
            .unwrap();
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .unwrap();
        let mut pixels = vec![0; decoder.output_buffer_size().unwrap()];
        let frame = decoder.next_frame(&mut pixels).unwrap();
        assert_eq!(&pixels[..frame.buffer_size()], &[64, 32, 16, 255]);
        assert_eq!(p["parameters"]["shader_abi"], "otter.hlsl.v1");
        let payload = json!({"type":"shader","abi":"otter.hlsl.v1","source":"float4 otter_main(float2 uv){float4 p=u_input.Sample(otter_sampler,uv);return float4(p.rgb*u_gain,p.a);}","uniforms":{"gain":0}});
        socket.send(tungstenite::Message::Text(json!({"protocolVersion":"loom.hook.v1","requestId":p["requestId"],"status":"succeeded","data":{
            "protocolVersion":"loom.hook.v1","requestId":p["requestId"],"nodeId":p["nodeId"],"generation":p["generation"],"resultRevision":1,"outputs":{"output":{"kind":"value","value":payload}}}}).to_string())).unwrap();
        drop(socket);
        self.listener
    }
}
