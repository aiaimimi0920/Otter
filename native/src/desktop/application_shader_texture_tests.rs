//! 公共图像参数的正式端口、同源 PNG 及准备后的来源/拓扑拒绝；复用原隐藏应用夹具。
use super::*;
const RGBA: [u8; 4] = [53, 35, 21, 1];

pub(super) fn commit(state: &State) {
    let png = staging::png(1, 1, &RGBA);
    let image = Image::from_encoded(&png).unwrap();
    let image = crate::desktop::loom_image_value::ImageValue::from_png(image, png).unwrap();
    let request = crate::graph_requests::Request {
        id: new_unit_id().unwrap(),
        generation: 1,
    };
    let mut results = state.execution_results.borrow_mut();
    results.begin(request.clone());
    results
        .finish(
            &request,
            crate::desktop::loom_output_batch::Batch::local_image(image).unwrap(),
        )
        .unwrap();
}
pub(super) unsafe fn attach(app: &mut Application, target: &crate::workspace_id::UnitId) {
    let mut state = State::new(None, false).unwrap();
    state.restored_art = true;
    let source = state.unit_id.clone();
    commit(&state);
    app.add_state(state, false).unwrap();
    app.workspace
        .connect(crate::workspace::Link {
            id: new_unit_id().unwrap(),
            from_unit: source,
            from_port: "output".into(),
            to_unit: target.clone(),
            to_port: "overlay".into(),
        })
        .unwrap();
}
pub(super) fn assert_input(value: &serde_json::Value) {
    assert_eq!(value["width"], 1);
    assert_eq!(value["height"], 1);
    assert_eq!(
        &*staging::decode_png(value["dataBase64"].as_str().unwrap()).rgba,
        &RGBA
    );
}
pub(super) unsafe fn guards(app: &mut Application) {
    let link = app
        .workspace
        .links()
        .iter()
        .find(|link| link.to_port == "overlay")
        .unwrap()
        .clone();
    prepare(app);
    // 同像素新结果仍具有新的正式输出身份，不能启动旧准备所捕获的纹理。
    let source = app
        .documents
        .iter()
        .find(|doc| doc.state.unit_id == link.from_unit)
        .unwrap();
    commit(&source.state);
    app.pending_windows(false).unwrap();
    assert!(app.documents[0].state.pending_execution.borrow().is_none());
    assert!(app.execution_jobs.is_empty());
    assert!(app.executions.is_idle());
    prepare(app);
    assert!(app.workspace.remove_link(&link.id));
    app.pending_windows(false).unwrap();
    assert!(app.documents[0].state.pending_execution.borrow().is_none());
    assert!(app.execution_jobs.is_empty());
    assert!(app.executions.is_idle());
    app.workspace.connect(link).unwrap();
}
#[test]
fn shader_application_graph_texture_rejects_changed_output_and_topology_before_network() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap();
    let _input = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    let _apartment = Apartment::new().unwrap();
    unsafe {
        run(false, Backend::D3d11, false, TextureSource::Graph);
    }
}
