//! 隐藏 HWND + 自建 TLS 服务覆盖应用启动/消费链，不注册全局输入或读取用户文件。
use super::*;
use crate::{
    desktop::{
        document_window::DocumentWindow, loom_execution_job::Execution,
        loom_input_job::PreparedBatch, loom_result_state::Status, Apartment, State,
    },
    loom_bridge_client::tests::{execution_job::SERIAL, fixture},
    loom_bridge_stream::BridgeStream,
    workspace_id::UnitId,
};
use std::{
    thread,
    time::{Duration, Instant},
};
use windows::{
    core::w,
    Win32::{
        Foundation::HINSTANCE, System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*,
    },
};

struct Class(HINSTANCE);
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Native.Workspace"), Some(self.0));
        }
    }
}
#[test]
fn hidden_application_starts_commits_and_rejects_changed_preparation() {
    let _serial = SERIAL.lock().unwrap();
    let _input_serial = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: w!("Otter.Native.Workspace"),
                lpfnWndProc: Some(crate::desktop::procedure::window_proc),
                ..Default::default()
            }),
            0
        );
        let _class = Class(instance);
        let pixels: crate::pixels::Pixels = vec![10, 20, 30, 255, 40, 50, 60, 255].into();
        let image = crate::desktop::image::Image {
            width: 2,
            height: 1,
            pixels: pixels.clone(),
        };
        let document =
            DocumentWindow::create(instance, State::new(Some(image), false).unwrap(), false)
                .unwrap();
        let mut app = Application::new(instance, document);
        let unit = app.documents[0].state.unit_id.clone();
        let stamp = app.documents[0]
            .state
            .canvas
            .borrow()
            .editor
            .document_stamp();
        use crate::graph_image::{DispatchOptions, Graph, Kind, Node, Preparation};
        let ports = [crate::graph_image::Port {
            name: "image",
            kind: "image",
        }];
        let nodes = [Node {
            id: &unit,
            kind: Kind::Art,
            inputs: &[],
            outputs: &[],
            capability_inputs: Some(&ports),
            capability_outputs: &[],
            values: &[],
            source: None,
            preview: None,
            manual_image: None,
            input_disabled: false,
            accept_upstream: true,
            locally_edited: false,
            pending: false,
        }];
        let graph = Graph::new(&nodes, &[]).unwrap();
        let manual = [("image", crate::graph_image::Output::Image("fixture-image"))];
        let Preparation::Ready(prepared) = graph
            .prepare_dispatch(
                &unit,
                &[],
                &manual,
                DispatchOptions {
                    config: Default::default(),
                    runtime: Default::default(),
                    trigger: crate::graph_execution_config::Trigger::Manual,
                },
            )
            .unwrap()
        else {
            panic!("prepared graph");
        };
        let ticket = UnitId::parse("prepare-fixture").unwrap();
        {
            let canvas = app.documents[0].state.canvas.borrow();
            let source = crate::desktop::loom_input_job::Source::capture(
                "image".into(),
                unit.clone(),
                2,
                1,
                pixels.clone(),
                &canvas.editor,
            )
            .unwrap();
            app.input_jobs
                .start(ticket.clone(), unit.clone(), &canvas.editor, vec![source])
                .unwrap();
        }
        let deadline = Instant::now() + Duration::from_secs(10);
        let inputs = loop {
            let mut message = MSG::default();
            if PeekMessageW(&mut message, None, WM_APP + 92, WM_APP + 92, PM_REMOVE).as_bool() {
                app.execution_wake.acknowledge();
                // 此段单独验证手动 stage；尚未安装 Pending 配置，不进入生产配对流程。
                app.input_jobs.poll();
                if let Some(report) = app.input_jobs.take_completed() {
                    assert_eq!(report.ticket, ticket);
                    assert_eq!(report.target, unit);
                    break report.result.unwrap();
                }
            }
            assert!(Instant::now() < deadline, "image preparation timeout");
            thread::sleep(Duration::from_millis(1));
        };
        let (listener, identity, config) = fixture();
        let server_unit = unit.clone();
        let server = thread::spawn(move || {
            let stream = BridgeStream::new(
                listener.accept().unwrap().0,
                Instant::now() + Duration::from_secs(5),
            )
            .unwrap();
            let mut socket = tungstenite::accept(rustls::StreamOwned::new(
                rustls::ServerConnection::new(config).unwrap(),
                stream,
            ))
            .unwrap();
            let wire: serde_json::Value =
                serde_json::from_str(&socket.read().unwrap().into_text().unwrap()).unwrap();
            assert_eq!(wire["method"], "loom.hook.art.execute");
            assert_eq!(wire["params"]["nodeId"], server_unit.as_str());
            assert_eq!(wire["params"]["requestId"], "fixture-request");
            assert_eq!(wire["params"]["generation"], 1);
            assert_eq!(wire["params"]["artId"], "fixture-art");
            assert_eq!(wire["params"]["inputs"]["image"]["kind"], "inline_resource");
            assert_eq!(wire["params"]["inputs"]["image"]["mime"], "image/png");
            let decoded = crate::desktop::loom_output_inline::decode(
                &wire["params"]["inputs"]["image"],
                &std::sync::atomic::AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!((decoded.width, decoded.height), (2, 1));
            assert_eq!(decoded.pixels, pixels);
            socket.send(tungstenite::Message::Text(serde_json::json!({"protocolVersion":"loom.hook.v1",
                "requestId":"fixture-request","status":"succeeded","data":{
                "protocolVersion":"loom.hook.v1","requestId":"fixture-request","nodeId":server_unit.as_str(),
                "generation":1,"resultRevision":1,"outputs":{"output_image":wire["params"]["inputs"]["image"]}}}).to_string())).unwrap();
        });
        let canvas = app.documents[0].state.canvas.borrow();
        for reason in ["timeout", "stale", "art"] {
            let (_unused_listener, invalid_identity, _) = fixture();
            let other = crate::editor::Editor::default();
            let rejected = crate::desktop::loom_execution_prepare::stage(
                &mut app.executions,
                crate::desktop::loom_execution_prepare::Plan {
                    identity: invalid_identity,
                    request_id: UnitId::parse("rejected-request").unwrap(),
                    art_id: if reason == "art" { "" } else { "fixture-art" },
                    prepared: &prepared,
                    inputs: PreparedBatch::empty_for_test(
                        unit.clone(),
                        if reason == "stale" {
                            &other
                        } else {
                            &canvas.editor
                        },
                    ),
                    timeout: if reason == "timeout" {
                        Duration::ZERO
                    } else {
                        Duration::from_secs(5)
                    },
                    transport: crate::loom_art_wire::OutputTransport::Websocket,
                },
                |_| Some(&canvas.editor),
            );
            assert!(rejected.is_err());
            assert!(app.executions.active(&unit).is_none());
        }
        let execution = crate::desktop::loom_execution_prepare::stage(
            &mut app.executions,
            crate::desktop::loom_execution_prepare::Plan {
                identity,
                request_id: UnitId::parse("fixture-request").unwrap(),
                art_id: "fixture-art",
                prepared: &prepared,
                inputs,
                timeout: Duration::from_secs(5),
                transport: crate::loom_art_wire::OutputTransport::Websocket,
            },
            |_| Some(&canvas.editor),
        )
        .unwrap();
        drop(canvas);
        *app.documents[0].state.pending_execution.borrow_mut() = Some(execution);
        app.pending_windows(false).unwrap();
        assert!(app.documents[0].state.pending_execution.borrow().is_none());
        assert_eq!(
            app.documents[0].state.execution_results.borrow().status(),
            Status::Running
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        while app.documents[0].state.execution_results.borrow().status() == Status::Running {
            let mut message = MSG::default();
            // 只有 worker 的应用消息到达才运行消费，不用反复轮询任务冒充唤醒集成。
            if PeekMessageW(&mut message, None, WM_APP + 92, WM_APP + 92, PM_REMOVE).as_bool() {
                app.pending_windows(false).unwrap();
            }
            assert!(Instant::now() < deadline, "application execution timeout");
            thread::sleep(Duration::from_millis(1));
        }
        {
            let results = app.documents[0].state.execution_results.borrow();
            assert_eq!(results.status(), Status::Completed);
            assert_eq!(results.ports().len(), 2);
            for port in results.ports() {
                let crate::desktop::loom_output_batch::Output::Image(image) = &port.value else {
                    panic!("image");
                };
                assert_eq!((image.width, image.height), (2, 1));
                assert_eq!(&*image.pixels, &[10, 20, 30, 255, 40, 50, 60, 255]);
            }
            assert_eq!(
                &*results.display().unwrap().pixels,
                &[10, 20, 30, 255, 40, 50, 60, 255]
            );
        }
        assert!(app.executions.active(&unit).is_none());
        assert!(app.execution_jobs.is_empty());
        assert!(stamp.matches(&app.documents[0].state.canvas.borrow().editor));
        server.join().unwrap();

        // 生产配对入口只准备正式请求；测试在启动前领取，验证此阶段不联网。
        let (pair_listener, pair_identity, _) = fixture();
        pair_listener.set_nonblocking(true).unwrap();
        let pair_ticket = UnitId::parse("paired-preparation").unwrap();
        *app.documents[0].state.pending_preparation.borrow_mut() =
            Some(crate::desktop::loom_execution_prepare::Pending {
                shader: None,
                dependencies: Vec::new(),
                ticket: pair_ticket.clone(),
                topology: app.workspace.topology_stamp(),
                selection: crate::desktop::configuration_stamp::Stamp::capture(
                    &app.documents[0].state,
                ),
                plan: crate::desktop::loom_execution_prepare::Configured {
                    identity: pair_identity,
                    request_id: UnitId::parse("paired-request").unwrap(),
                    configuration: crate::loom_art_wire::Configuration::capture(
                        "paired-art",
                        &prepared,
                    )
                    .unwrap(),
                    timeout: Duration::from_secs(5),
                    transport: crate::loom_art_wire::OutputTransport::Websocket,
                },
            });
        {
            let canvas = app.documents[0].state.canvas.borrow();
            let image = canvas.image.as_ref().unwrap();
            let source = crate::desktop::loom_input_job::Source::capture(
                "image".into(),
                unit.clone(),
                image.width,
                image.height,
                image.pixels.clone(),
                &canvas.editor,
            )
            .unwrap();
            app.input_jobs
                .start(pair_ticket, unit.clone(), &canvas.editor, vec![source])
                .unwrap();
        }
        let deadline = Instant::now() + Duration::from_secs(10);
        while app.documents[0].state.pending_execution.borrow().is_none() {
            app.input_jobs.poll();
            app.collect_prepared_inputs();
            assert!(Instant::now() < deadline, "configuration pairing timeout");
            thread::sleep(Duration::from_millis(1));
        }
        assert!(app.documents[0]
            .state
            .pending_preparation
            .borrow()
            .is_none());
        assert!(app.documents[0].state.preparation_error.borrow().is_none());
        let paired = app.documents[0]
            .state
            .pending_execution
            .borrow_mut()
            .take()
            .unwrap();
        let wire: serde_json::Value = serde_json::from_str(&paired.message).unwrap();
        assert_eq!(wire["params"]["artId"], "paired-art");
        assert_eq!(wire["params"]["generation"], 2);
        assert!(app.executions.finish(&unit, &paired.request));
        assert_eq!(
            pair_listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );

        // 不同 Editor 身份模拟准备后替换；listener 非阻塞证明应用未发起连接。
        let (listener, identity, _) = fixture();
        listener.set_nonblocking(true).unwrap();
        let request = app
            .executions
            .begin(unit.clone(), UnitId::parse("stale-request").unwrap())
            .unwrap();
        *app.documents[0].state.pending_execution.borrow_mut() = Some(Execution {
            shader: None,
            identity,
            unit: unit.clone(),
            request,
            message: "must-not-send".into(),
            inputs: PreparedBatch::empty_for_test(unit.clone(), &crate::editor::Editor::default()),
            timeout: Duration::from_secs(5),
        });
        app.pending_windows(false).unwrap();
        assert_eq!(
            app.documents[0].state.execution_results.borrow().status(),
            Status::Failed
        );
        assert!(app.executions.active(&unit).is_none());
        assert!(app.execution_jobs.is_empty());
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        let (replacement_listener, replacement_identity, _) = fixture();
        replacement_listener.set_nonblocking(true).unwrap();
        let current_ticket = UnitId::parse("current-config").unwrap();
        *app.documents[0].state.pending_preparation.borrow_mut() =
            Some(crate::desktop::loom_execution_prepare::Pending {
                shader: None,
                dependencies: Vec::new(),
                ticket: current_ticket.clone(),
                topology: app.workspace.topology_stamp(),
                selection: crate::desktop::configuration_stamp::Stamp::capture(
                    &app.documents[0].state,
                ),
                plan: crate::desktop::loom_execution_prepare::Configured {
                    identity: replacement_identity,
                    request_id: UnitId::parse("future-request").unwrap(),
                    configuration: crate::loom_art_wire::Configuration::capture(
                        "current-art",
                        &prepared,
                    )
                    .unwrap(),
                    timeout: Duration::from_secs(5),
                    transport: crate::loom_art_wire::OutputTransport::Websocket,
                },
            });
        app.input_jobs
            .start(
                UnitId::parse("retired-config").unwrap(),
                unit.clone(),
                &app.documents[0].state.canvas.borrow().editor,
                Vec::new(),
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !app.input_jobs.is_empty() {
            app.pending_windows(false).unwrap();
            assert!(
                Instant::now() < deadline,
                "retired preparation was not reclaimed"
            );
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            app.documents[0]
                .state
                .pending_preparation
                .borrow()
                .as_ref()
                .unwrap()
                .ticket,
            current_ticket
        );
        assert!(app.documents[0].state.pending_execution.borrow().is_none());
        assert!(app.executions.active(&unit).is_none());
        assert_eq!(
            replacement_listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        let state = &app.documents[0].state;
        crate::desktop::context_menu::execute(
            app.documents[0].window.0,
            state,
            crate::desktop::execution_cancel::COMMAND,
        )
        .unwrap();
        assert!(state.pending_preparation.borrow().is_some());
        app.pending_windows(false).unwrap();
        assert!(app.documents[0]
            .state
            .pending_preparation
            .borrow()
            .is_none());
        // 菜单请求捕获旧令牌；处理前开始新代际时，不能取消新的运行状态。
        let old = app
            .executions
            .begin(unit.clone(), UnitId::parse("cancel-old").unwrap())
            .unwrap();
        app.documents[0]
            .state
            .execution_results
            .borrow_mut()
            .begin(old);
        crate::desktop::context_menu::execute(
            app.documents[0].window.0,
            &app.documents[0].state,
            crate::desktop::execution_cancel::COMMAND,
        )
        .unwrap();
        let new = app
            .executions
            .begin(unit.clone(), UnitId::parse("cancel-new").unwrap())
            .unwrap();
        app.documents[0]
            .state
            .execution_results
            .borrow_mut()
            .begin(new.clone());
        app.cancel_requested_executions();
        assert!(app.executions.is_latest(&unit, &new));
        assert_eq!(
            app.documents[0].state.execution_results.borrow().status(),
            Status::Running
        );
        crate::desktop::context_menu::execute(
            app.documents[0].window.0,
            &app.documents[0].state,
            crate::desktop::execution_cancel::COMMAND,
        )
        .unwrap();
        app.cancel_requested_executions();
        assert!(app.executions.active(&unit).is_none());
        assert_eq!(
            app.documents[0].state.execution_results.borrow().status(),
            Status::Cancelled
        );
    }
}
