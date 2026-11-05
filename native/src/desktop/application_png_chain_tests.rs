//! 复用真实隐藏应用/中继 fixture，验证新正式 PNG 的显示、传播准入和下游编码；不请求用户服务。
use super::*;
use crate::desktop::{
    loom_input_job::Job,
    loom_result_state::Results,
    shader_image::{self, tests as oracle},
};
use base64::Engine;
use std::sync::atomic::AtomicBool;

pub(super) fn verify(app: &mut Application) {
    let cancel = AtomicBool::new(false);
    let output = shader_image::prepare(oracle::matrix(), &cancel).unwrap();
    let png = output.png.clone();
    let pixels = output.image.pixels.clone();
    let source = app.documents[0].state.unit_id.clone();
    let target = app.documents[1].state.unit_id.clone();
    let request = crate::graph_requests::Request {
        id: crate::desktop::document_state::new_unit_id().unwrap(),
        generation: 1,
    };
    let mut results = Results::new(source.clone());
    results.begin(request.clone());
    results
        .finish(&request, output.into_batch().unwrap())
        .unwrap();
    let original = std::mem::replace(
        &mut *app.documents[0].state.execution_results.borrow_mut(),
        results,
    );
    app.sync_display_frames();
    for index in [0, 2] {
        let display = app.documents[index].state.resolved_display.borrow();
        assert!(display.as_ref().unwrap().image.pixels.same_storage(&pixels));
    }
    let inputs = Application::image_inputs(
        &app.documents,
        &app.live_sessions,
        &app.workspace,
        &target,
        true,
        None,
        None,
    )
    .unwrap()
    .unwrap();
    assert_eq!(inputs.ports, ["input_image"]);
    let mut job = Job::default();
    job.start(
        target.clone(),
        &app.documents[1].state.canvas.borrow().editor,
        inputs.sources,
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let batch = loop {
        if let Some(result) = job.poll() {
            break result.unwrap();
        }
        assert!(
            Instant::now() < deadline,
            "formal PNG graph preparation timeout"
        );
        std::thread::sleep(Duration::from_millis(1));
    };
    let (
        _,
        crate::loom_art_wire::Input::InlinePng {
            base64,
            width,
            height,
        },
    ) = batch.descriptors().next().unwrap()
    else {
        panic!("PNG required")
    };
    assert_eq!((width, height), (256, 256));
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(base64)
            .unwrap()
            .as_slice(),
        png.as_ref()
    );
    unsafe {
        app.propagate_result(&source);
    }
    assert!(app.documents[1].state.pending_start.borrow().is_some());
    // 不启动第二次远端执行；恢复共享 fixture，旧测试继续验证原有行为。
    for document in &app.documents {
        document.state.pending_start.borrow_mut().take();
    }
    *app.documents[0].state.execution_results.borrow_mut() = original;
    app.sync_display_frames();
    eprintln!("native-formal-png-graph: hidden-hwnd=true display=true relay=true propagation=true original-png=true no-network=true");
}
