//! Scheduling failures start no computation; preserve existing single slots, cancellation, and one-shot history commits.
use super::*;
use crate::{blur::Region, blur_job::Request, editor::FlipPlan};
use std::time::{Duration, Instant};

fn state() -> State {
    State::new(
        Some(crate::desktop::image::Image {
            width: 2,
            height: 1,
            pixels: vec![10, 20, 30, 255, 40, 50, 60, 255].into(),
        }),
        false,
    )
    .unwrap()
}
fn plan(state: &State) -> FlipPlan {
    state
        .canvas
        .borrow()
        .editor
        .prepare_flip(Axis::Horizontal, |_| unreachable!())
        .unwrap()
        .unwrap()
}
fn finish(state: &State) -> Result<RasterRefreshPlan, Error> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(result) = state.flip_job.borrow_mut().poll() {
            return result;
        }
        assert!(Instant::now() < deadline, "flip worker timed out");
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn timer_failure_preserves_document_and_does_not_cancel_existing_blur() {
    let state = state();
    let source = state.canvas.borrow().editor.layers().unwrap().clone();
    let stamp = state.canvas.borrow().editor.document_stamp();
    let request = Request {
        stamp: stamp.clone(),
        target: None,
        annotation_frame: None,
        base_projection: None,
        source: source.clone(),
        size: (2, 1),
        area: Region {
            x: 0,
            y: 0,
            width: 2,
            height: 1,
        },
        sigma: 2.0,
        revision: 0,
    };
    state.blur_job.borrow_mut().request(request).unwrap();
    let error = submit_with(
        &state,
        plan(&state),
        || {
            assert!(state.canvas.try_borrow_mut().is_ok());
            assert!(state.flip_job.try_borrow_mut().is_ok());
            assert!(!state.flip_job.borrow().is_busy());
            false
        },
        || panic!("failed reservation must not cancel a timer"),
    )
    .unwrap_err();
    assert_eq!(error, "Cannot schedule flip result");
    assert!(!state.flip_job.borrow().is_busy());
    assert!(stamp.matches(&state.canvas.borrow().editor));
    assert!(state
        .canvas
        .borrow()
        .editor
        .layers()
        .unwrap()
        .same_storage(&source));
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(completed) = state.blur_job.borrow_mut().poll() {
            assert!(completed.result.is_ok());
            break;
        }
        assert!(state.blur_job.borrow().is_busy(), "blur was cancelled");
        assert!(Instant::now() < deadline, "blur worker timed out");
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn reservation_precedes_worker_and_busy_keeps_original_result_and_timer() {
    let state = state();
    let source = state.canvas.borrow().editor.layers().unwrap().clone();
    submit_with(
        &state,
        plan(&state),
        || {
            assert!(!state.flip_job.borrow().is_busy());
            true
        },
        || panic!("successful start must keep timer"),
    )
    .unwrap();
    assert!(state.flip_job.borrow().is_busy());
    assert_eq!(
        submit_with(
            &state,
            plan(&state),
            || panic!("busy must not reserve"),
            || panic!("busy must not cancel"),
        )
        .unwrap_err(),
        "Busy"
    );
    let ready = finish(&state).unwrap();
    assert!(state.flip_job.borrow_mut().poll().is_none());
    let mut canvas = state.canvas.borrow_mut();
    assert!(accept(&mut canvas, ready).unwrap());
    assert_eq!(
        &canvas.image.as_ref().unwrap().pixels[..],
        &[40, 50, 60, 255, 10, 20, 30, 255]
    );
    assert_eq!(canvas.editor.revision(), 1);
    assert!(canvas.editor.undo());
    assert!(canvas.editor.layers().unwrap().same_storage(&source));
    assert!(!canvas.editor.undo());
}
#[test]
fn cancellation_keeps_worker_owned_until_poll_without_publishing() {
    let state = state();
    let stamp = state.canvas.borrow().editor.document_stamp();
    submit_with(
        &state,
        plan(&state),
        || true,
        || panic!("unexpected cancel"),
    )
    .unwrap();
    state.flip_job.borrow().cancel();
    assert!(state.flip_job.borrow().is_busy());
    assert!(matches!(finish(&state), Err(Error::Cancelled)));
    assert!(!state.flip_job.borrow().is_busy());
    assert!(stamp.matches(&state.canvas.borrow().editor));
}
