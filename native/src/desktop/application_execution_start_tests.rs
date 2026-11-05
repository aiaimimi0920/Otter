use super::*;
use crate::desktop::application::execution_test_peer as test_peer;
use crate::desktop::{document_window::DocumentWindow, Apartment, State};
use std::{
    rc::Rc,
    time::{Duration, Instant},
};
use windows::Win32::{Foundation::HINSTANCE, System::LibraryLoader::GetModuleHandleW};
#[path = "application_shader_start_tests.rs"]
mod shader;
struct Class(HINSTANCE);
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Native.Workspace"), Some(self.0));
        }
    }
}
#[test]
fn menu_start_prepares_fixed_identity_and_configuration_without_connecting_on_ui_thread() {
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
        let state = State::new(None, false).unwrap();
        let (listener, identity, config) = crate::loom_bridge_client::tests::fixture();
        listener.set_nonblocking(true).unwrap();
        let mut first = crate::loom_handshake::tests::fixture();
        first["capabilities"]["artDefinitions"][0]["inputs"] =
            serde_json::json!([{"name":"input_image","type":"image","required":true}]);
        // 手动运行缺图仍发出真实的空 inputs，不伪造输入也不阻止服务端给出结果。
        let handshake = crate::loom_handshake::response(&serde_json::to_vec(&first).unwrap())
            .unwrap()
            .unwrap();
        let parameters =
            crate::loom_parameter_state::Parameters::new(&handshake.capability(0).unwrap())
                .unwrap();
        *state.selected_art.borrow_mut() = Some(crate::desktop::capability_selection::Selection {
            execution: Default::default(),
            identity: new_unit_id().unwrap(),
            parameters,
            catalog: Rc::new(crate::desktop::capability_job::Loaded {
                identity,
                handshake,
            }),
        });
        let document = DocumentWindow::create(instance, state, false).unwrap();
        let mut app = Application::new(instance, document);
        let window = app.documents[0].window.0;
        let busy = crate::graph_requests::Request {
            id: new_unit_id().unwrap(),
            generation: 1,
        };
        app.documents[0]
            .state
            .execution_results
            .borrow_mut()
            .begin(busy.clone());
        crate::desktop::execution_start::parameter_changed(&app.documents[0].state);
        app.start_requested_preparations(false);
        assert!(app.documents[0].state.pending_start.borrow().is_some());
        assert!(app.input_jobs.is_empty());
        crate::desktop::context_menu::execute(
            window,
            &app.documents[0].state,
            crate::desktop::execution_cancel::COMMAND,
        )
        .unwrap();
        assert!(app.documents[0].state.pending_start.borrow().is_none());
        app.cancel_requested_executions();
        crate::desktop::execution_start::parameter_changed(&app.documents[0].state);
        app.documents[0]
            .state
            .execution_results
            .borrow_mut()
            .cancel(&busy);
        app.start_requested_preparations(false);
        assert!(app.documents[0].state.pending_start.borrow().is_none());
        assert!(app.input_jobs.is_empty());
        assert!(app.documents[0]
            .state
            .pending_preparation
            .borrow()
            .is_none());
        assert!(app.documents[0].state.preparation_error.borrow().is_none());
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        crate::desktop::context_menu::execute(
            window,
            &app.documents[0].state,
            crate::desktop::execution_start::COMMAND,
        )
        .unwrap();
        assert!(app.documents[0].state.pending_start.borrow().is_some());
        app.start_requested_preparations(false);
        assert!(app.documents[0]
            .state
            .pending_preparation
            .borrow()
            .is_some());
        assert_eq!(app.input_jobs.len(), 1);
        assert!(!crate::desktop::execution_start::available(
            &app.documents[0].state
        ));
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.documents[0].state.pending_execution.borrow().is_none() {
            assert!(Instant::now() < deadline);
            app.input_jobs.poll();
            app.retire_preparations();
            app.collect_prepared_inputs();
            std::thread::yield_now();
        }
        let execution = app.documents[0]
            .state
            .pending_execution
            .borrow_mut()
            .take()
            .unwrap();
        let wire: serde_json::Value = serde_json::from_str(&execution.message).unwrap();
        assert_eq!(wire["params"]["artId"], "test-art");
        assert_eq!(wire["params"]["inputs"], serde_json::json!({}));
        assert!(!execution.message.contains("secret"));
        assert!(crate::desktop::execution_configuration::matches(
            &app.documents[0].state,
            &execution.request,
            &app.workspace
        ));
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        let fixed_identity = execution.identity.clone();
        let unit = execution.unit.clone();
        let server = test_peer::serve(listener.try_clone().unwrap(), config.clone(), 1);
        *app.documents[0].state.pending_execution.borrow_mut() = Some(execution);
        app.pending_windows(false).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while app.documents[0].state.execution_results.borrow().status()
            == crate::desktop::loom_result_state::Status::Running
        {
            let mut message = MSG::default();
            if PeekMessageW(&mut message, None, WM_APP + 92, WM_APP + 92, PM_REMOVE).as_bool() {
                app.pending_windows(false).unwrap();
            }
            assert!(Instant::now() < deadline, "menu result timeout");
            std::thread::sleep(Duration::from_millis(1));
        }
        server.join().unwrap();
        {
            let results = app.documents[0].state.execution_results.borrow();
            assert_eq!(
                results.status(),
                crate::desktop::loom_result_state::Status::Completed
            );
            let crate::desktop::loom_output_batch::Output::Value(value) = &results.ports()[0].value
            else {
                panic!("expected scalar result");
            };
            assert_eq!(*value, serde_json::json!(7));
        }
        assert!(app.execution_jobs.is_empty());
        assert!(app.documents[0]
            .state
            .execution_configuration
            .borrow()
            .is_none());
        app.input_jobs.shutdown();
        let mut raw = crate::loom_handshake::tests::fixture();
        let reference = crate::desktop::loom_inline::prepare(
            1,
            1,
            vec![77, 88, 99, 255].into(),
            &crate::editor::Editor::default(),
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
        let crate::loom_art_wire::Input::InlinePng { base64, .. } = reference.descriptor() else {
            panic!("inline")
        };
        let reference_uri = format!("data:image/png;base64,{base64}");
        raw["capabilities"]["artDefinitions"][0]["inputs"] = serde_json::json!([{"name":"input_image","type":"image","required":true},
                {"name":"reference_image","type":"image","required":true}]);
        raw["capabilities"]["artDefinitions"][0]["parameters"] = serde_json::json!([
            {"id":"secret","secret":true},
            {"id":"amount","widget":"number","default":2,"max":10},
            {"id":"reference_image","widget":"image_link","default":reference_uri}
        ]);
        let handshake = crate::loom_handshake::response(&serde_json::to_vec(&raw).unwrap())
            .unwrap()
            .unwrap();
        let parameters =
            crate::loom_parameter_state::Parameters::new(&handshake.capability(0).unwrap())
                .unwrap();
        *app.documents[0].state.selected_art.borrow_mut() =
            Some(crate::desktop::capability_selection::Selection {
                execution: Default::default(),
                identity: new_unit_id().unwrap(),
                parameters,
                catalog: Rc::new(crate::desktop::capability_job::Loaded {
                    identity: fixed_identity,
                    handshake,
                }),
            });
        app.documents[0]
            .state
            .selected_art
            .borrow_mut()
            .as_mut()
            .unwrap()
            .parameters
            .set(
                "reference_image",
                Some(serde_json::json!("data:text/plain;base64,YQ==")),
            )
            .unwrap();
        crate::desktop::context_menu::execute(
            window,
            &app.documents[0].state,
            crate::desktop::execution_start::COMMAND,
        )
        .unwrap();
        app.start_requested_preparations(false);
        assert!(app.documents[0]
            .state
            .preparation_error
            .borrow()
            .as_ref()
            .unwrap()
            .contains("内联图片"));
        assert!(app.input_jobs.is_empty());
        assert!(app.documents[0]
            .state
            .pending_preparation
            .borrow()
            .is_none());
        assert!(app.executions.active(&unit).is_none());
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        let source = State::new(
            Some(crate::desktop::image::Image {
                width: 1,
                height: 1,
                pixels: vec![11, 22, 33, 255].into(),
            }),
            false,
        )
        .unwrap();
        app.documents[0]
            .state
            .selected_art
            .borrow_mut()
            .as_mut()
            .unwrap()
            .parameters
            .set("reference_image", Some(serde_json::json!(reference_uri)))
            .unwrap();
        let source_id = source.unit_id.clone();
        let scalar_request = crate::graph_requests::Request {
            id: new_unit_id().unwrap(),
            generation: 1,
        };
        source
            .execution_results
            .borrow_mut()
            .begin(scalar_request.clone());
        let scalar = crate::desktop::loom_output_batch::decode(
            &serde_json::json!({"output":{"kind":"value","value":{"value":"12.5"}}}),
            None,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
        source
            .execution_results
            .borrow_mut()
            .finish(&scalar_request, scalar)
            .unwrap();
        app.add_state(source, false).unwrap();
        app.add_state(State::new(None, false).unwrap(), false)
            .unwrap();
        let upstream = State::new(
            Some(crate::desktop::image::Image {
                width: 1,
                height: 1,
                pixels: vec![44, 55, 66, 255].into(),
            }),
            false,
        )
        .unwrap();
        let upstream_id = upstream.unit_id.clone();
        app.add_state(upstream, false).unwrap();
        app.workspace
            .connect(crate::workspace::Link {
                id: new_unit_id().unwrap(),
                from_unit: upstream_id,
                from_port: "output_image".into(),
                to_unit: source_id.clone(),
                to_port: "image".into(),
            })
            .unwrap();
        // 关闭中继后必须发送钉图自己的 11/22/33，而不是更上游的 44/55/66。
        crate::desktop::context_menu::execute(
            app.documents[1].window.0,
            &app.documents[1].state,
            crate::desktop::sticker_propagation::COMMAND,
        )
        .unwrap();
        assert!(!app.documents[1].state.accept_upstream.get());
        app.workspace
            .connect(crate::workspace::Link {
                id: new_unit_id().unwrap(),
                from_unit: source_id.clone(),
                from_port: "output_image".into(),
                to_unit: unit.clone(),
                to_port: "input_image".into(),
            })
            .unwrap();
        app.workspace
            .connect(crate::workspace::Link {
                id: new_unit_id().unwrap(),
                from_unit: source_id,
                from_port: "output".into(),
                to_unit: unit.clone(),
                to_port: "amount".into(),
            })
            .unwrap();
        crate::desktop::execution_start::parameter_changed(&app.documents[0].state);
        app.start_requested_preparations(false);
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.documents[0].state.pending_execution.borrow().is_none() {
            assert!(Instant::now() < deadline, "linked preparation timeout");
            app.input_jobs.poll();
            app.retire_preparations();
            app.collect_prepared_inputs();
            std::thread::sleep(Duration::from_millis(1));
        }
        let execution = app.documents[0]
            .state
            .pending_execution
            .borrow_mut()
            .take()
            .unwrap();
        let wire: serde_json::Value = serde_json::from_str(&execution.message).unwrap();
        let image = crate::desktop::loom_output_inline::decode(
            &wire["params"]["inputs"]["input_image"],
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(&*image.pixels, &[11, 22, 33, 255]);
        assert_eq!(
            wire["params"]["parameters"]["amount"],
            serde_json::json!(10)
        );
        assert_eq!(
            app.documents[0]
                .state
                .selected_art
                .borrow()
                .as_ref()
                .unwrap()
                .parameters
                .get("amount"),
            Some(Some(&serde_json::json!(2)))
        );
        // 无关文档的编辑身份改变不能取消此上游子图的执行。
        let unrelated = &app.documents[2].state;
        let old = unrelated.canvas.borrow().editor.document_stamp();
        unrelated.canvas.borrow_mut().editor = crate::editor::Editor::default();
        assert!(!old.matches(&unrelated.canvas.borrow().editor));
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        // 图片沿生产发送路径到达服务端，而不只验证本地消息编码。
        let server = test_peer::serve(listener.try_clone().unwrap(), config.clone(), 2);
        *app.documents[0].state.pending_execution.borrow_mut() = Some(execution);
        app.pending_windows(false).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while app.documents[0].state.execution_results.borrow().status()
            == crate::desktop::loom_result_state::Status::Running
        {
            let mut message = MSG::default();
            if PeekMessageW(&mut message, None, WM_APP + 92, WM_APP + 92, PM_REMOVE).as_bool() {
                app.pending_windows(false).unwrap();
            }
            assert!(Instant::now() < deadline, "image menu result timeout");
            std::thread::sleep(Duration::from_millis(1));
        }
        server.join().unwrap();
        let results = app.documents[0].state.execution_results.borrow();
        assert_eq!(
            results.status(),
            crate::desktop::loom_result_state::Status::Completed
        );
        let crate::desktop::loom_output_batch::Output::Value(value) = &results.ports()[0].value
        else {
            panic!("expected image execution scalar result");
        };
        assert_eq!(*value, serde_json::json!(33));
        drop(results);
        app.verify_direct_propagation();
        // 同一生产入口对纯本地 shader 能力拒绝后台派发，不以错误执行冒充预览支持。
        raw["capabilities"]["artDefinitions"][0]["metadata"] =
            serde_json::json!({"capabilities":{"preview":"shader"}});
        let handshake = crate::loom_handshake::response(&serde_json::to_vec(&raw).unwrap())
            .unwrap()
            .unwrap();
        {
            let mut selected = app.documents[0].state.selected_art.borrow_mut();
            let selected = selected.as_mut().unwrap();
            selected.catalog = Rc::new(crate::desktop::capability_job::Loaded {
                identity: selected.catalog.identity.clone(),
                handshake,
            });
        }
        crate::desktop::context_menu::execute(
            window,
            &app.documents[0].state,
            crate::desktop::execution_start::COMMAND,
        )
        .unwrap();
        app.start_requested_preparations(false);
        assert!(app.documents[0]
            .state
            .preparation_error
            .borrow()
            .as_ref()
            .unwrap()
            .contains("本地预览"));
        assert!(app.documents[0]
            .state
            .pending_preparation
            .borrow()
            .is_none());
        assert!(app.input_jobs.is_empty());
        assert!(app.executions.active(&unit).is_none());
        assert!(app.execution_jobs.is_empty());
        assert!(app.documents[0]
            .state
            .execution_configuration
            .borrow()
            .is_none());
    }
}
