//! 强制模式/图片重名必须在输入 worker 和请求替换前拒绝；复用原隐藏应用夹具。
use super::*;

#[test]
fn shader_modes_reject_connected_images_before_worker_and_preserve_old_request_and_output() {
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
        for ports in [
            vec!["mode"],
            vec!["output_mode"],
            vec!["mode", "output_mode"],
        ] {
            let (listener, identity, _) = fixture();
            listener.set_nonblocking(true).unwrap();
            let mut raw = crate::loom_handshake::tests::fixture();
            let art = &mut raw["capabilities"]["artDefinitions"][0];
            art["metadata"] = json!({"capabilities":{"preview":"shader","shaderInput":"input"}});
            let names: Vec<_> = std::iter::once("input")
                .chain(ports.iter().copied())
                .collect();
            art["inputs"] = names
                .iter()
                .map(|name| json!({"name":name,"type":"image","required":true}))
                .collect();
            // 不声明同名标量，仍必须基于真实图片端口而非参数声明检查冲突。
            art["parameters"] = json!([]);
            let handshake = crate::loom_handshake::response(&serde_json::to_vec(&raw).unwrap())
                .unwrap()
                .unwrap();
            let parameters =
                crate::loom_parameter_state::Parameters::new(&handshake.capability(0).unwrap())
                    .unwrap();
            let state = State::new(
                Some(Image {
                    width: 1,
                    height: 1,
                    pixels: vec![11, 22, 33, 255].into(),
                }),
                false,
            )
            .unwrap();
            texture::commit(&state);
            let output = state.execution_results.borrow().output_stamp();
            let original = state
                .execution_results
                .borrow()
                .image_source("output")
                .unwrap()
                .image;
            *state.selected_art.borrow_mut() = Some(Selection {
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
            let target = app.documents[0].state.unit_id.clone();
            let source = State::new(
                Some(Image {
                    width: 1,
                    height: 1,
                    pixels: vec![44, 55, 66, 255].into(),
                }),
                false,
            )
            .unwrap();
            let source_id = source.unit_id.clone();
            app.add_state(source, false).unwrap();
            for name in names {
                app.workspace
                    .connect(crate::workspace::Link {
                        id: new_unit_id().unwrap(),
                        from_unit: source_id.clone(),
                        from_port: "output_image".into(),
                        to_unit: target.clone(),
                        to_port: name.into(),
                    })
                    .unwrap();
            }
            app.shader_installation = Some(shader_prepare::Installation {
                bundle: Some(Arc::new(
                    crate::shader_process::TrustedBundle::new(
                        &std::env::temp_dir(),
                        crate::shader_angle::tests::fixture_identity(),
                    )
                    .unwrap(),
                )),
                backend: Backend::D3d11,
            });
            // 注册表旧令牌与已提交输出分别保存；没有活动 worker，不用 busy 门掩盖配置拒绝。
            let old = app
                .executions
                .begin(target.clone(), new_unit_id().unwrap())
                .unwrap();
            app.executions
                .mark_preview(&target, &old, "retained-preview".into())
                .unwrap();
            crate::desktop::execution_start::request(
                app.documents[0].window.0,
                &app.documents[0].state,
            )
            .unwrap();
            assert!(crate::desktop::execution_start::ready(
                &app.documents[0].state
            ));
            app.start_requested_preparations(false);
            let state = &app.documents[0].state;
            assert_eq!(
                state.preparation_error.borrow().as_deref(),
                Some("无法捕获执行参数")
            );
            assert!(state.pending_start.borrow().is_none());
            assert!(state.pending_preparation.borrow().is_none());
            assert!(state.pending_execution.borrow().is_none());
            assert!(state.execution_configuration.borrow().is_none());
            assert!(app.input_jobs.is_empty());
            assert!(app.execution_jobs.is_empty());
            assert!(app.executions.is_latest(&target, &old));
            assert_eq!(
                app.executions.preview(&target, &old),
                Some("retained-preview")
            );
            let results = state.execution_results.borrow();
            assert_eq!(results.status(), Status::Completed);
            assert!(output.matches(&results));
            assert_eq!(
                &*results.image_source("output").unwrap().image.pixels,
                &*original.pixels
            );
            assert_eq!(
                listener.accept().unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
        }
    }
}
