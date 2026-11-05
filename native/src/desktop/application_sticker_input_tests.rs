//! 活动Application正式输入/通知/文件取消；真实像素经过原输入worker，不以Graph模型代接线。
use super::*;
use crate::desktop::{
    self,
    node_menu::art_picker::{model::Snapshot, tests as fixture},
    Apartment,
};
use std::{
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};
unsafe fn app() -> Application {
    let (instance, doc) = fixture::document();
    let mut app = Application::new(instance, doc);
    app.catalog.borrow_mut().loaded = Some(fixture::catalog(|_| {}));
    let request = Snapshot::capture(app.documents[0].window.0, &app.documents[0].state)
        .request(0, &app.documents[0].state)
        .unwrap();
    *app.documents[0].state.art_request.borrow_mut() = Some(request);
    app.process_art_requests(false).unwrap();
    app
}
fn prepare(app: &Application) -> desktop::loom_input_job::PreparedBatch {
    let target = &app.documents[1].state;
    let inputs = Application::image_inputs(
        &app.documents,
        &app.live_sessions,
        &app.workspace,
        &target.unit_id,
        true,
        None,
        None,
    )
    .unwrap()
    .unwrap();
    assert_eq!(inputs.ports, ["pixels"]);
    let mut job = desktop::loom_input_job::Job::default();
    job.start(
        target.unit_id.clone(),
        &target.canvas.borrow().editor,
        inputs.sources,
    )
    .unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(result) = job.poll() {
            return result.unwrap();
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn pixels(batch: &desktop::loom_input_job::PreparedBatch) -> crate::pixels::Pixels {
    let png = batch.pngs().next().unwrap();
    desktop::image::Image::from_encoded(&png).unwrap().pixels
}
#[test]
fn active_graph_prefers_upstream_then_manual_and_disabled_or_relay_off_returns_original() {
    let _serial = desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let mut app = app();
        let manual = desktop::sticker_input::tests::asset();
        let original = app.documents[0]
            .state
            .canvas
            .borrow()
            .image
            .as_ref()
            .unwrap()
            .pixels
            .clone();
        app.documents[0]
            .state
            .sticker_input
            .set_asset(Some(manual.clone()))
            .unwrap();
        let ready = prepare(&app);
        assert_eq!(pixels(&ready), manual.image.pixels);
        let freshness = ready.freshness().unwrap();
        assert!(freshness.is_current(|id, stamp| app
            .documents
            .iter()
            .any(|d| d.state.unit_id == *id && stamp.matches_state(&d.state))));
        let upstream = desktop::document_window::DocumentWindow::create(
            app.instance,
            desktop::State::new(
                Some(desktop::image::Image {
                    width: 2,
                    height: 2,
                    pixels: [255, 0, 0, 255].repeat(4).into(),
                }),
                false,
            )
            .unwrap(),
            false,
        )
        .unwrap();
        let upstream_id = upstream.state.unit_id.clone();
        let blue = upstream
            .state
            .canvas
            .borrow()
            .image
            .as_ref()
            .unwrap()
            .pixels
            .clone();
        app.workspace.add_units([upstream_id.clone()]).unwrap();
        app.documents.push(upstream);
        let link = crate::workspace::Link {
            id: desktop::document_state::new_unit_id().unwrap(),
            from_unit: upstream_id,
            from_port: "output_image".into(),
            to_unit: app.documents[0].state.unit_id.clone(),
            to_port: "image".into(),
        };
        app.workspace.connect(link.clone()).unwrap();
        assert_eq!(pixels(&prepare(&app)), blue);
        app.documents[0].state.sticker_input.toggle().unwrap();
        assert_eq!(pixels(&prepare(&app)), original);
        assert!(app.workspace.links().iter().any(|edge| edge.id == link.id));
        app.documents[0].state.sticker_input.toggle().unwrap();
        app.workspace.remove_link(&link.id);
        assert_eq!(pixels(&prepare(&app)), manual.image.pixels);
        app.documents[0].state.accept_upstream.set(false);
        assert_eq!(pixels(&prepare(&app)), original);
        app.documents[0].state.accept_upstream.set(true);
        // 本地编辑停止中继，但不清除手动输入；取消草稿后仍回到原手动图。
        assert!(app.documents[0]
            .state
            .canvas
            .borrow_mut()
            .editor
            .begin(
                crate::editor::Tool::Brush,
                crate::viewport::Point { x: 0.0, y: 0.0 }
            )
            .unwrap());
        assert!(
            Application::image_inputs(
                &app.documents,
                &app.live_sessions,
                &app.workspace,
                &app.documents[1].state.unit_id,
                true,
                None,
                None,
            )
            .is_err(),
            "未完成草稿仍应阻止执行捕获"
        );
        assert!(app.documents[0]
            .state
            .sticker_input
            .snapshot()
            .asset
            .is_some());
        assert!(app.documents[0].state.canvas.borrow_mut().editor.cancel());
        assert_eq!(pixels(&prepare(&app)), manual.image.pixels);
        {
            let mut canvas = app.documents[0].state.canvas.borrow_mut();
            // 画布外已提交标注使本地编辑生效但不改变2×2像素，能区分自身原图和红色手动图。
            assert!(canvas
                .editor
                .begin(
                    crate::editor::Tool::Brush,
                    crate::viewport::Point { x: 100.0, y: 100.0 }
                )
                .unwrap());
            assert!(canvas.editor.commit());
        }
        assert_eq!(pixels(&prepare(&app)), original);
        app.documents[0]
            .state
            .sticker_input
            .set_asset(None)
            .unwrap();
        assert!(!freshness.is_current(|id, stamp| app
            .documents
            .iter()
            .any(|d| d.state.unit_id == *id && stamp.matches_state(&d.state))));
        assert_eq!(pixels(&prepare(&app)), original);
        app.shutdown();
    }
}

#[test]
fn manual_configuration_changes_invalidate_save_receipts() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let mut app = app();
        app.documents[1].state.pending_start.borrow_mut().take();
        for change in 0..3 {
            let (receipt, _) = app.capture_session_snapshot().unwrap();
            assert!(receipt.matches(&app));
            let control = &app.documents[0].state.sticker_input;
            match change {
                0 => control
                    .set_asset(Some(desktop::sticker_input::tests::asset()))
                    .unwrap(),
                1 => control.toggle().unwrap(),
                _ => control.set_asset(None).unwrap(),
            }
            assert!(!receipt.matches(&app));
        }
        app.shutdown();
    }
}
#[test]
fn manual_upload_notifies_downstream_and_clear_rejects_inflight_file_result() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let mut app = app();
        let manual = desktop::sticker_input::tests::asset();
        app.documents[1].state.pending_start.borrow_mut().take();
        app.documents[0]
            .state
            .sticker_input
            .set_asset(Some(manual.clone()))
            .unwrap();
        app.pending_parameter_files(false).unwrap();
        assert!(app.documents[1].state.pending_start.borrow().is_some());
        let path = std::path::PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi").join(
            format!(
                "otter-sticker-worker-{}.png",
                desktop::document_state::new_unit_id().unwrap().as_str()
            ),
        );
        let bytes = desktop::export::png_raw(&manual.image, &AtomicBool::new(false)).unwrap();
        std::fs::write(&path, &*bytes).unwrap();
        let state = &app.documents[0].state;
        let request = desktop::parameter_file_input::Request {
            path: path.clone(),
            target: desktop::parameter_file_input::Target::Sticker,
            stamp: desktop::configuration_stamp::Stamp::capture(state),
            view: None,
        };
        assert!(
            desktop::parameter_file_input::queue(app.documents[0].window.0, state, request)
                .unwrap()
        );
        app.pending_parameter_files(false).unwrap();
        assert!(app.parameter_file_job.is_some());
        app.documents[0]
            .state
            .sticker_input
            .set_asset(None)
            .unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        while app.parameter_file_job.is_some() {
            assert!(Instant::now() < until);
            app.pending_parameter_files(false).unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(app.documents[0]
            .state
            .sticker_input
            .snapshot()
            .asset
            .is_none());
        std::fs::remove_file(path).unwrap();
        app.shutdown();
    }
}
