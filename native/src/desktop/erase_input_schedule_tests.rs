//! 预约失败必须发生在线程启动前；真实 Job 保留单槽、预览及历史原子性。
use super::*;
use crate::{editor::Tool, viewport::Point};
use std::{
    cell::Cell,
    sync::Arc,
    time::{Duration, Instant},
};

fn state() -> State {
    let state = State::new(
        Some(crate::desktop::image::Image {
            width: 32,
            height: 32,
            pixels: vec![255; 32 * 32 * 4].into(),
        }),
        false,
    )
    .unwrap();
    {
        let mut canvas = state.canvas.borrow_mut();
        canvas.viewport = crate::viewport::Viewport {
            origin: Point::default(),
            scale: 1.0,
        };
        canvas.select_tool(Tool::Eraser);
        assert!(canvas.pointer_down(Point { x: 16.0, y: 16.0 }));
        assert!(canvas.pointer_up(Point { x: 16.0, y: 16.0 }));
    }
    state
}
fn finish(state: &State) -> crate::erase_job::Completed {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(completed) = state.erase_job.borrow_mut().poll() {
            return completed;
        }
        assert!(Instant::now() < deadline, "erase worker timed out");
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn timer_reservation_failure_does_not_start_worker_or_change_document() {
    let state = state();
    let stamp = state.canvas.borrow().editor.document_stamp();
    let source = state.canvas.borrow().editor.layers().unwrap().clone();
    let error = submit_with(
        &state,
        || {
            assert!(state.canvas.try_borrow_mut().is_ok());
            assert!(state.erase_job.try_borrow_mut().is_ok());
            assert!(!state.erase_job.borrow().is_busy());
            false
        },
        || panic!("failed reservation must not cancel another timer"),
    )
    .unwrap_err();
    assert_eq!(error, "Cannot schedule erase result");
    assert!(!state.erase_job.borrow().is_busy());
    let canvas = state.canvas.borrow();
    assert!(canvas.erase_pending.is_none());
    assert!(canvas.erase_display.is_none());
    assert!(stamp.matches(&canvas.editor));
    assert!(canvas.editor.layers().unwrap().same_storage(&source));
}
#[test]
fn rejected_worker_start_rolls_back_only_the_reserved_timer() {
    let state = state();
    state
        .canvas
        .borrow_mut()
        .erase_pending
        .as_mut()
        .unwrap()
        .width = f32::NAN;
    let stamp = state.canvas.borrow().editor.document_stamp();
    let cancelled = Cell::new(0);
    let error = submit_with(
        &state,
        || {
            assert!(!state.erase_job.borrow().is_busy());
            true
        },
        || {
            assert!(state.canvas.try_borrow_mut().is_ok());
            assert!(state.erase_job.try_borrow_mut().is_ok());
            cancelled.set(cancelled.get() + 1);
        },
    )
    .unwrap_err();
    assert_eq!(error, "InvalidInput");
    assert_eq!(cancelled.get(), 1);
    assert!(!state.erase_job.borrow().is_busy());
    let canvas = state.canvas.borrow();
    assert!(canvas.erase_pending.is_none());
    assert!(canvas.erase_display.is_none());
    assert!(stamp.matches(&canvas.editor));
}
#[test]
fn busy_submission_preserves_original_timer_preview_and_result() {
    let state = state();
    submit_with(&state, || true, || panic!("unexpected cancel")).unwrap();
    // WM_LBUTTONUP 无新请求时仍会调用 submit，不能因原任务忙而弹框。
    submit_with(
        &state,
        || panic!("empty submission must not reserve"),
        || panic!("empty submission must not cancel"),
    )
    .unwrap();
    let original = state.canvas.borrow().erase_display.clone().unwrap();
    let mut pending = original.clone();
    pending.preview_identity = Arc::default();
    state.canvas.borrow_mut().erase_pending = Some(pending);
    let error = submit_with(
        &state,
        || panic!("busy job must keep original timer"),
        || panic!("busy job must not cancel original timer"),
    )
    .unwrap_err();
    assert_eq!(error, "Busy");
    assert!(state.erase_job.borrow().is_busy());
    assert!(Arc::ptr_eq(
        &state
            .canvas
            .borrow()
            .erase_display
            .as_ref()
            .unwrap()
            .preview_identity,
        &original.preview_identity,
    ));
    assert!(state.canvas.borrow().erase_pending.is_none());
    let completed = finish(&state);
    assert!(accept(&mut state.canvas.borrow_mut(), completed).unwrap());
}
#[test]
fn timer_is_reserved_before_start_and_completion_commits_once_with_undo() {
    let state = state();
    let source = state.canvas.borrow().editor.layers().unwrap().clone();
    let reserved = Cell::new(0);
    submit_with(
        &state,
        || {
            assert!(!state.erase_job.borrow().is_busy());
            reserved.set(reserved.get() + 1);
            true
        },
        || panic!("successful start must keep timer"),
    )
    .unwrap();
    assert_eq!(reserved.get(), 1);
    assert!(state.erase_job.borrow().is_busy());
    let completed = finish(&state);
    assert!(state.erase_job.borrow_mut().poll().is_none());
    let mut canvas = state.canvas.borrow_mut();
    assert!(accept(&mut canvas, completed).unwrap());
    assert_eq!(canvas.editor.revision(), 1);
    assert!(canvas.erase_display.is_none());
    assert!(canvas.editor.undo());
    assert!(canvas.editor.layers().unwrap().same_storage(&source));
    assert!(!canvas.editor.undo());
}
