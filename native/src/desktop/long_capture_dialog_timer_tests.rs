//! 使用真正消息循环回归旧手动/主动采集，不用直接调用 WM_TIMER 掩盖等待 timer 丢失。
use super::*;
use crate::long_capture::{Frame, Session};
use std::time::{Duration, Instant};
#[test]
fn slow_manual_and_active_scroll_workers_keep_the_timer_until_the_reply() {
    unsafe {
        for automatic in [false, true] {
            let input = tests::input();
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
            *input.job.borrow_mut() = SessionJob::start_with_actions(session, move |_, _| {
                std::thread::sleep(Duration::from_millis(160));
                Ok(make(4))
            })
            .unwrap();
            let window = tests::hidden(&input);
            handle(
                window.0,
                WM_COMMAND,
                WPARAM(if automatic { 106 } else { 101 }),
                LPARAM(0),
            );
            let deadline = Instant::now() + Duration::from_secs(5);
            while input.job.borrow().is_pending() {
                let mut message = MSG::default();
                while PeekMessageW(&mut message, Some(window.0), 0, 0, PM_REMOVE).as_bool() {
                    DispatchMessageW(&message);
                }
                assert!(
                    Instant::now() < deadline,
                    "original timer stopped before worker reply"
                );
                std::thread::sleep(Duration::from_millis(2));
            }
            assert_eq!(input.job.borrow().snapshot().frame_count, 2);
            assert!(!input.automatic.borrow().running());
        }
    }
}
