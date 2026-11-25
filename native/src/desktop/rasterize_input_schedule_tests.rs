//! WIC/D2D threads start only after successful reservation; rejected starts leave no timer.
use super::*;
use crate::{editor::Tool, viewport::Point};
use std::{
    cell::Cell,
    time::{Duration, Instant},
};

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
    {
        let mut canvas = state.canvas.borrow_mut();
        canvas
            .editor
            .begin(Tool::Brush, Point { x: 8.0, y: 8.0 })
            .unwrap();
        assert!(canvas.editor.commit());
    }
    state
}
fn finish(state: &State) -> (Request, Result<RasterRefreshPlan, String>) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let ready = state
            .rasterize_job
            .borrow()
            .as_ref()
            .unwrap()
            .worker
            .as_ref()
            .unwrap()
            .is_finished();
        if ready {
            let mut job = state.rasterize_job.borrow_mut().take().unwrap();
            let result = job.worker.take().unwrap().join().unwrap();
            // Job implements Drop; after claiming refresh, copy only the request identity needed by accept.
            let request = Request {
                source: job.request.source.clone(),
                revision: job.request.revision,
                indices: job.request.indices.clone(),
                refresh: None,
            };
            return (request, result);
        }
        assert!(Instant::now() < deadline, "rasterize worker timed out");
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn timer_failure_preserves_document_without_starting_wic_worker() {
    let state = state();
    let stamp = state.canvas.borrow().editor.document_stamp();
    let source = state.canvas.borrow().editor.layers().unwrap().clone();
    let (request, snapshot) = prepare(&state.canvas.borrow(), false).unwrap();
    let error = submit_with(
        &state,
        request,
        snapshot,
        || {
            assert!(state.canvas.try_borrow_mut().is_ok());
            assert!(state.rasterize_job.try_borrow_mut().is_ok());
            assert!(state.rasterize_job.borrow().is_none());
            false
        },
        || panic!("failed reservation must not cancel"),
    )
    .unwrap_err();
    assert_eq!(error, "Cannot schedule rasterization result");
    assert!(state.rasterize_job.borrow().is_none());
    let canvas = state.canvas.borrow();
    assert!(stamp.matches(&canvas.editor));
    assert!(canvas.editor.layers().unwrap().same_storage(&source));
    assert_eq!(canvas.editor.elements().len(), 1);
}
#[test]
fn rejected_real_spawn_cancels_reserved_timer_once() {
    let state = state();
    let stamp = state.canvas.borrow().editor.document_stamp();
    let (mut request, snapshot) = prepare(&state.canvas.borrow(), false).unwrap();
    request.refresh = None;
    let cancelled = Cell::new(0);
    let error = submit_with(
        &state,
        request,
        snapshot,
        || {
            assert!(state.rasterize_job.borrow().is_none());
            true
        },
        || {
            assert!(state.canvas.try_borrow_mut().is_ok());
            assert!(state.rasterize_job.try_borrow_mut().is_ok());
            cancelled.set(cancelled.get() + 1);
        },
    )
    .unwrap_err();
    assert_eq!(error, "Missing raster refresh plan");
    assert_eq!(cancelled.get(), 1);
    assert!(state.rasterize_job.borrow().is_none());
    assert!(stamp.matches(&state.canvas.borrow().editor));
}
#[test]
fn busy_keeps_original_worker_and_timer_then_commits_once_with_undo() {
    let state = state();
    let source = state.canvas.borrow().editor.layers().unwrap().clone();
    let revision = state.canvas.borrow().editor.revision();
    let (request, snapshot) = prepare(&state.canvas.borrow(), false).unwrap();
    submit_with(
        &state,
        request,
        snapshot,
        || {
            assert!(state.rasterize_job.borrow().is_none());
            true
        },
        || panic!("successful start must keep timer"),
    )
    .unwrap();
    assert!(state.rasterize_job.borrow().is_some());
    let (request, snapshot) = prepare(&state.canvas.borrow(), false).unwrap();
    assert_eq!(
        submit_with(
            &state,
            request,
            snapshot,
            || panic!("busy must not reserve"),
            || panic!("busy must not cancel"),
        )
        .unwrap_err(),
        "Busy"
    );
    let (request, result) = finish(&state);
    assert!(state.rasterize_job.borrow().is_none());
    let mut canvas = state.canvas.borrow_mut();
    assert!(accept(&mut canvas, &request, result).unwrap());
    assert_eq!(canvas.editor.revision(), revision + 1);
    assert!(canvas.editor.elements().is_empty());
    assert!(canvas.editor.layers().unwrap().annotations.is_some());
    assert!(canvas.editor.undo());
    assert_eq!(canvas.editor.elements().len(), 1);
    assert!(canvas.editor.layers().unwrap().same_storage(&source));
}
