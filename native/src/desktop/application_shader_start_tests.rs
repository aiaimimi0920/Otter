//! 菜单/参数→准备→认证取码→受限 child→应用主循环；不开放发行安装或访问用户服务。
use super::*;
use crate::{
    desktop::{
        capability_selection::Selection,
        image::Image,
        loom_result_state::Status,
        shader_prepare::{self, tests as staging},
    },
    loom_bridge_client::tests::{execution_job::shader_fetch::authenticated_socket, fixture},
    shader_angle::Backend,
};
use serde_json::json;
use std::{sync::Arc, thread};
#[path = "application_shader_fallback_tests.rs"]
mod fallback;
#[path = "application_shader_mode_tests.rs"]
mod mode;
#[path = "application_native_shader_tests.rs"]
mod native;
#[path = "application_shader_texture_tests.rs"]
mod texture;
#[path = "application_shader_unit_tests.rs"]
mod unit;
#[derive(Clone, Copy, Debug)]
enum TextureSource {
    Inline,
    Graph,
    Unit,
    DeclaredUnit,
    LinkedUnit,
}
const FRAGMENT: &str = "#version 300 es\nprecision highp float;uniform sampler2D u_input;uniform sampler2D u_overlay;uniform float u_gain;uniform float u_enabled;in vec2 uv;out vec4 color;void main(){vec4 p=texture(u_input,uv);color=vec4(p.rgb*u_gain+texture(u_overlay,uv).rgb*u_enabled,p.a);}";

unsafe fn prepare(app: &mut Application) {
    let document = &app.documents[0];
    crate::desktop::context_menu::execute(
        document.window.0,
        &document.state,
        crate::desktop::execution_start::COMMAND,
    )
    .unwrap();
    app.start_requested_preparations(false);
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.documents[0].state.pending_execution.borrow().is_none() {
        assert!(
            Instant::now() < deadline,
            "shader input preparation timeout"
        );
        app.input_jobs.poll();
        app.retire_preparations();
        app.collect_prepared_inputs();
        thread::sleep(Duration::from_millis(1));
    }
}
unsafe fn run(real: bool, backend: Backend, own: bool, texture: TextureSource) {
    run_mode(real, backend, own, texture, false);
}
unsafe fn run_mode(real: bool, backend: Backend, own: bool, texture: TextureSource, native: bool) {
    let graph_texture = matches!(texture, TextureSource::Graph);
    let unit_texture = matches!(
        texture,
        TextureSource::Unit | TextureSource::DeclaredUnit | TextureSource::LinkedUnit
    );
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
    let (listener, identity, config) = fixture();
    listener.set_nonblocking(true).unwrap();
    let overlay = format!(
        "data:image/png;base64,{}",
        staging::encoded(&staging::png(1, 1, &[8, 16, 24, 255]))
    );
    let mut raw = crate::loom_handshake::tests::fixture();
    let art = &mut raw["capabilities"]["artDefinitions"][0];
    art["metadata"] = json!({"capabilities":{"preview":"shader","shaderInput":"input"}});
    art["inputs"] = json!([{"name":"input","type":"image","required":true},{"name":"reference","type":"image","required":true}]);
    art["parameters"] = json!([
        {"id":"gain","widget":"number","default":0.5,"max":1},
        {"id":"enabled","default":true},{"id":"overlay","default":overlay},
        {"id":"mode","default":"__DISABLED__"},{"id":"output_mode","default":"cpu"},
        {"id":"secret","secret":true,"default":"never-send-shader-secret"}]);
    if graph_texture
        || matches!(
            texture,
            TextureSource::DeclaredUnit | TextureSource::LinkedUnit
        )
    {
        art["inputs"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name":"overlay","type":"image"}));
        art["parameters"][2]["widget"] = json!("image_link");
    }
    if matches!(texture, TextureSource::Unit) {
        art["parameters"].as_array_mut().unwrap().remove(2);
    }
    let handshake = crate::loom_handshake::response(&serde_json::to_vec(&raw).unwrap())
        .unwrap()
        .unwrap();
    let parameters =
        crate::loom_parameter_state::Parameters::new(&handshake.capability(0).unwrap()).unwrap();
    let state = State::new(
        Some(Image {
            width: 1,
            height: 1,
            pixels: if own {
                vec![16, 32, 64, 255]
            } else {
                vec![180, 180, 180, 255]
            }
            .into(),
        }),
        false,
    )
    .unwrap();
    // display-only 的恢复画面不能替代原始底图；有主连接时也不能选自身底图。
    *state.execution_results.borrow_mut() =
        crate::desktop::loom_result_state::Results::from_restored_display(
            state.unit_id.clone(),
            Some(Image {
                width: 1,
                height: 1,
                pixels: vec![200, 200, 200, 255].into(),
            }),
            false,
        );
    *state.selected_art.borrow_mut() = Some(Selection {
        execution: Default::default(),
        identity: new_unit_id().unwrap(),
        parameters,
        catalog: Rc::new(crate::desktop::capability_job::Loaded {
            identity: identity.clone(),
            handshake,
        }),
    });
    // 此测试不启动下游，避免把本地成功的传播变成无关的新远端执行。
    state
        .selected_art
        .borrow_mut()
        .as_mut()
        .unwrap()
        .execution
        .notify_downstream = false;
    let document = DocumentWindow::create(instance, state, false).unwrap();
    let mut app = Application::new(instance, document);
    let target = app.documents[0].state.unit_id.clone();
    let source = State::new(
        Some(Image {
            width: 1,
            height: 1,
            pixels: vec![16, 32, 64, 255].into(),
        }),
        false,
    )
    .unwrap();
    let source_id = source.unit_id.clone();
    let bootstrap = crate::graph_requests::Request {
        id: new_unit_id().unwrap(),
        generation: 1,
    };
    source
        .execution_results
        .borrow_mut()
        .begin(bootstrap.clone());
    source
        .execution_results
        .borrow_mut()
        .finish(&bootstrap, staging::batch(json!({"value":9})))
        .unwrap();
    app.add_state(source, false).unwrap();
    for (from, to) in [
        ("output_image", "input"),
        ("output_image", "reference"),
        ("output", "gain"),
    ] {
        if own && to == "input" {
            continue;
        }
        app.workspace
            .connect(crate::workspace::Link {
                id: new_unit_id().unwrap(),
                from_unit: source_id.clone(),
                from_port: from.into(),
                to_unit: target.clone(),
                to_port: to.into(),
            })
            .unwrap();
    }
    if graph_texture {
        texture::attach(&mut app, &target);
    }
    if unit_texture {
        unit::attach(&mut app, &target, texture);
    }
    // 未安装时维持原拒绝：没有输入 worker、请求注册或连接。
    crate::desktop::execution_start::request(app.documents[0].window.0, &app.documents[0].state)
        .unwrap();
    app.start_requested_preparations(false);
    assert!(app.documents[0]
        .state
        .preparation_error
        .borrow()
        .as_ref()
        .unwrap()
        .contains("本地预览"));
    assert!(app.input_jobs.is_empty());
    assert!(app.executions.is_idle());
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    let fixture = if real && !native {
        std::path::PathBuf::from(
            std::env::var_os("OTTER_SHADER_ANGLE_FIXTURE").expect("Set verified ANGLE fixture"),
        )
    } else {
        std::env::temp_dir()
    };
    app.shader_installation = if native {
        crate::desktop::startup_entry::shader::resolve(&crate::desktop::Startup::default(), false)
            .unwrap()
            .installation
    } else {
        Some(shader_prepare::Installation {
            bundle: Some(Arc::new(
                crate::shader_process::TrustedBundle::new(
                    &fixture,
                    crate::shader_angle::tests::fixture_identity(),
                )
                .unwrap(),
            )),
            backend,
        })
    };
    if own {
        fallback::guards(&mut app);
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
    if graph_texture {
        texture::guards(&mut app);
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
    if unit_texture {
        unit::guards(&mut app, texture);
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
    app.documents[0]
        .state
        .selected_art
        .borrow_mut()
        .as_mut()
        .unwrap()
        .execution
        .param_driven = false;
    crate::desktop::execution_start::parameter_changed(&app.documents[0].state);
    app.start_requested_preparations(false);
    assert!(app.input_jobs.is_empty());
    assert!(app.executions.is_idle());
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    app.documents[0]
        .state
        .selected_art
        .borrow_mut()
        .as_mut()
        .unwrap()
        .execution
        .param_driven = true;
    prepare(&mut app);
    {
        let pending = app.documents[0].state.pending_execution.borrow();
        let execution = pending.as_ref().unwrap();
        assert!(execution.shader.is_some());
        let message: serde_json::Value = serde_json::from_str(&execution.message).unwrap();
        assert_eq!(message["params"]["parameters"]["gain"], 1);
        assert_eq!(message["params"]["parameters"]["mode"], "shader");
        assert_eq!(message["params"]["disabledParameters"], json!([]));
    }
    // 准备后配置被替换：带旧快照的 Shader 请求不能启动连接或占据执行槽。
    app.documents[0]
        .state
        .selected_art
        .borrow_mut()
        .as_mut()
        .unwrap()
        .parameters
        .set("enabled", Some(json!(false)))
        .unwrap();
    app.pending_windows(false).unwrap();
    assert!(app.documents[0].state.pending_execution.borrow().is_none());
    assert!(app.execution_jobs.is_empty());
    assert!(app.executions.is_idle());
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    app.documents[0]
        .state
        .selected_art
        .borrow_mut()
        .as_mut()
        .unwrap()
        .parameters
        .set("enabled", Some(json!(true)))
        .unwrap();
    let iterations = if real { 2 } else { 1 };
    for iteration in 0..iterations {
        if iteration == 1 {
            app.documents[0]
                .state
                .selected_art
                .borrow_mut()
                .as_mut()
                .unwrap()
                .parameters
                .set("enabled", Some(json!(false)))
                .unwrap();
            crate::desktop::execution_start::parameter_changed(&app.documents[0].state);
        } else {
            crate::desktop::execution_start::request(
                app.documents[0].window.0,
                &app.documents[0].state,
            )
            .unwrap();
        }
        app.start_requested_preparations(false);
        let server = (iteration == 0).then(|| {
        listener.set_nonblocking(false).unwrap();
        let server_listener = listener.try_clone().unwrap();
        let server_identity = identity.clone();
        let server_config = config.clone();
        let response_overlay = format!(
            "data:image/png;base64,{}",
            staging::encoded(&staging::png(1, 1, &[128, 0, 0, 255]))
        );
        thread::spawn(move || {
            let mut socket =
                authenticated_socket(&server_listener, server_config, &server_identity);
            let message = socket.read().unwrap();
            assert!(!message
                .to_text()
                .unwrap()
                .contains("never-send-shader-secret"));
            let value: serde_json::Value =
                serde_json::from_str(message.to_text().unwrap()).unwrap();
            let params = &value["params"];
            assert_eq!(params["parameters"]["mode"], "shader");
            assert_eq!(params["parameters"]["output_mode"], "shader");
            assert_eq!(params["parameters"]["gain"], 1);
            assert_eq!(params["parameters"]["enabled"], iteration == 0);
            assert!(params["inputs"]["input"]["dataBase64"].is_string());
            assert_eq!(params["inputs"]["input"], params["inputs"]["reference"]);
            if graph_texture || unit_texture {
                texture::assert_input(&params["inputs"]["overlay"]);
            }
            // 与真实 Loom adapter 一致：图片输入和参数不能重名。
            for name in params["inputs"].as_object().unwrap().keys() {
                assert!(!params["parameters"].as_object().unwrap().contains_key(name));
            }
            let payload = if native {
                assert_eq!(params["parameters"]["shader_abi"], crate::shader_hlsl::ABI);
                json!({"type":"shader","abi":crate::shader_hlsl::ABI,"source":"float4 otter_main(float2 uv){float4 p=u_input.Sample(otter_sampler,uv);return float4(p.rgb*u_gain+u_overlay.Sample(otter_sampler,uv).rgb*u_enabled,p.a);}","uniforms":{"gain":0,"enabled":0},"textures":{"overlay":response_overlay}})
            } else if real {
                json!({"type":"shader","vertex_shader":crate::shader_angle::tests::VERTEX,"fragment_shader":FRAGMENT,"uniforms":{"gain":0,"enabled":0},"textures":{"overlay":response_overlay}})
            } else {
                json!({"type":"unsupported","success":false})
            };
            socket.send(tungstenite::Message::Text(json!({"protocolVersion":"loom.hook.v1","requestId":params["requestId"],"status":"succeeded","data":{"protocolVersion":"loom.hook.v1","requestId":params["requestId"],"nodeId":params["nodeId"],"generation":params["generation"],"resultRevision":1,"outputs":{"output":{"kind":"value","value":payload}}}}).to_string())).unwrap();
        })
        });
        let deadline = Instant::now() + Duration::from_secs(25);
        loop {
            app.pending_windows(false).unwrap();
            let status = app.documents[0].state.execution_results.borrow().status();
            if app.input_jobs.is_empty()
                && app.execution_jobs.is_empty()
                && app.documents[0]
                    .state
                    .pending_preparation
                    .borrow()
                    .is_none()
                && app.documents[0].state.pending_execution.borrow().is_none()
                && matches!(status, Status::Completed | Status::Failed)
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "application shader pipeline timed out: {status:?}"
            );
            thread::sleep(Duration::from_millis(1));
        }
        if let Some(server) = server {
            server.join().unwrap();
        }
        listener.set_nonblocking(true).unwrap();
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        assert!(app.executions.is_idle());
        let results = app.documents[0].state.execution_results.borrow();
        if real {
            assert_eq!(results.status(), Status::Completed);
            let image = results.image_source("output").unwrap().image;
            let expected = if iteration == 0 && (graph_texture || unit_texture) {
                [37, 67, 117, 255]
            } else if iteration == 0 {
                [40, 48, 72, 255]
            } else {
                [16, 32, 64, 255]
            };
            assert_eq!(&*image.pixels, &expected);
            assert_eq!(
                staging::decode_png(&staging::encoded(image.encoded_png().unwrap()))
                    .rgba
                    .iter()
                    .copied()
                    .collect::<Vec<_>>(),
                vec![expected[2], expected[1], expected[0], expected[3]]
            );
        } else {
            assert_eq!(results.status(), Status::Failed);
            assert!(results.image_source("output").is_none());
        }
        let requested_backend = if native {
            "Auto".into()
        } else {
            format!("{backend:?}")
        };
        println!("native-shader-start: backend={requested_backend} iteration={iteration} menu-or-param=true formal-ports=true linked-uniform=true frozen-png=true own-fallback={own} texture={texture:?} real-child={real} cache-hit={} network={}", iteration > 0, iteration == 0);
    }
    app.input_jobs.shutdown();
    assert!(app.execution_jobs.shutdown().is_empty());
}
#[test]
fn shader_application_start_keeps_installation_gate_and_rejects_stale_snapshot_before_network() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap();
    let _input = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    let _apartment = Apartment::new().unwrap();
    unsafe {
        run(false, Backend::D3d11, false, TextureSource::Inline);
    }
}
#[test]
fn shader_application_own_fallback_keeps_missing_reference_preview_and_document_guards() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap();
    let _input = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    let _apartment = Apartment::new().unwrap();
    unsafe {
        run(false, Backend::D3d11, true, TextureSource::Inline);
    }
}
#[test]
#[ignore = "requires verified ANGLE fixture; actual menu and parameter startup over authenticated TLS with real child"]
fn shader_application_start_real_child_uses_formal_ports_resolved_parameters_and_png_overrides() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap();
    let _input = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    let _apartment = Apartment::new().unwrap();
    unsafe {
        for backend in [Backend::D3d11, Backend::Warp] {
            for own in [false, true] {
                run(true, backend, own, TextureSource::Inline);
            }
            run(true, backend, false, TextureSource::Graph);
            run(true, backend, false, TextureSource::Unit);
        }
    }
}
