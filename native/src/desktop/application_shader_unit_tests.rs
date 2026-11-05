//! 未连接 Unit 图片引用的真实应用来源/参数一致性与 freshness；复用原 Class/COM/TLS。
use super::*;
use crate::workspace_id::UnitId;

fn values(app: &Application) -> serde_json::Map<String, serde_json::Value> {
    app.documents[0]
        .state
        .selected_art
        .borrow()
        .as_ref()
        .unwrap()
        .parameters
        .entries()
        .filter_map(|(name, value)| value.map(|value| (name.into(), value.clone())))
        .collect()
}
fn replace(app: &Application, values: serde_json::Map<String, serde_json::Value>) {
    app.documents[0]
        .state
        .selected_art
        .borrow_mut()
        .as_mut()
        .unwrap()
        .parameters
        .replace(values)
        .unwrap();
}
fn reference(app: &Application, mode: TextureSource, unit: &UnitId) {
    if matches!(mode, TextureSource::LinkedUnit) {
        let request = crate::graph_requests::Request {
            id: new_unit_id().unwrap(),
            generation: 1,
        };
        let mut result = app.documents[4].state.execution_results.borrow_mut();
        result.begin(request.clone());
        result
            .finish(&request, staging::batch(json!(unit.as_str())))
            .unwrap();
    } else {
        let mut next = values(app);
        next.insert("overlay".into(), json!(unit.as_str()));
        replace(app, next);
    }
}
pub(super) unsafe fn attach(app: &mut Application, target: &UnitId, mode: TextureSource) {
    let mut source = State::new(None, false).unwrap();
    source.restored_art = true;
    let source_id = source.unit_id.clone();
    texture::commit(&source);
    app.add_state(source, false).unwrap();
    let relay = State::new(
        Some(Image {
            width: 1,
            height: 1,
            pixels: vec![1, 2, 3, 255].into(),
        }),
        false,
    )
    .unwrap();
    let relay_id = relay.unit_id.clone();
    app.add_state(relay, false).unwrap();
    app.workspace
        .connect(crate::workspace::Link {
            id: new_unit_id().unwrap(),
            from_unit: source_id,
            from_port: "output".into(),
            to_unit: relay_id.clone(),
            to_port: "image".into(),
        })
        .unwrap();
    if matches!(mode, TextureSource::LinkedUnit) {
        // manual 仍是旧内联 PNG；同名标量连接必须让同轮 resolved Unit 来源取代它。
        let mut scalar = State::new(None, false).unwrap();
        scalar.restored_art = true;
        let scalar_id = scalar.unit_id.clone();
        let request = crate::graph_requests::Request {
            id: new_unit_id().unwrap(),
            generation: 1,
        };
        scalar.execution_results.borrow_mut().begin(request.clone());
        scalar
            .execution_results
            .borrow_mut()
            .finish(&request, staging::batch(json!(relay_id.as_str())))
            .unwrap();
        app.add_state(scalar, false).unwrap();
        app.workspace
            .connect(crate::workspace::Link {
                id: new_unit_id().unwrap(),
                from_unit: scalar_id,
                from_port: "output".into(),
                to_unit: target.clone(),
                to_port: "overlay".into(),
            })
            .unwrap();
    } else {
        let mut values = values(app);
        values.insert("overlay".into(), json!(relay_id.as_str()));
        let mut selected = app.documents[0].state.selected_art.borrow_mut();
        let selected = selected.as_mut().unwrap();
        selected.parameters = crate::loom_parameter_state::Parameters::from_persisted(
            &selected.catalog.handshake.capability(0).unwrap(),
            values,
        )
        .unwrap();
    }
}
type CapturedInputs = (Vec<String>, Vec<String>);
fn inputs(app: &Application) -> Result<Option<CapturedInputs>, &'static str> {
    let selected = app.documents[0].state.selected_art.borrow();
    let selected = selected.as_ref().unwrap();
    let cap = selected.catalog.handshake.capability(0).unwrap();
    Application::with_parameters(
        &app.documents,
        &app.workspace,
        &app.documents[0].state.unit_id,
        &cap,
        &selected.parameters,
        |resolved| {
            Application::image_inputs(
                &app.documents,
                &app.live_sessions,
                &app.workspace,
                &app.documents[0].state.unit_id,
                false,
                Some(&cap),
                Some(resolved),
            )
        },
    )
    .map(|(inputs, _)| inputs.map(|inputs| (inputs.ports, inputs.shader_parameters)))
}
fn assert_retired(app: &Application) {
    assert!(app.documents[0].state.pending_execution.borrow().is_none());
    assert!(app.execution_jobs.is_empty());
    assert!(app.executions.is_idle());
}
pub(super) unsafe fn guards(app: &mut Application, mode: TextureSource) {
    let captured = inputs(app)
        .unwrap()
        .expect("automatic Unit inputs must satisfy declared image ports");
    assert!(captured.0.iter().any(|name| name == "overlay"));
    assert_eq!(captured.1, ["overlay"]);
    drop(captured);
    // 引用 Sticker 未连到目标，仍要捕获其实际正式上游及中继配置/草稿。
    prepare(app);
    texture::commit(&app.documents[2].state);
    app.pending_windows(false).unwrap();
    assert_retired(app);
    prepare(app);
    app.documents[3].state.accept_upstream.set(false);
    app.pending_windows(false).unwrap();
    assert_retired(app);
    app.documents[3].state.accept_upstream.set(true);
    prepare(app);
    app.documents[3]
        .state
        .canvas
        .borrow_mut()
        .editor
        .begin(
            crate::editor::Tool::Brush,
            crate::viewport::Point { x: 0.0, y: 0.0 },
        )
        .unwrap();
    app.pending_windows(false).unwrap();
    assert_retired(app);
    app.documents[3].state.canvas.borrow_mut().editor.cancel();
    let link = app
        .workspace
        .links()
        .iter()
        .find(|link| link.to_unit == app.documents[3].state.unit_id)
        .unwrap()
        .clone();
    prepare(app);
    assert!(app.workspace.remove_link(&link.id));
    app.pending_windows(false).unwrap();
    assert_retired(app);
    app.workspace.connect(link).unwrap();
    let original = values(app);
    prepare(app);
    let mut changed = original.clone();
    changed.insert(
        "overlay".into(),
        json!(app.documents[1].state.unit_id.as_str()),
    );
    replace(app, changed);
    app.pending_windows(false).unwrap();
    assert_retired(app);
    replace(app, original);
    // 直接引用 Art 即使有 display，也不能绕过正式输出门。
    // Sticker 在上游缺图时可合法退回自身底图，不能把该行为误断言为 Art 预览泄漏。
    reference(app, mode, &app.documents[2].state.unit_id);
    let display = Some(Image {
        width: 1,
        height: 1,
        pixels: vec![240; 4].into(),
    });
    let saved = std::mem::replace(
        &mut *app.documents[2].state.execution_results.borrow_mut(),
        crate::desktop::loom_result_state::Results::from_restored_display(
            app.documents[2].state.unit_id.clone(),
            display,
            false,
        ),
    );
    assert!(inputs(app).is_err());
    crate::desktop::execution_start::request(app.documents[0].window.0, &app.documents[0].state)
        .unwrap();
    app.start_requested_preparations(false);
    assert!(app.input_jobs.is_empty());
    assert_retired(app);
    assert!(app.documents[0]
        .state
        .preparation_error
        .borrow()
        .as_ref()
        .unwrap()
        .contains("正式图片"));
    *app.documents[2].state.execution_results.borrow_mut() = saved;
    reference(app, mode, &app.documents[3].state.unit_id);
    if matches!(mode, TextureSource::DeclaredUnit) {
        connected_priority(app);
    }
}
unsafe fn connected_priority(app: &mut Application) {
    let link = crate::workspace::Link {
        id: new_unit_id().unwrap(),
        from_unit: app.documents[2].state.unit_id.clone(),
        from_port: "output".into(),
        to_unit: app.documents[0].state.unit_id.clone(),
        to_port: "overlay".into(),
    };
    app.workspace.connect(link.clone()).unwrap();
    app.documents[3]
        .state
        .canvas
        .borrow_mut()
        .editor
        .begin(
            crate::editor::Tool::Brush,
            crate::viewport::Point { x: 0.0, y: 0.0 },
        )
        .unwrap();
    assert!(
        inputs(app).unwrap().is_some(),
        "shadowed Unit draft must not block connected image"
    );
    prepare(app);
    app.documents[3].state.accept_upstream.set(false);
    {
        let pending = app.documents[0].state.pending_execution.borrow();
        let inputs = &pending.as_ref().unwrap().inputs;
        assert!(inputs.freshness().unwrap().is_current(|unit, stamp| app
            .documents
            .iter()
            .find(|doc| doc.state.unit_id == *unit)
            .is_some_and(|doc| stamp.matches_state(&doc.state))));
        let (_, crate::loom_art_wire::Input::InlinePng { base64, .. }) = inputs
            .descriptors()
            .find(|(port, _)| *port == "overlay")
            .unwrap()
        else {
            panic!("formal PNG")
        };
        assert_eq!(&*staging::decode_png(base64).rgba, &[53, 35, 21, 1]);
    }
    app.documents[3].state.canvas.borrow_mut().editor.cancel();
    app.documents[3].state.accept_upstream.set(true);
    assert!(app.workspace.remove_link(&link.id));
    app.pending_windows(false).unwrap();
    assert_retired(app);
}
#[test]
fn shader_application_unit_parameters_preserve_legacy_and_declared_sources_and_linked_override() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap();
    let _input = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    let _apartment = Apartment::new().unwrap();
    unsafe {
        for mode in [
            TextureSource::Unit,
            TextureSource::DeclaredUnit,
            TextureSource::LinkedUnit,
        ] {
            run(false, Backend::D3d11, false, mode);
        }
    }
}
