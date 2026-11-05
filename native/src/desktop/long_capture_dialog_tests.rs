use super::*;
use crate::long_capture::{Frame, Session};
use std::time::{Duration, Instant};
#[test]
fn hidden_dialog_consumes_spatial_replies_for_preview_and_finish() {
    unsafe {
        let input = input();
        let window = hidden(&input);
        let frame = Frame::new(
            4,
            16,
            (0..16)
                .flat_map(|y| vec![4 + y; 16])
                .collect::<Vec<_>>()
                .into(),
        )
        .unwrap();
        input.job.borrow_mut().append_at(frame, (1, 4)).unwrap();
        settle(window.0, &input);
        assert_eq!(input.job.borrow().snapshot().spatial_position, Some((1, 4)));
        handle(window.0, WM_COMMAND, WPARAM(102), LPARAM(0));
        settle(window.0, &input);
        assert_eq!(
            input.preview.borrow().as_ref().map(|p| (p.width, p.height)),
            Some((5, 20))
        );
        {
            let preview = input.preview.borrow();
            let pixels = &preview.as_ref().unwrap().pixels;
            assert_eq!(&pixels[..4], &[0, 0, 0, 255]);
            assert_eq!(&pixels[16..20], &[208, 208, 208, 255]);
        }
        assert!(input.output.borrow().is_none());
        handle(window.0, WM_COMMAND, WPARAM(1), LPARAM(0));
        settle(window.0, &input);
        assert_eq!(
            input.output.borrow().as_ref().map(|p| (p.width, p.height)),
            Some((4, 16))
        );
        // 显示棋盘格不能进入最终导出；第一行仍是原始黑色。
        assert_eq!(
            &input.output.borrow().as_ref().unwrap().pixels[..4],
            &[0, 0, 0, 255]
        );
    }
}
#[test]
fn direction_switch_drives_manual_and_automatic_capture_on_both_axes() {
    unsafe {
        for axis in [Axis::Vertical, Axis::Horizontal] {
            for automatic in [false, true] {
                let mut input = input();
                input.axis = axis;
                let make = move |origin: u8| {
                    let (width, height) = match axis {
                        Axis::Vertical => (4, 16),
                        Axis::Horizontal => (16, 4),
                    };
                    let mut bytes = Vec::new();
                    for y in 0..height {
                        for x in 0..width {
                            let position = match axis {
                                Axis::Vertical => y,
                                Axis::Horizontal => x,
                            };
                            bytes.extend_from_slice(&[origin + position as u8; 4]);
                        }
                    }
                    Frame::new(width, height, bytes.into()).unwrap()
                };
                let session = Session::new(
                    axis,
                    make(4),
                    MatchOptions {
                        minimum_overlap: 8,
                        maximum_shift: 8,
                        maximum_mean_error: 3.0,
                    },
                )
                .unwrap();
                let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
                let seen = count.clone();
                *input.job.borrow_mut() =
                    SessionJob::start_with_actions(session, move |mode, _| {
                        assert_eq!(mode, crate::long_capture::CaptureMode::ScrollBackward);
                        seen.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                        Ok(make(0))
                    })
                    .unwrap();
                let window = hidden(&input);
                handle(window.0, WM_COMMAND, WPARAM(107), LPARAM(0));
                assert!(!input.forward.get());
                let mut label = [0u16; 32];
                let length = GetDlgItemTextW(window.0, 107, &mut label);
                assert_eq!(
                    String::from_utf16_lossy(&label[..length as usize]),
                    match axis {
                        Axis::Vertical => "方向：向上",
                        Axis::Horizontal => "方向：向左",
                    }
                );
                handle(
                    window.0,
                    WM_COMMAND,
                    WPARAM(if automatic { 106 } else { 105 }),
                    LPARAM(0),
                );
                assert!(
                    !windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(
                        GetDlgItem(Some(window.0), 107).unwrap()
                    )
                    .as_bool()
                );
                // 即使绕过禁用按钮投递命令，也不能改变在途动作或自动循环的方向。
                handle(window.0, WM_COMMAND, WPARAM(107), LPARAM(0));
                assert!(!input.forward.get());
                settle(window.0, &input);
                assert_eq!(
                    count.load(std::sync::atomic::Ordering::Acquire),
                    if automatic { 2 } else { 1 }
                );
                assert_eq!(input.job.borrow().snapshot().frame_count, 2);
                assert!(!input.automatic.borrow().running());
                handle(window.0, WM_COMMAND, WPARAM(102), LPARAM(0));
                settle(window.0, &input);
                let preview = input.preview.borrow();
                let image = preview.as_ref().unwrap();
                assert_eq!(
                    (image.width, image.height),
                    match axis {
                        Axis::Vertical => (4, 20),
                        Axis::Horizontal => (20, 4),
                    }
                );
                drop(preview);
                handle(window.0, WM_COMMAND, WPARAM(107), LPARAM(0));
                assert!(input.forward.get());
            }
        }
    }
}
#[test]
fn automatic_capture_stops_on_duplicate_and_user_stop_keeps_current_result() {
    unsafe {
        for stop in [false, true] {
            let input = input();
            let make = |origin: u8| {
                Frame::new(
                    4,
                    16,
                    (0..16)
                        .flat_map(|y| vec![origin + y; 16])
                        .collect::<Vec<_>>()
                        .into(),
                )
                .unwrap()
            };
            let session = Session::new(
                Axis::Vertical,
                make(0),
                MatchOptions {
                    minimum_overlap: 8,
                    maximum_shift: 8,
                    maximum_mean_error: 3.0,
                },
            )
            .unwrap();
            let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let seen = count.clone();
            *input.job.borrow_mut() = SessionJob::start_with_actions(session, move |mode, _| {
                assert_eq!(mode, crate::long_capture::CaptureMode::ScrollForward);
                seen.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                Ok(make(4))
            })
            .unwrap();
            let window = hidden(&input);
            handle(window.0, WM_COMMAND, WPARAM(106), LPARAM(0));
            assert!(input.automatic.borrow().running());
            if stop {
                handle(window.0, WM_COMMAND, WPARAM(106), LPARAM(0));
            }
            settle(window.0, &input);
            assert!(!input.automatic.borrow().running());
            assert_eq!(
                count.load(std::sync::atomic::Ordering::Acquire),
                if stop { 1 } else { 2 }
            );
            assert_eq!(input.job.borrow().snapshot().frame_count, 2);
            assert!(input.output.borrow().is_none());
            assert_eq!(input.preview.borrow().is_some(), !stop);
        }
    }
}
#[test]
fn auto_skips_unconfirmed_fixed_edges_then_previews_confirmed_content() {
    unsafe {
        let input = input();
        let frame = |origin: u32| {
            let mut pixels = Vec::new();
            for row in 0..16 {
                let value = if row < 2 {
                    10
                } else if row >= 14 {
                    20
                } else {
                    ((row + origin) * 37 % 200) as u8
                };
                for _ in 0..4 {
                    pixels.extend_from_slice(&[value, value / 2, 240 - value, 255]);
                }
            }
            Frame::new(4, 16, pixels.into()).unwrap()
        };
        let session = Session::new(
            Axis::Vertical,
            frame(0),
            MatchOptions {
                minimum_overlap: 8,
                maximum_shift: 4,
                maximum_mean_error: 3.0,
            },
        )
        .unwrap();
        let mut count = 0;
        *input.job.borrow_mut() = SessionJob::start_with_actions(session, move |_, _| {
            count += 1;
            Ok(frame(if count == 1 { 2 } else { 4 }))
        })
        .unwrap();
        let window = hidden(&input);
        handle(window.0, WM_COMMAND, WPARAM(106), LPARAM(0));
        settle(window.0, &input);
        assert_eq!(input.job.borrow().snapshot().frame_count, 3);
        assert_eq!(input.job.borrow().snapshot().confirmed_edges, Some((2, 2)));
        assert!(!input.automatic.borrow().running());
        let preview = input.preview.borrow();
        let image = preview.as_ref().unwrap();
        assert_eq!((image.width, image.height), (4, 20));
        assert!(input.output.borrow().is_none());
    }
}
pub(super) fn input() -> Input {
    let frame = |origin: u8| {
        Frame::new(
            4,
            16,
            (0..16)
                .flat_map(|y| vec![origin + y; 16])
                .collect::<Vec<_>>()
                .into(),
        )
        .unwrap()
    };
    let session = Session::new(
        Axis::Vertical,
        frame(0),
        MatchOptions {
            minimum_overlap: 8,
            maximum_shift: 8,
            maximum_mean_error: 3.0,
        },
    )
    .unwrap();
    let job = SessionJob::start_with_capture(session, move |_| Ok(frame(4))).unwrap();
    Input {
        finish: finish::State::new(std::rc::Rc::new(Cell::new(true))),
        outline: RefCell::new(None),
        movement: movement::State::new(Region {
            left: 0,
            top: 0,
            width: 4,
            height: 16,
        }),
        axis: Axis::Vertical,
        forward: Cell::new(true),
        automatic: RefCell::new(Default::default()),
        recording: recording::State::new(None),
        job: RefCell::new(job),
        output: RefCell::new(None),
        preview: RefCell::new(None),
        error: RefCell::new(None),
        protect: false,
    }
}
pub(super) unsafe fn hidden(input: &Input) -> crate::desktop::Window {
    let template = layout::template();
    let hwnd = CreateDialogIndirectParamW(
        None,
        template.as_ptr().cast(),
        None,
        Some(procedure),
        LPARAM(input as *const _ as isize),
    )
    .unwrap();
    assert!(!IsWindowVisible(hwnd).as_bool());
    crate::desktop::Window(hwnd)
}
pub(super) unsafe fn settle(hwnd: HWND, input: &Input) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while input.job.borrow().is_pending() {
        handle(hwnd, WM_TIMER, WPARAM(TIMER), LPARAM(0));
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn hidden_scroll_button_uses_action_source_and_manual_source_disables_it() {
    unsafe {
        let manual = input();
        let window = hidden(&manual);
        assert!(
            !windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(
                GetDlgItem(Some(window.0), 105).unwrap()
            )
            .as_bool()
        );
        // 生产只允许一个活动长截图；结束手动夹具后再建立滚动夹具。
        drop(window);
        let input = input();
        let make = |origin: u8| {
            Frame::new(
                4,
                16,
                (0..16)
                    .flat_map(|y| vec![origin + y; 16])
                    .collect::<Vec<_>>()
                    .into(),
            )
            .unwrap()
        };
        let session = Session::new(
            Axis::Vertical,
            make(0),
            MatchOptions {
                minimum_overlap: 8,
                maximum_shift: 8,
                maximum_mean_error: 3.0,
            },
        )
        .unwrap();
        *input.job.borrow_mut() = SessionJob::start_with_actions(session, move |mode, _| {
            assert_eq!(mode, crate::long_capture::CaptureMode::ScrollForward);
            Ok(make(4))
        })
        .unwrap();
        let scroll = hidden(&input);
        assert!(
            windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(
                GetDlgItem(Some(scroll.0), 105).unwrap()
            )
            .as_bool()
        );
        handle(scroll.0, WM_COMMAND, WPARAM(105), LPARAM(0));
        settle(scroll.0, &input);
        assert_eq!(input.job.borrow().snapshot().frame_count, 2);
        assert!(input.output.borrow().is_none());
    }
}
#[test]
fn hidden_controls_capture_and_compose_using_synthetic_source() {
    unsafe {
        let input = input();
        let window = hidden(&input);
        handle(window.0, WM_COMMAND, WPARAM(101), LPARAM(0));
        settle(window.0, &input);
        assert_eq!(input.job.borrow().snapshot().frame_count, 2);
        handle(window.0, WM_COMMAND, WPARAM(102), LPARAM(0));
        settle(window.0, &input);
        assert!(input.preview.borrow().is_some());
        assert!(input.output.borrow().is_none());
        handle(window.0, WM_COMMAND, WPARAM(1), LPARAM(0));
        settle(window.0, &input);
        let result = input.output.borrow();
        let image = result.as_ref().unwrap();
        assert_eq!((image.width, image.height), (4, 20));
    }
}
#[test]
fn layout_change_cancels_without_output() {
    unsafe {
        let input = input();
        let window = hidden(&input);
        handle(window.0, WM_DISPLAYCHANGE, WPARAM(0), LPARAM(0));
        assert!(input.error.borrow().is_some());
        assert!(input.output.borrow().is_none());
        assert_eq!(
            input.job.borrow_mut().capture(),
            Err(crate::long_capture::DispatchError::Stopped)
        );
    }
}
