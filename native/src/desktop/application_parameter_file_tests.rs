//! 真实文件 worker/隐藏窗口提交回归；仅在明确 scratch 根目录创建合成文件。
use super::*;
use crate::desktop::{
    configuration_stamp::Stamp,
    parameter_file_input::{self, Request},
};
fn request(app: &Application, path: std::path::PathBuf) -> Request {
    let state = &app.documents[0].state;
    Request {
        path,
        target: parameter_file_input::Target::Parameter("upload".into()),
        stamp: Stamp::capture(state),
        view: state
            .parameter_view
            .borrow()
            .as_ref()
            .map(|view| view.identity.clone()),
    }
}
unsafe fn finish(app: &mut Application) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.parameter_file_job.is_some() {
        assert!(Instant::now() < deadline, "file worker did not settle");
        app.pending_parameter_files(false).unwrap();
        std::thread::sleep(Duration::from_millis(2));
    }
}
pub(super) unsafe fn verify(app: &mut Application) {
    let original = app.documents[0]
        .state
        .selected_art
        .borrow()
        .as_ref()
        .unwrap()
        .parameters
        .clone();
    app.sync_display_frames();
    let path =
        std::path::PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi").join(format!(
            "otter-param-{}.txt",
            crate::desktop::document_state::new_unit_id()
                .unwrap()
                .as_str()
        ));
    std::fs::write(&path, b"native file parameter").unwrap();
    let doc = &app.documents[0];
    for id in ["secret", "amount", "missing"] {
        let mut invalid = request(app, path.clone());
        invalid.target = parameter_file_input::Target::Parameter(id.into());
        assert!(!parameter_file_input::queue(doc.window.0, &doc.state, invalid).unwrap());
    }
    doc.state.group_locked.set(true);
    assert!(
        !parameter_file_input::queue(doc.window.0, &doc.state, request(app, path.clone())).unwrap()
    );
    doc.state.group_locked.set(false);
    assert!(
        parameter_file_input::queue(doc.window.0, &doc.state, request(app, path.clone())).unwrap()
    );
    app.pending_parameter_files(false).unwrap();
    assert!(app.parameter_file_job.is_some());
    let newer = path.with_extension("replacement.txt");
    // 大于普通文本转换的 64 KiB 上限，证明文件参数没有被当作短文本截断。
    let bytes: Vec<_> = (0..128 * 1024).map(|index| index as u8).collect();
    std::fs::write(&newer, &bytes).unwrap();
    let doc = &app.documents[0];
    assert!(
        parameter_file_input::queue(doc.window.0, &doc.state, request(app, newer.clone())).unwrap()
    );
    finish(app);
    use base64::Engine;
    let expected = serde_json::json!(format!(
        "data:text/plain;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ));
    assert_eq!(
        app.documents[0]
            .state
            .selected_art
            .borrow()
            .as_ref()
            .unwrap()
            .parameters
            .get("upload"),
        Some(Some(&expected))
    );
    assert!(app.documents[0].state.pending_start.borrow().is_some());
    verify_wire(app, &expected);
    app.documents[0].state.pending_start.borrow_mut().take();
    app.sync_display_frames();
    // 读取失败不能清空之前已经提交的值，也不能触发新执行。
    assert!(
        app.documents[0]
            .state
            .parameter_view
            .borrow()
            .as_ref()
            .unwrap()
            .values
            .is_empty(),
        "unlinked file data must not be duplicated in the display view"
    );
    let doc = &app.documents[0];
    assert!(parameter_file_input::queue(
        doc.window.0,
        &doc.state,
        request(app, path.with_extension("does-not-exist"))
    )
    .unwrap());
    app.pending_parameter_files(false).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let error = loop {
        assert!(Instant::now() < deadline);
        if let Err(error) = app.pending_parameter_files(false) {
            break error;
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    assert!(error.to_string().contains("文件读取失败"));
    assert!(app.parameter_file_job.is_none());
    assert!(app.documents[0].state.pending_start.borrow().is_none());
    verify_wire(app, &expected);
    std::fs::write(&path, b"stale replacement").unwrap();
    let doc = &app.documents[0];
    assert!(
        parameter_file_input::queue(doc.window.0, &doc.state, request(app, path.clone())).unwrap()
    );
    app.pending_parameter_files(false).unwrap();
    // 修改配置后，即使后台读取已经结束但尚未发布，也不能写回旧选择。
    app.documents[0]
        .state
        .selected_art
        .borrow_mut()
        .as_mut()
        .unwrap()
        .parameters
        .set("amount", Some(serde_json::json!(99)))
        .unwrap();
    finish(app);
    assert_eq!(
        app.documents[0]
            .state
            .selected_art
            .borrow()
            .as_ref()
            .unwrap()
            .parameters
            .get("upload"),
        Some(Some(&expected))
    );
    assert!(app.documents[0].state.pending_start.borrow().is_none());
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(newer).unwrap();
    verify_restored_defaults(app);
    app.documents[0]
        .state
        .selected_art
        .borrow_mut()
        .as_mut()
        .unwrap()
        .parameters = original;
    app.sync_display_frames();
}

fn verify_wire(app: &Application, expected: &serde_json::Value) {
    let json = encoded(app);
    assert_eq!(&json["params"]["parameters"]["upload"], expected);
    assert!(json["params"]["parameters"].get("secret").is_none());
    assert!(
        !json.to_string().contains("otter-param-"),
        "local path must not enter execution wire"
    );
}
fn encoded(app: &Application) -> serde_json::Value {
    let state = &app.documents[0].state;
    let selected = state.selected_art.borrow();
    let selected = selected.as_ref().unwrap();
    let capability = selected.catalog.handshake.capability(0).unwrap();
    let (plan, _) = Application::execution_parameters(
        &app.documents,
        &app.workspace,
        &state.unit_id,
        &capability,
        &selected.parameters,
        None,
        |_| {
            Ok(Some(super::image_inputs::ImageInputs {
                ports: Vec::new(),
                sources: Vec::new(),
                shader_parameters: Vec::new(),
            }))
        },
    )
    .unwrap();
    let (config, _, _) = plan.unwrap();
    let wire = crate::loom_art_wire::execute_config(
        &state.unit_id,
        &crate::graph_requests::Request {
            id: crate::desktop::document_state::new_unit_id().unwrap(),
            generation: 1,
        },
        &config,
        &[],
        crate::loom_art_wire::OutputTransport::Websocket,
    )
    .unwrap();
    serde_json::from_str(&wire).unwrap()
}
fn verify_restored_defaults(app: &Application) {
    for (raw, expected) in [
        ("{}", serde_json::json!(1)),
        (r#"{"amount":null}"#, serde_json::Value::Null),
        (
            r#"{"legacy":{"n":42},"__exec_mode":"ignore"}"#,
            serde_json::json!(1),
        ),
    ] {
        let raw = serde_json::value::RawValue::from_string(raw.into()).unwrap();
        let parameters = {
            let selection = app.documents[0].state.selected_art.borrow();
            let capability = selection
                .as_ref()
                .unwrap()
                .catalog
                .handshake
                .capability(0)
                .unwrap();
            crate::session_parameters::restore(&capability, Some(&raw)).unwrap()
        };
        app.documents[0]
            .state
            .selected_art
            .borrow_mut()
            .as_mut()
            .unwrap()
            .parameters = parameters;
        assert_eq!(encoded(app)["params"]["parameters"]["amount"], expected);
        if raw.get().contains("legacy") {
            let wire = encoded(app);
            assert_eq!(
                wire["params"]["parameters"]["legacy"],
                serde_json::json!({"n":42})
            );
            assert!(wire["params"]["parameters"].get("__exec_mode").is_none());
            app.documents[0]
                .state
                .selected_art
                .borrow_mut()
                .as_mut()
                .unwrap()
                .parameters
                .set("amount", Some(serde_json::json!(7)))
                .unwrap();
            assert_eq!(
                encoded(app)["params"]["parameters"]["legacy"],
                serde_json::json!({"n":42})
            );
        }
    }
}
