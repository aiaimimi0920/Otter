//! 菜单/参数执行回归专用回环 TLS 对端：验证实际消息与像素，不访问用户服务。
use std::{
    net::TcpListener,
    sync::{atomic::AtomicBool, Arc},
    thread::JoinHandle,
    time::{Duration, Instant},
};
pub(super) fn serve(
    listener: TcpListener,
    config: Arc<rustls::ServerConfig>,
    revision: u64,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "execution did not connect");
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(error) => panic!("fixture accept: {error}"),
            }
        };
        let stream = crate::loom_bridge_stream::BridgeStream::new(stream, deadline).unwrap();
        let tls = rustls::StreamOwned::new(rustls::ServerConnection::new(config).unwrap(), stream);
        let mut socket = tungstenite::accept(tls).unwrap();
        let wire: serde_json::Value =
            serde_json::from_str(&socket.read().unwrap().into_text().unwrap()).unwrap();
        assert_eq!(wire["method"], "loom.hook.art.execute");
        assert_eq!(wire["params"]["artId"], "test-art");
        assert!(wire["params"]["parameters"].get("secret").is_none());
        let output = if revision == 1 {
            assert_eq!(wire["params"]["inputs"], serde_json::json!({}));
            7
        } else if revision == 3 {
            let image = crate::desktop::loom_output_inline::decode(
                &wire["params"]["inputs"]["input_image"],
                &AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!((image.width, image.height), (2, 2));
            assert_eq!(&*image.pixels, &[0, 0, 255, 255].repeat(4));
            44
        } else {
            assert_eq!(revision, 2);
            let _apartment = crate::desktop::Apartment::new().unwrap();
            for (name, expected) in [
                ("input_image", [11, 22, 33, 255]),
                ("reference_image", [77, 88, 99, 255]),
            ] {
                let image = crate::desktop::loom_output_inline::decode(
                    &wire["params"]["inputs"][name],
                    &AtomicBool::new(false),
                )
                .unwrap();
                assert_eq!((image.width, image.height), (1, 1));
                assert_eq!(&*image.pixels, &expected);
            }
            assert_eq!(
                wire["params"]["parameters"]["amount"],
                serde_json::json!(10)
            );
            33
        };
        let params = &wire["params"];
        socket.send(tungstenite::Message::Text(serde_json::json!({
            "protocolVersion":"loom.hook.v1", "requestId":params["requestId"], "status":"succeeded",
            "data":{"protocolVersion":"loom.hook.v1","requestId":params["requestId"],
                "nodeId":params["nodeId"],"generation":params["generation"],"resultRevision":revision,
                "outputs":{"output":{"kind":"value","value":output}}}
        }).to_string())).unwrap();
    })
}
