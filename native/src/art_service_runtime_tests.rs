//! 真实后台 Runtime 在 submit 已到达而响应未返回时关闭；取消与认证撤权必须分流。
use super::*;
use crate::surface_javascript::{fixture as js, signed::runtime::Runtime};
use std::sync::mpsc;
use std::time::Instant;

const SOURCE: &str = r#"
NeuroSurface.define({async mount({root}) {
 const value=await NeuroSurface.service('capability:vendor:service.run',{},5000);
 const label=document.createElement('span');label.textContent=String(value);root.appendChild(label);
}});
"#;

#[test]
fn art_service_runtime_close_interrupts_submit_and_cancels_without_replaying() {
    run(false);
}
#[test]
fn art_service_runtime_authority_revocation_never_sends_cleanup_with_revoked_credentials() {
    run(true);
}
#[test]
fn art_service_runtime_progress_precedes_frame_and_revocation_discards_it() {
    let _serial = js::SERIAL.lock().unwrap();
    for revoke in [false, true] {
        let (release, waiting) = mpsc::channel();
        let (arrived, observed) = mpsc::channel();
        let (prepared, client, _device, lease, peer) =
            setup_source(3, SOURCE, move |index, _, _, response| {
                if index == 1 {
                    response["status"] = json!("running");
                    response["progress"] = json!({"percent":50,"message":"must not be surfaced"});
                } else if index == 2 {
                    arrived.send(()).unwrap();
                    waiting.recv_timeout(Duration::from_secs(5)).unwrap();
                    response["status"] = json!("succeeded");
                    response["output"] = Value::Null;
                }
            });
        let mut runtime =
            Runtime::start(prepared, view(client.origin()), js::snapshot(1, 0), true).unwrap();
        let binding = runtime.binding().clone();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            assert!(runtime.take_reply(&binding).is_none());
            if let Some(update) = runtime.take_progress(&binding) {
                if update.percent == Some(50) {
                    assert_eq!(update.status, Status::Running);
                    break;
                }
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(runtime.pending());
        observed.recv_timeout(Duration::from_secs(5)).unwrap();
        release.send(()).unwrap();
        // 先确认服务端已写完终态，避免把客户端撤权中断连接误判为服务夹具失败。
        // UI仍未领取任何正式帧；撤权后既有/迟到的进度与帧均必须拒绝。
        peer.join().unwrap();
        if revoke {
            lease.revoke();
        }
        loop {
            if revoke {
                assert!(runtime.take_progress(&binding).is_none());
                if let Some(frame) = runtime.take_reply(&binding) {
                    assert!(frame.is_err());
                }
                if runtime.take_exit().is_some() {
                    break;
                }
            } else if let Some(frame) = runtime.take_reply(&binding) {
                assert_eq!(js::text(&frame.unwrap().scene), "null");
                assert!(runtime.take_progress(&binding).is_none());
                runtime.request_stop();
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        drop(runtime);
    }
}
fn run(revoke: bool) {
    let _serial = js::SERIAL.lock().unwrap();
    let (listener, client, config) = fixture();
    let origin = client.origin.clone();
    let (device, lease) = lease(&origin);
    let (submitted, received) = mpsc::channel();
    let (release, waiting) = mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut stream = StreamOwned::new(
            ServerConnection::new(config.clone()).unwrap(),
            accept(&listener),
        );
        let (head, _) = request(&mut stream);
        assert!(head.starts_with("POST /v1/art/local/prepare "));
        reply(
            &mut stream,
            200,
            &serde_json::to_vec(&service_response_source(&origin, SOURCE)).unwrap(),
        );
        drop(stream);
        let mut stream = StreamOwned::new(
            ServerConnection::new(config.clone()).unwrap(),
            accept(&listener),
        );
        let (head, body) = request(&mut stream);
        assert!(head.starts_with("POST /v1/art/local/service/submit "));
        assert_eq!(body["input"], json!({}));
        submitted.send(()).unwrap();
        waiting.recv_timeout(Duration::from_secs(5)).unwrap();
        drop(stream); // 故意不回复 submit：取消不得依赖成功取得 Accepted。
        if !revoke {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            let (head, cancel) = request(&mut stream);
            assert!(head.starts_with("POST /v1/art/local/service/cancel "));
            assert!(head
                .to_ascii_lowercase()
                .contains("authorization: device fixture-secret\r\n"));
            assert_eq!(cancel["owner"], body["owner"]);
            assert_eq!(cancel["grant"], body["grant"]);
            assert!(cancel.get("input").is_none());
            assert!(cancel.get("timeoutMs").is_none());
            reply(
                &mut stream,
                200,
                &serde_json::to_vec(&json!({
                    "protocolVersion":"loom.art.local.service.v1", "owner":cancel["owner"],
                    "sequence":1,"status":"cancelled","error":null,"progress":null
                }))
                .unwrap(),
            );
        }
        listener
    });
    let mut input = input(client.origin());
    input.permissions.push(wire::Permission::LoomService);
    input.services.push(SERVICE.into());
    let prepared = Prepared::fetch(&client, lease.clone(), input, pin()).unwrap();
    let mut runtime =
        Runtime::start(prepared, view(client.origin()), js::snapshot(1, 0), true).unwrap();
    let binding = runtime.binding().clone();
    received.recv_timeout(Duration::from_secs(5)).unwrap();
    let began = std::time::Instant::now();
    if revoke {
        drop(device);
        assert!(lease.is_revoked());
    } else {
        runtime.request_stop();
        assert!(lease.execution_stopped());
        assert!(!lease.is_revoked());
    }
    release.send(()).unwrap();
    let deadline = began + Duration::from_secs(3);
    loop {
        assert!(runtime.take_reply(&binding).is_none());
        if let Some(result) = runtime.take_exit() {
            assert!(result.is_err());
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(lease.is_revoked());
    assert!(runtime.finished());
    let listener = peer.join().unwrap();
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}
