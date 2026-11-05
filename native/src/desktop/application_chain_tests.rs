//! 两个真实隐藏宿主与生产调度器串联；唯一手动启动发生在第一节点。
use super::*;
use crate::desktop::{capability_job::Loaded, capability_selection::Selection, Apartment};
use std::{
    rc::Rc,
    time::{Duration, Instant},
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
struct Class(HINSTANCE);
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Native.Workspace"), Some(self.0));
        }
    }
}
fn node(catalog: &Rc<Loaded>) -> State {
    let state = State::new(None, false).unwrap();
    let parameters =
        crate::loom_parameter_state::Parameters::new(&catalog.handshake.capability(0).unwrap())
            .unwrap();
    *state.selected_art.borrow_mut() = Some(Selection {
        execution: Default::default(),
        identity: crate::desktop::document_state::new_unit_id().unwrap(),
        catalog: catalog.clone(),
        parameters,
    });
    state
}
#[test]
fn committed_formal_value_automatically_executes_direct_child_over_tls() {
    chain(false, None);
}
#[test]
fn committed_image_automatically_executes_child_through_sticker_over_tls() {
    chain(true, None);
}
#[test]
fn disabling_source_notifications_prevents_automatic_child_request() {
    chain(false, Some((0, 3)));
}
#[test]
fn disabling_child_listening_prevents_automatic_child_request() {
    chain(false, Some((1, 2)));
}
#[test]
fn disabling_child_upstream_trigger_prevents_automatic_child_request() {
    chain(false, Some((1, 0)));
}
fn chain(image_chain: bool, blocked: Option<(usize, usize)>) {
    chain_with_shader(image_chain, blocked, false);
}
#[test]
#[ignore = "requires explicit verified ANGLE fixture; actual local Shader child with hidden Application"]
fn local_shader_actual_application_commits_through_the_native_dispatcher() {
    chain_with_shader(true, None, true);
}
fn chain_with_shader(image_chain: bool, blocked: Option<(usize, usize)>, real_shader: bool) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap();
    let _input = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
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
        let (listener, identity, tls_config) = crate::loom_bridge_client::tests::fixture();
        listener.set_nonblocking(true).unwrap();
        let mut raw = crate::loom_handshake::tests::fixture();
        raw["capabilities"]["artDefinitions"][0]["parameters"] = serde_json::json!([
            {"id":"amount","widget":"number","default":1}, {"id":"secret","secret":true},
            {"id":"upload","widget":"file"}
        ]);
        let catalog = Rc::new(Loaded {
            identity,
            handshake: crate::loom_handshake::response(&serde_json::to_vec(&raw).unwrap())
                .unwrap()
                .unwrap(),
        });
        let first = node(&catalog);
        let first_id = first.unit_id.clone();
        let first = DocumentWindow::create(instance, first, false).unwrap();
        let mut app = Application::new(instance, first);
        let second_catalog = if image_chain {
            raw["capabilities"]["artDefinitions"][0]["inputs"] =
                serde_json::json!([{"name":"input_image","type":"image"}]);
            Rc::new(Loaded {
                identity: catalog.identity.clone(),
                handshake: crate::loom_handshake::response(&serde_json::to_vec(&raw).unwrap())
                    .unwrap()
                    .unwrap(),
            })
        } else {
            catalog.clone()
        };
        let second = node(&second_catalog);
        let second_id = second.unit_id.clone();
        app.add_state(second, false).unwrap();
        if let Some((index, flag)) = blocked {
            crate::desktop::context_menu::execute(
                app.documents[index].window.0,
                &app.documents[index].state,
                crate::desktop::execution_settings::FIRST + flag,
            )
            .unwrap();
        }
        let source = if image_chain {
            let relay = State::new(None, false).unwrap();
            let id = relay.unit_id.clone();
            app.add_state(relay, false).unwrap();
            app.workspace
                .connect(crate::workspace::Link {
                    id: crate::desktop::document_state::new_unit_id().unwrap(),
                    from_unit: first_id.clone(),
                    from_port: "output".into(),
                    to_unit: id.clone(),
                    to_port: "image".into(),
                })
                .unwrap();
            id
        } else {
            first_id.clone()
        };
        app.workspace
            .connect(crate::workspace::Link {
                id: crate::desktop::document_state::new_unit_id().unwrap(),
                from_unit: source,
                from_port: "output".into(),
                to_unit: second_id.clone(),
                to_port: if image_chain { "input_image" } else { "amount" }.into(),
            })
            .unwrap();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            for (id, amount, output) in [
                (first_id, 1, 7),
                (second_id, if image_chain { 1 } else { 7 }, 14),
            ]
            .into_iter()
            .take(if blocked.is_some() { 1 } else { 2 })
            {
                let stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(Instant::now() < deadline, "chain execution did not connect");
                            std::thread::sleep(Duration::from_millis(1));
                        }
                        Err(error) => panic!("chain fixture accept: {error}"),
                    }
                };
                let stream =
                    crate::loom_bridge_stream::BridgeStream::new(stream, deadline).unwrap();
                let tls = rustls::StreamOwned::new(
                    rustls::ServerConnection::new(tls_config.clone()).unwrap(),
                    stream,
                );
                let mut socket = tungstenite::accept(tls).unwrap();
                let wire: serde_json::Value =
                    serde_json::from_str(&socket.read().unwrap().into_text().unwrap()).unwrap();
                assert_eq!(wire["method"], "loom.hook.art.execute");
                let params = &wire["params"];
                assert_eq!(params["nodeId"], id.as_str());
                assert_eq!(params["parameters"]["amount"], amount);
                assert!(params["parameters"].get("secret").is_none());
                if image_chain && output == 14 {
                    let image = crate::desktop::loom_output_inline::decode(
                        &params["inputs"]["input_image"],
                        &std::sync::atomic::AtomicBool::new(false),
                    )
                    .unwrap();
                    assert_eq!(&*image.pixels, &[31, 41, 51, 255]);
                } else {
                    assert_eq!(params["inputs"], serde_json::json!({}));
                }
                let result = if image_chain && output == 7 {
                    let image = crate::desktop::loom_inline::prepare(
                        1,
                        1,
                        vec![31, 41, 51, 255].into(),
                        &crate::editor::Editor::default(),
                        &std::sync::atomic::AtomicBool::new(false),
                    )
                    .unwrap();
                    let crate::loom_art_wire::Input::InlinePng { base64, .. } = image.descriptor()
                    else {
                        panic!("PNG")
                    };
                    serde_json::json!({"kind":"inline_resource","mime":"image/png","dataBase64":base64})
                } else {
                    serde_json::json!({"kind":"value","value":output})
                };
                socket.send(tungstenite::Message::Text(serde_json::json!({
                    "protocolVersion":"loom.hook.v1","requestId":params["requestId"],"status":"succeeded",
                    "data":{"protocolVersion":"loom.hook.v1","requestId":params["requestId"],
                        "nodeId":params["nodeId"],"generation":params["generation"],"resultRevision":1,
                        "outputs":{"output":result}}
                }).to_string())).unwrap();
            }
            assert_eq!(
                listener.accept().unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
            listener
        });
        crate::desktop::context_menu::execute(
            app.documents[0].window.0,
            &app.documents[0].state,
            crate::desktop::execution_start::COMMAND,
        )
        .unwrap();
        app.pending_windows(false).unwrap();
        let deadline = Instant::now() + Duration::from_secs(12);
        let last = if blocked.is_some() { 0 } else { 1 };
        while app.documents[last]
            .state
            .execution_results
            .borrow()
            .status()
            != crate::desktop::loom_result_state::Status::Completed
        {
            let mut message = MSG::default();
            if PeekMessageW(&mut message, None, WM_APP + 92, WM_APP + 92, PM_REMOVE).as_bool() {
                app.pending_windows(false).unwrap();
            }
            assert!(Instant::now() < deadline, "automatic chain result timeout");
            std::thread::sleep(Duration::from_millis(1));
        }
        let listener = server.join().unwrap();
        if blocked.is_some() {
            let child = &app.documents[1].state;
            assert_eq!(
                child.execution_results.borrow().status(),
                crate::desktop::loom_result_state::Status::Idle
            );
            assert!(child.pending_start.borrow().is_none());
            assert!(child.pending_preparation.borrow().is_none());
            assert!(child.pending_execution.borrow().is_none());
            assert!(app.executions.active(&child.unit_id).is_none());
        }
        for (document, expected) in app.documents.iter().zip([7, 14]).take(last + 1) {
            let results = document.state.execution_results.borrow();
            assert_eq!(
                results.status(),
                crate::desktop::loom_result_state::Status::Completed
            );
            if image_chain && expected == 7 {
                let image = results.image_source("output").unwrap();
                assert_eq!(&*image.image.pixels, &[31, 41, 51, 255]);
            } else {
                let crate::desktop::loom_output_batch::Output::Value(value) =
                    &results.ports()[0].value
                else {
                    panic!("scalar expected")
                };
                assert_eq!(*value, serde_json::json!(expected));
            }
            assert!(app.executions.active(&document.state.unit_id).is_none());
            assert!(document.state.pending_start.borrow().is_none());
            assert!(document.state.execution_configuration.borrow().is_none());
        }
        assert_eq!(
            app.documents[1]
                .state
                .selected_art
                .borrow()
                .as_ref()
                .unwrap()
                .parameters
                .get("amount"),
            Some(Some(&serde_json::json!(1)))
        );
        assert!(app.input_jobs.is_empty() && app.execution_jobs.is_empty());
        {
            let view = app.documents[1].state.parameter_view.borrow();
            let view = view.as_ref().expect("resolved parameter view");
            assert_eq!(
                view.values
                    .get("amount")
                    .and_then(serde_json::Value::as_f64),
                if image_chain { None } else { Some(7.0) }
            );
            assert_eq!(view.linked.iter().any(|id| id == "amount"), !image_chain);
        }
        if image_chain {
            // 显示与正式结果共享像素，但不得把结果写进原图或编辑历史。
            let results = app.documents[0].state.execution_results.borrow();
            let source = results.image_source("output").unwrap();
            for index in [0, 2] {
                let state = &app.documents[index].state;
                let display = state.resolved_display.borrow();
                let frame = display.as_ref().expect("resolved native display frame");
                assert!(frame.image.pixels.same_storage(&source.image.pixels));
                assert!(frame.editor.is_none());
                assert!(state.canvas.borrow().image.is_none());
            }
            // 缓存命中不得重建快照；在 RefCell 共享借用期间调用也应直接返回。
            let retained = app.documents[2].state.resolved_display.borrow();
            app.sync_display_frames();
            assert!(retained.is_some());
            drop(retained);
            drop(results);
            png_tests::verify(&mut app);
            local_shader_tests::verify(&mut app, real_shader);
            link_menu_tests::verify(&mut app);
            parameter_file_tests::verify(&mut app);
            display_art_tests::verify(&mut app);
            display_composed_tests::verify(&mut app);
            verify_display_lifetime(&mut app);
            display_tests::verify_display_edit(&mut app);
            display_erase_tests::verify(&mut app);
            display_flip_tests::verify(&mut app);
            display_crop_tests::verify(&mut app);
            display_rasterize_tests::verify(&mut app);
            display_resize_tests::verify(&mut app);
            display_blur_resize_tests::verify(&mut app);
            display_blur_crop_tests::verify(&mut app);
        }
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}

#[path = "application_display_tests.rs"]
mod display_tests;
use display_tests::verify_display_lifetime;
#[path = "application_display_art_tests.rs"]
mod display_art_tests;
#[path = "application_display_blur_crop_tests.rs"]
mod display_blur_crop_tests;
#[path = "application_display_blur_resize_tests.rs"]
mod display_blur_resize_tests;
#[path = "application_display_composed_tests.rs"]
mod display_composed_tests;
#[path = "application_display_crop_tests.rs"]
mod display_crop_tests;
#[path = "application_display_erase_tests.rs"]
mod display_erase_tests;
#[path = "application_display_flip_tests.rs"]
mod display_flip_tests;
#[path = "application_display_rasterize_tests.rs"]
mod display_rasterize_tests;
#[path = "application_display_resize_tests.rs"]
mod display_resize_tests;
#[path = "application_link_menu_tests.rs"]
mod link_menu_tests;
#[path = "application_local_shader_tests.rs"]
mod local_shader_tests;
#[path = "application_parameter_file_tests.rs"]
mod parameter_file_tests;
#[path = "application_png_chain_tests.rs"]
mod png_tests;
