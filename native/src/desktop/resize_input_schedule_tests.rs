//! Both asynchronous entries share reservation; an actual rejected start must revoke the reservation and clear this preview.
use super::*;
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
        let pixels = canvas.image.as_ref().unwrap().pixels.clone();
        canvas
            .editor
            .begin_blur(
                Point { x: 4.0, y: 8.0 },
                crate::blur_image::BlurImage::new(16, 16, pixels, 4).unwrap(),
            )
            .unwrap();
        assert!(canvas.editor.commit());
        canvas.select_tool(Tool::Select);
        canvas.selected = Some((0, canvas.editor.revision()));
        canvas.scale_display_revision = Some(canvas.editor.revision());
        canvas.group_preview = vec![(0, canvas.editor.elements()[0].clone())];
    }
    state
}
fn plan(state: &State, factor: f32) -> ResizePlan {
    state
        .canvas
        .borrow()
        .editor
        .prepare_resize_many(
            &[0],
            Point { x: 8.0, y: 8.0 },
            Point {
                x: factor,
                y: factor,
            },
            |_| unreachable!(),
        )
        .unwrap()
        .unwrap()
}
fn finish(state: &State) -> ResizePlan {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(result) = state.resize_job.borrow_mut().poll() {
            return result.unwrap();
        }
        assert!(Instant::now() < deadline, "resize worker timed out");
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn failed_reservation_clears_only_temporary_preview_without_starting_worker() {
    let state = state();
    let stamp = state.canvas.borrow().editor.document_stamp();
    let source = state.canvas.borrow().editor.layers().unwrap().clone();
    let error = schedule_with(
        &state,
        plan(&state, 2.0),
        || {
            assert!(state.canvas.try_borrow_mut().is_ok());
            assert!(state.resize_job.try_borrow_mut().is_ok());
            assert!(!state.resize_job.borrow().is_busy());
            false
        },
        || panic!("failed reservation must not cancel"),
    )
    .unwrap_err();
    assert_eq!(error, "Cannot schedule resize result");
    assert!(!state.resize_job.borrow().is_busy());
    let canvas = state.canvas.borrow();
    assert!(stamp.matches(&canvas.editor));
    assert!(canvas.editor.layers().unwrap().same_storage(&source));
    assert!(canvas.scale_display_revision.is_none());
    assert!(canvas.group_preview.is_empty());
    assert_eq!(canvas.selected_index(), Some(0));
}
#[test]
fn rejected_real_start_cancels_only_new_timer_and_clears_preview() {
    let state = state();
    let stamp = state.canvas.borrow().editor.document_stamp();
    let cancelled = Cell::new(0);
    let error = schedule_with(
        &state,
        plan(&state, 20.0),
        || {
            assert!(!state.resize_job.borrow().is_busy());
            true
        },
        || {
            assert!(state.canvas.try_borrow_mut().is_ok());
            assert!(state.resize_job.try_borrow_mut().is_ok());
            cancelled.set(cancelled.get() + 1);
        },
    )
    .unwrap_err();
    assert_eq!(error, "InvalidInput");
    assert_eq!(cancelled.get(), 1);
    assert!(!state.resize_job.borrow().is_busy());
    let canvas = state.canvas.borrow();
    assert!(stamp.matches(&canvas.editor));
    assert!(canvas.scale_display_revision.is_none());
    assert!(canvas.group_preview.is_empty());
}
#[test]
fn busy_keeps_original_timer_and_preview_then_commits_once_with_undo() {
    let state = state();
    let revision = state.canvas.borrow().editor.revision();
    let source = state.canvas.borrow().editor.layers().unwrap().clone();
    schedule_with(
        &state,
        plan(&state, 2.0),
        || {
            assert!(!state.resize_job.borrow().is_busy());
            true
        },
        || panic!("successful start must keep timer"),
    )
    .unwrap();
    assert!(state.resize_job.borrow().is_busy());
    assert_eq!(
        schedule_with(
            &state,
            plan(&state, 3.0),
            || panic!("busy must not reserve"),
            || panic!("busy must not cancel"),
        )
        .unwrap_err(),
        "Busy"
    );
    assert_eq!(state.canvas.borrow().scale_display_revision, Some(revision));
    assert_eq!(state.canvas.borrow().group_preview.len(), 1);
    let ready = finish(&state);
    assert!(state.resize_job.borrow_mut().poll().is_none());
    let mut canvas = state.canvas.borrow_mut();
    canvas.clear_scale_display();
    assert!(accept(&mut canvas, ready).unwrap());
    assert_eq!(canvas.editor.revision(), revision + 1);
    assert_eq!(
        canvas.editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .strength(),
        8.0
    );
    assert!(canvas.editor.undo());
    assert_eq!(
        canvas.editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .strength(),
        4.0
    );
    assert!(canvas.editor.layers().unwrap().same_storage(&source));
}
