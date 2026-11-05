//! 完成请求必须先收下唯一在途帧，且不能再启动自动滚动或重复拼接。
use super::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc, Arc,
};
use std::time::Duration;
use tests::{hidden, input, settle};

fn frame(axis: Axis, origin: u8) -> crate::long_capture::Frame {
    let (width, height) = match axis {
        Axis::Vertical => (4, 16),
        Axis::Horizontal => (16, 4),
    };
    let mut pixels = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let position = if axis == Axis::Vertical { y } else { x };
            pixels.extend_from_slice(&[origin + position as u8; 4]);
        }
    }
    crate::long_capture::Frame::new(width, height, pixels.into()).unwrap()
}

#[test]
fn finish_during_pending_capture_drains_once_on_both_axes_without_another_scroll() {
    unsafe {
        for axis in [Axis::Vertical, Axis::Horizontal] {
            for automatic in [false, true] {
                let mut input = input();
                input.axis = axis;
                let session = crate::long_capture::Session::new(
                    axis,
                    frame(axis, 0),
                    MatchOptions {
                        minimum_overlap: 8,
                        maximum_shift: 8,
                        maximum_mean_error: 3.0,
                    },
                )
                .unwrap();
                let count = Arc::new(AtomicUsize::new(0));
                let seen = count.clone();
                let (release, wait) = mpsc::channel();
                *input.job.borrow_mut() = SessionJob::start_with_actions(session, move |_, _| {
                    if seen.fetch_add(1, Ordering::AcqRel) == 0 {
                        wait.recv_timeout(Duration::from_secs(5)).unwrap();
                    }
                    Ok(frame(axis, 4))
                })
                .unwrap();
                let window = hidden(&input);
                handle(
                    window.0,
                    WM_COMMAND,
                    WPARAM(if automatic { 106 } else { 105 }),
                    LPARAM(0),
                );
                assert!(input.job.borrow().is_pending());
                assert!(
                    windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(
                        GetDlgItem(Some(window.0), 1).unwrap()
                    )
                    .as_bool()
                );
                handle(window.0, WM_COMMAND, WPARAM(1), LPARAM(0));
                handle(window.0, WM_COMMAND, WPARAM(1), LPARAM(0));
                assert!(
                    !windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(
                        GetDlgItem(Some(window.0), 1).unwrap()
                    )
                    .as_bool()
                );
                for command in [101, 102, 105, 106, 107, 108, 109, 110, 111] {
                    handle(window.0, WM_COMMAND, WPARAM(command), LPARAM(0));
                }
                assert!(input.forward.get());
                release.send(()).unwrap();
                settle(window.0, &input);
                assert_eq!(count.load(Ordering::Acquire), 1);
                assert!(!input.automatic.borrow().running());
                assert_eq!(
                    input.output.borrow().as_ref().map(|p| (p.width, p.height)),
                    Some(if axis == Axis::Vertical {
                        (4, 20)
                    } else {
                        (20, 4)
                    })
                );
            }
        }
    }
}

#[test]
fn pending_preview_and_spatial_append_finish_through_the_existing_worker() {
    unsafe {
        for spatial in [false, true] {
            let input = input();
            let window = hidden(&input);
            if spatial {
                input
                    .job
                    .borrow_mut()
                    .append_at(frame(Axis::Vertical, 4), (1, 4))
                    .unwrap();
            } else {
                handle(window.0, WM_COMMAND, WPARAM(102), LPARAM(0));
            }
            handle(window.0, WM_COMMAND, WPARAM(1), LPARAM(0));
            settle(window.0, &input);
            assert!(input.output.borrow().is_some());
            assert_eq!(
                input.job.borrow().snapshot().frame_count,
                if spatial { 2 } else { 1 }
            );
            assert_eq!(
                input.job.borrow().snapshot().spatial_position,
                if spatial { Some((1, 4)) } else { None }
            );
            handle(window.0, WM_COMMAND, WPARAM(1), LPARAM(0));
            assert!(!input.job.borrow().is_pending());
        }
    }
}

#[test]
fn failed_inflight_capture_keeps_session_and_requires_explicit_finish_retry() {
    unsafe {
        let input = input();
        let session = crate::long_capture::Session::new(
            Axis::Vertical,
            frame(Axis::Vertical, 0),
            MatchOptions {
                minimum_overlap: 8,
                maximum_shift: 8,
                maximum_mean_error: 3.0,
            },
        )
        .unwrap();
        let (release, wait) = mpsc::channel();
        *input.job.borrow_mut() = SessionJob::start_with_capture(session, move |_| {
            wait.recv_timeout(Duration::from_secs(5)).unwrap();
            Err("finish capture fixture".into())
        })
        .unwrap();
        let window = hidden(&input);
        handle(window.0, WM_COMMAND, WPARAM(101), LPARAM(0));
        handle(window.0, WM_COMMAND, WPARAM(1), LPARAM(0));
        release.send(()).unwrap();
        settle(window.0, &input);
        assert!(input.output.borrow().is_none());
        assert!(!input.finish.pending());
        let mut label = [0u16; 128];
        let length = GetDlgItemTextW(window.0, 103, &mut label);
        assert!(
            String::from_utf16_lossy(&label[..length as usize]).contains("finish capture fixture")
        );
        handle(window.0, WM_COMMAND, WPARAM(1), LPARAM(0));
        settle(window.0, &input);
        assert_eq!(
            input.output.borrow().as_ref().map(|p| (p.width, p.height)),
            Some((4, 16))
        );
    }
}

#[test]
fn cancel_close_destroy_and_display_change_discard_pending_finish_output() {
    unsafe {
        for message in [WM_COMMAND, WM_CLOSE, WM_DESTROY, WM_DISPLAYCHANGE] {
            let input = input();
            let session = crate::long_capture::Session::new(
                Axis::Vertical,
                frame(Axis::Vertical, 0),
                MatchOptions {
                    minimum_overlap: 8,
                    maximum_shift: 8,
                    maximum_mean_error: 3.0,
                },
            )
            .unwrap();
            let (release, wait) = mpsc::channel();
            *input.job.borrow_mut() = SessionJob::start_with_capture(session, move |_| {
                wait.recv_timeout(Duration::from_secs(5)).unwrap();
                Ok(frame(Axis::Vertical, 4))
            })
            .unwrap();
            let window = hidden(&input);
            handle(window.0, WM_COMMAND, WPARAM(101), LPARAM(0));
            handle(window.0, WM_COMMAND, WPARAM(1), LPARAM(0));
            if message == WM_DESTROY {
                DestroyWindow(window.0).unwrap();
            } else {
                handle(window.0, message, WPARAM(2), LPARAM(0));
            }
            let _ = release.send(());
            settle(window.0, &input);
            handle(window.0, WM_COMMAND, WPARAM(1), LPARAM(0));
            assert!(input.output.borrow().is_none());
            assert_eq!(
                input.job.borrow_mut().capture(),
                Err(crate::long_capture::DispatchError::Stopped)
            );
            assert_eq!(input.error.borrow().is_some(), message == WM_DISPLAYCHANGE);
        }
    }
}

#[test]
fn exact_global_hotkey_reaches_current_dialog_despite_disabled_busy_owner() {
    unsafe {
        use crate::desktop::{global_capture, State};
        use windows::{
            core::w,
            Win32::UI::Input::KeyboardAndMouse::{IsWindowEnabled, MOD_CONTROL, MOD_SHIFT},
        };
        let owner = crate::desktop::Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("capture owner test"),
                WS_OVERLAPPED,
                0,
                0,
                1,
                1,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let state = State::new(None, false).unwrap();
        state.capture_active.set(true);
        state.file_dialog.set(true);
        state.global_capture.set(true);
        state.global_long_capture.set(true);
        state.modals.register(&state.file_dialog);
        let _ = EnableWindow(owner.0, false);
        assert!(!IsWindowEnabled(owner.0).as_bool());
        assert!(!crate::desktop::capture_input::exclusive_available(&state));
        let mut input = input();
        input.finish = finish::State::new(state.capture_active.clone());
        let window = hidden(&input);
        let payload = |modifiers| LPARAM(((u32::from(b'3') << 16) | modifiers) as isize);
        let send = |wp, lp| global_capture::dispatch(owner.0, &state, WM_HOTKEY, wp, lp);
        let mut msg = MSG::default();
        let queued = |msg: &mut MSG| {
            PeekMessageW(
                msg,
                Some(window.0),
                super::super::finish_route::MESSAGE,
                super::super::finish_route::MESSAGE,
                PM_REMOVE,
            )
            .as_bool()
        };
        send(WPARAM(0x4f55), payload(MOD_CONTROL.0 | MOD_SHIFT.0));
        send(WPARAM(0x4f54), payload(MOD_CONTROL.0));
        state.global_long_capture.set(false);
        send(WPARAM(0x4f55), payload(MOD_CONTROL.0));
        assert!(!queued(&mut msg));
        state.global_long_capture.set(true);
        for _ in 0..3 {
            send(WPARAM(0x4f55), payload(MOD_CONTROL.0));
        }
        assert!(queued(&mut msg));
        assert!(!input.finish.pending()); // PostMessage 没有同步重入对话框。
        DispatchMessageW(&msg);
        assert!(!queued(&mut msg));
        assert!(input.finish.pending());
        settle(window.0, &input);
        assert_eq!(
            input.output.borrow().as_ref().map(|p| (p.width, p.height)),
            Some((4, 16))
        );
        assert!(state.capture_active.get() && state.file_dialog.get());
        assert!(state.global_capture.get() && state.global_long_capture.get());
        // 不借用测试的模拟注册状态去注销用户真实快捷键。
        state.global_capture.set(false);
        state.global_long_capture.set(false);
    }
}
