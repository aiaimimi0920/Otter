//! 仅 idle 请求预约 timer；latest-wins 请求继续借用原回收入口。
use super::*;
use std::time::{Duration, Instant};

fn state() -> State {
    let state = State::new(
        Some(crate::desktop::image::Image {
            width: 16,
            height: 16,
            pixels: vec![255; 16 * 16 * 4].into(),
        }),
        false,
    )
    .unwrap();
    state.canvas.borrow_mut().select_tool(Tool::Blur);
    state
}
fn request(state: &State, sigma: f32) -> Request {
    let canvas = state.canvas.borrow();
    Request {
        stamp: canvas.editor.document_stamp(),
        target: None,
        annotation_frame: canvas.editor.annotation_frame(),
        base_projection: canvas.editor.raster_base_projection(),
        source: canvas.editor.layers().unwrap().clone(),
        size: (16, 16),
        area: Region {
            x: 0,
            y: 0,
            width: 16,
            height: 16,
        },
        sigma,
        revision: canvas.editor.revision(),
    }
}
fn drain(state: &State) -> Vec<Completed> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut completed = Vec::new();
    while state.blur_job.borrow().is_busy() {
        if let Some(result) = state.blur_job.borrow_mut().poll() {
            completed.push(result);
        }
        assert!(Instant::now() < deadline, "blur worker timed out");
        std::thread::sleep(Duration::from_millis(1));
    }
    completed
}
#[test]
fn timer_failure_does_not_start_worker_or_replace_ready_preview() {
    let state = state();
    let source = state.canvas.borrow().editor.layers().unwrap().clone();
    let stamp = state.canvas.borrow().editor.document_stamp();
    state.canvas.borrow_mut().blur_ready = Some((
        source.clone(),
        BlurImage::new(16, 16, source.base.clone(), 8).unwrap(),
        None,
        None,
    ));
    let error = submit_with(
        &state,
        request(&state, 8.0),
        || {
            assert!(state.canvas.try_borrow_mut().is_ok());
            assert!(state.blur_job.try_borrow_mut().is_ok());
            assert!(!state.blur_job.borrow().is_busy());
            false
        },
        || panic!("failed reservation must not cancel"),
    )
    .unwrap_err();
    assert_eq!(error, "无法启动模糊任务定时器。");
    assert!(!state.blur_job.borrow().is_busy());
    let canvas = state.canvas.borrow();
    assert!(stamp.matches(&canvas.editor));
    assert!(canvas.editor.layers().unwrap().same_storage(&source));
    assert!(canvas
        .blur_ready
        .as_ref()
        .unwrap()
        .1
        .pixels()
        .same_storage(&source.base));
}
#[test]
fn busy_replacements_reuse_timer_and_only_deliver_latest_strength() {
    let state = state();
    let stamp = state.canvas.borrow().editor.document_stamp();
    submit_with(
        &state,
        request(&state, 8.0),
        || {
            assert!(!state.blur_job.borrow().is_busy());
            true
        },
        || panic!("successful start must keep timer"),
    )
    .unwrap();
    for sigma in [9.0, 10.0] {
        submit_with(
            &state,
            request(&state, sigma),
            || panic!("busy request must reuse timer"),
            || panic!("busy request must not cancel timer"),
        )
        .unwrap();
        assert_eq!(
            state.blur_job.borrow().current_request().unwrap().sigma,
            sigma
        );
    }
    state
        .canvas
        .borrow_mut()
        .tool_profiles
        .set_blur_strength(10.0);
    let mut results = drain(&state);
    assert_eq!(results.len(), 1);
    let ready = results.pop().unwrap();
    assert_eq!(ready.request.sigma, 10.0);
    assert!(accept(&mut state.canvas.borrow_mut(), ready).unwrap());
    assert!(state.blur_job.borrow_mut().poll().is_none());
    let mut canvas = state.canvas.borrow_mut();
    assert!(stamp.matches(&canvas.editor));
    assert_eq!(canvas.blur_ready.as_ref().unwrap().1.strength(), 10.0);
    assert!(!canvas.editor.undo());
}
#[test]
fn cancellation_discards_running_and_pending_without_delivery() {
    let state = state();
    submit_with(
        &state,
        request(&state, 8.0),
        || true,
        || panic!("unexpected cancel"),
    )
    .unwrap();
    submit_with(
        &state,
        request(&state, 9.0),
        || panic!("busy request must reuse timer"),
        || panic!("unexpected cancel"),
    )
    .unwrap();
    state.blur_job.borrow_mut().cancel();
    assert!(state.blur_job.borrow().is_busy());
    assert!(drain(&state).is_empty());
    assert!(state.canvas.borrow().blur_ready.is_none());
}
