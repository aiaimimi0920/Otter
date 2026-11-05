//! 新取码模式仍使用原网络取消/清理 owner；拒绝未确认租约，不尝试本地渲染。
use super::*;
use crate::desktop::{loom_execution_session::Failure, loom_execution_tasks::Completion};

fn start(
    identity: crate::loom_bridge_identity::Identity,
    registry: &mut Requests,
    editor: &Editor,
) -> Tasks {
    let inputs = staging::prepared(editor, 1, 1, &[64, 32, 16, 128]);
    let mut execution = Configured {
        identity,
        request_id: staging::id("shader-fetch"),
        configuration: configuration(),
        timeout: Duration::from_secs(5),
        transport: OutputTransport::Websocket,
    }
    .stage_checked(registry, inputs, |_, _| true)
    .unwrap();
    execution.shader = Some(staging::runtime(Backend::D3d11, false));
    let mut tasks = Tasks::default();
    tasks.start(execution, registry).unwrap();
    tasks
}
#[test]
fn shader_fetch_cancel_while_waiting_reuses_authenticated_cancel_and_reaps_worker() {
    let _serial = SERIAL.lock().unwrap();
    let _input = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    let (listener, identity, config) = fixture();
    let server_identity = identity.clone();
    let (sent, received) = std::sync::mpsc::channel();
    let server = thread::spawn(move || {
        let mut socket = authenticated_socket(&listener, config.clone(), &server_identity);
        socket.read().unwrap();
        sent.send(()).unwrap();
        assert!(socket.read().is_err());
        let mut control = authenticated_socket(&listener, config, &server_identity);
        let value: serde_json::Value =
            serde_json::from_str(control.read().unwrap().to_text().unwrap()).unwrap();
        assert_eq!(value["method"], "loom.hook.art.cancel");
        assert_eq!(value["params"]["requestId"], "shader-fetch");
        control.send(tungstenite::Message::Text(json!({"protocolVersion":"loom.hook.v1","requestId":"shader-fetch","status":"succeeded"}).to_string())).unwrap();
    });
    let mut registry = Requests::default();
    let editor = Editor::default();
    let mut tasks = start(identity, &mut registry, &editor);
    received.recv_timeout(Duration::from_secs(5)).unwrap();
    tasks.retain_units(|_| false);
    let report = wait(&mut tasks, &registry);
    assert!(!report.current);
    let Completion::Remote(Ok(completion)) = report.completion else {
        panic!("remote completion expected")
    };
    assert!(matches!(completion.result, Err(Failure::Cancelled)));
    assert_eq!(
        completion.remote_cancel,
        Some(Ok(crate::loom_art_control::Outcome::Succeeded))
    );
    assert!(!completion.cleanup_pending());
    assert!(tasks.is_empty());
    server.join().unwrap();
}
#[test]
fn shader_fetch_unreleased_remote_output_stays_owned_and_cannot_reach_local_renderer() {
    let _serial = SERIAL.lock().unwrap();
    let _input = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    let (listener, identity, config) = fixture();
    let server_identity = identity.clone();
    let server = thread::spawn(move || {
        let mut socket = authenticated_socket(&listener, config.clone(), &server_identity);
        let wire: serde_json::Value =
            serde_json::from_str(socket.read().unwrap().to_text().unwrap()).unwrap();
        let params = &wire["params"];
        socket.send(tungstenite::Message::Text(json!({"protocolVersion":"loom.hook.v1","requestId":params["requestId"],"status":"succeeded","data":{
            "protocolVersion":"loom.hook.v1","requestId":params["requestId"],"nodeId":params["nodeId"],"generation":params["generation"],"resultRevision":1,
            "outputs":{"output":{"kind":"value","value":staging::payload()},"z_resource":{"kind":"shared_memory","handle":"Loom_Buffer_missing_shader_fetch_fixture","format":"rgba8","width":1,"height":1,"size":4}}}}).to_string())).unwrap();
        drop(socket);
        let (mut releases, mut cancels) = (0, 0);
        for _ in 0..4 {
            let mut control = authenticated_socket(&listener, config.clone(), &server_identity);
            let value: serde_json::Value =
                serde_json::from_str(control.read().unwrap().to_text().unwrap()).unwrap();
            let status = match value["method"].as_str().unwrap() {
                "loom.hook.art.resources.release" => {
                    releases += 1;
                    "failed"
                }
                "loom.hook.art.cancel" => {
                    cancels += 1;
                    "succeeded"
                }
                other => panic!("unexpected control: {other}"),
            };
            control.send(tungstenite::Message::Text(json!({"protocolVersion":"loom.hook.v1","requestId":value["params"]["requestId"],"status":status}).to_string())).unwrap();
        }
        assert_eq!((releases, cancels), (3, 1));
    });
    let mut registry = Requests::default();
    let editor = Editor::default();
    let mut tasks = start(identity, &mut registry, &editor);
    server.join().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        tasks.poll();
        assert!(tasks.take_settled(&registry).is_none());
        // 让 worker 的补偿和完成栅栏结束；未确认资源仍占同一个槽。
        if let Some(report) = tasks.take_completed(&registry) {
            let Completion::Remote(Ok(mut completion)) = report.completion else {
                panic!("remote completion expected")
            };
            assert!(completion.cleanup_pending());
            assert_eq!(completion.cleanup_retries, 2);
            let Err(Failure::PendingRelease(delivery)) = &mut completion.result else {
                panic!("lease owner lost")
            };
            assert_eq!(delivery.pending_release_count(), 1);
            // 本测试服务的虚构租约显式终结；不把未确认的真实远端资源丢进本地成功分支。
            delivery
                .release_with(staging::id("fixture-final-release"), |_, _| {
                    Ok(crate::loom_art_control::Outcome::Succeeded)
                })
                .unwrap();
            assert!(!completion.cleanup_pending());
            return;
        }
        thread::sleep(Duration::from_millis(1));
    }
    panic!("pending cleanup report did not arrive");
}
