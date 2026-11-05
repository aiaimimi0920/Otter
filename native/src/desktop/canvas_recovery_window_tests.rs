//! 生产 WM_PAINT/WM_TIMER 的恢复事务；隐藏自有窗口，不注入系统输入或驱动故障。
use super::*;
use crate::desktop::composition_retirement::Status;
use std::time::{Duration, Instant};
use windows::{
    core::Error,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

const TIMER: usize = 0x4F63;

unsafe fn no_close(hwnd: HWND) {
    let mut message = MSG::default();
    assert!(!PeekMessageW(&mut message, Some(hwnd), WM_CLOSE, WM_CLOSE, PM_NOREMOVE).as_bool());
    assert!(IsWindow(Some(hwnd)).as_bool());
}

#[test]
fn window_recovers_real_d3d12_failure_after_retirement_and_ignores_stale_ticks() {
    software_tests::fixture(|state, hwnd| unsafe {
        state.canvas.borrow_mut().software_renderer = false;
        SendMessageW(hwnd, WM_PAINT, None, None);
        let mut canvas = state.canvas.borrow_mut();
        assert!(matches!(
            canvas.target.as_ref().unwrap().surface,
            RenderSurface::D3d12(_)
        ));
        assert!(!canvas.retry_render_without_composition());
        let stamp = canvas.editor.document_stamp();
        let paints = canvas.confirmed_paints;
        assert!(canvas
            .target
            .as_mut()
            .unwrap()
            .surface
            .draw(|target| {
                target.PopAxisAlignedClip();
                Ok(())
            })
            .is_err());
        drop(canvas);
        SendMessageW(hwnd, WM_PAINT, None, None);
        let original = state.error.borrow().as_ref().unwrap().code();
        assert!(state.canvas.borrow().render_suspended());
        assert!(state.canvas.borrow().recovery.target.is_some());
        // 正常最小化/discard 和额外 paint 不能丢退休 owner 或提前确认帧。
        SendMessageW(hwnd, WM_SIZE, Some(WPARAM(SIZE_MINIMIZED as usize)), None);
        SendMessageW(hwnd, WM_PAINT, None, None);
        assert!(state.canvas.borrow().recovery.target.is_some());
        assert_eq!(state.canvas.borrow().confirmed_paints, paints);
        assert_eq!(state.error.borrow().as_ref().unwrap().code(), original);
        assert!(stamp.matches(&state.canvas.borrow().editor));
        state
            .canvas
            .borrow_mut()
            .replace_image(Image {
                width: 2,
                height: 2,
                pixels: [90, 60, 30, 255].repeat(4).into(),
            })
            .unwrap();
        let stamp = state.canvas.borrow().editor.document_stamp();
        assert!(state.canvas.borrow().recovery.target.is_some());
        assert!(state.canvas.borrow().target.is_none());
        let deadline = Instant::now() + Duration::from_secs(3);
        while state.canvas.borrow().render_suspended() {
            assert!(Instant::now() < deadline, "recovery did not complete");
            let mut message = MSG::default();
            while PeekMessageW(&mut message, Some(hwnd), WM_TIMER, WM_TIMER, PM_REMOVE).as_bool() {
                DispatchMessageW(&message);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(state.canvas.borrow().compatibility_renderer);
        assert!(state.canvas.borrow().recovery.target.is_none());
        // 完成通知本身不清错误；只有新后端成功呈现才确认新帧。
        assert_eq!(state.error.borrow().as_ref().unwrap().code(), original);
        SendMessageW(hwnd, WM_PAINT, None, None);
        assert!(state.error.borrow().is_none());
        assert_eq!(state.canvas.borrow().confirmed_paints, paints + 1);
        assert!(stamp.matches(&state.canvas.borrow().editor));
        SendMessageW(hwnd, WM_TIMER, Some(WPARAM(TIMER)), None);
        assert_eq!(state.canvas.borrow().confirmed_paints, paints + 1);
        assert!(!state.canvas.borrow_mut().recover_render());
        no_close(hwnd);
    });
}

#[test]
fn terminal_window_keeps_error_document_and_counts_across_paints() {
    for status in [
        Status::TimedOut,
        Status::Failed {
            stage: crate::desktop::composition_retirement::Stage::Completion,
            code: E_FAIL,
        },
    ] {
        software_tests::fixture(|state, hwnd| unsafe {
            let stamp = state.canvas.borrow().editor.document_stamp();
            state.canvas.borrow_mut().recovery.status = Some(status);
            *state.error.borrow_mut() = Some(Error::new(E_FAIL, "retirement fixture"));
            for _ in 0..3 {
                SendMessageW(hwnd, WM_PAINT, None, None);
                SendMessageW(hwnd, WM_TIMER, Some(WPARAM(TIMER)), None);
                no_close(hwnd);
            }
            let canvas = state.canvas.borrow();
            assert!(canvas.target.is_none());
            assert_eq!(canvas.confirmed_paints, 0);
            assert!(stamp.matches(&canvas.editor));
            assert_eq!(state.error.borrow().as_ref().unwrap().code(), E_FAIL);
        });
    }
}

#[test]
fn nested_tick_defers_and_destroy_cancels_pending_timer_without_rebinding() {
    software_tests::fixture(|state, hwnd| unsafe {
        state.canvas.borrow_mut().recovery.status = Some(Status::Pending);
        assert_ne!(SetTimer(Some(hwnd), TIMER, 16, None), 0);
        state.render_active.set(true);
        SendMessageW(hwnd, WM_TIMER, Some(WPARAM(TIMER)), None);
        assert_eq!(state.canvas.borrow().recovery.status, Some(Status::Pending));
        state.render_active.set(false);
        DestroyWindow(hwnd).unwrap();
        assert!(!state.lifetime.owns(hwnd));
        assert!(state.canvas.borrow().render_suspended());
        assert!(state.canvas.borrow().recovery.target.is_none());
        assert_eq!(state.canvas.borrow().confirmed_paints, 0);
    });
}

#[test]
fn nonrecoverable_paint_error_preserves_window_instead_of_posting_close() {
    software_tests::fixture(|state, hwnd| unsafe {
        state.canvas.borrow_mut().window_opacity = f32::NAN;
        let stamp = state.canvas.borrow().editor.document_stamp();
        SendMessageW(hwnd, WM_PAINT, None, None);
        assert!(state.error.borrow().is_some());
        assert!(state.canvas.borrow().render_suspended());
        SendMessageW(hwnd, WM_PAINT, None, None);
        no_close(hwnd);
        assert!(stamp.matches(&state.canvas.borrow().editor));
        assert_eq!(state.canvas.borrow().confirmed_paints, 0);
    });
}

#[test]
fn software_target_retains_safe_rebuild_without_composition_retirement() {
    software_tests::fixture(|state, hwnd| unsafe {
        SendMessageW(hwnd, WM_PAINT, None, None);
        let stamp = state.canvas.borrow().editor.document_stamp();
        assert!(state.canvas.borrow_mut().retry_render_without_composition());
        assert!(!state.canvas.borrow().render_suspended());
        assert!(state.canvas.borrow().target.is_none());
        SendMessageW(hwnd, WM_PAINT, None, None);
        assert!(state.error.borrow().is_none());
        assert!(stamp.matches(&state.canvas.borrow().editor));
        assert_eq!(state.canvas.borrow().confirmed_paints, 2);
    });
}
