//! 隐藏控件/真实 worker 的录制状态测试；合成源与通知，不读取屏幕。timer 子模块另验真实 hook 清理。
#[path = "long_capture_recording_timer_tests.rs"]
mod timer_tests;
use super::super::tests::{hidden, input as manual_input};
use super::*;
use crate::long_capture::{CaptureMode, CaptureSource, Frame};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};
fn frame(x0: u32, y0: u32) -> Frame {
    let pixels = (0..48u32)
        .flat_map(|y| {
            (0..32u32).flat_map(move |x| {
                let mut value = (x + x0).wrapping_mul(7919) ^ (y + y0).wrapping_mul(104729);
                value = (value ^ (value >> 16)).wrapping_mul(0x7feb352d);
                let v = (value ^ (value >> 15)) as u8;
                [v, v / 2, 255 - v, 255]
            })
        })
        .collect::<Vec<_>>();
    Frame::new(32, 48, pixels.into()).unwrap()
}
struct Source {
    frames: VecDeque<std::result::Result<Frame, String>>,
    last: Frame,
    calls: Arc<AtomicUsize>,
    gate: Option<mpsc::Receiver<()>>,
}
impl CaptureSource for Source {
    fn can_move(&self) -> bool {
        true
    }
    fn prepare_move(
        &mut self,
        _: (i32, i32),
        cancel: &AtomicBool,
    ) -> std::result::Result<Frame, String> {
        self.capture(CaptureMode::Current, cancel)
    }
    fn capture(&mut self, mode: CaptureMode, _: &AtomicBool) -> std::result::Result<Frame, String> {
        assert_eq!(mode, CaptureMode::Current);
        self.calls.fetch_add(1, Ordering::AcqRel);
        if let Some(gate) = self.gate.take() {
            gate.recv_timeout(Duration::from_secs(5))
                .map_err(|error| error.to_string())?;
        }
        match self.frames.pop_front() {
            Some(Ok(frame)) => {
                self.last = frame.clone();
                Ok(frame)
            }
            Some(Err(error)) => Err(error),
            None => Ok(self.last.clone()),
        }
    }
}
fn setup(
    frames: Vec<std::result::Result<Frame, String>>,
    gate: Option<mpsc::Receiver<()>>,
) -> (Input, Arc<AtomicUsize>) {
    let mut input = manual_input();
    let calls = Arc::new(AtomicUsize::new(0));
    let first = frame(0, 0);
    *input.job.borrow_mut() = SessionJob::start_detecting(
        first.clone(),
        Source {
            frames: frames.into(),
            last: first,
            calls: calls.clone(),
            gate,
        },
    )
    .unwrap();
    // 仅此测试构造绕开外部 HWND 绑定；生产启用位只由已绑定 Target 构造。
    input.recording.enabled = true;
    input.recording.schedule.borrow_mut().start(Instant::now());
    (input, calls)
}
unsafe fn pump(hwnd: HWND, input: &Input) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        super::super::handle(hwnd, WM_TIMER, WPARAM(TIMER), LPARAM(0));
        if !input.job.borrow().is_pending() && !input.recording.schedule.borrow().needs_timer(false)
        {
            break;
        }
        assert!(Instant::now() < deadline, "recording did not settle");
        std::thread::sleep(Duration::from_millis(2));
    }
}
unsafe fn hint(hwnd: HWND, input: &Input) {
    input.recording.schedule.borrow_mut().notify(Instant::now());
    drive(hwnd, input);
}
unsafe fn command(hwnd: HWND, id: usize) {
    super::super::handle(hwnd, WM_COMMAND, WPARAM(id), LPARAM(0));
}
#[test]
fn merged_wheel_burst_updates_preview_then_finishes_through_the_original_worker() {
    unsafe {
        let (input, calls) = setup(vec![Ok(frame(0, 4)), Ok(frame(0, 8))], None);
        let window = hidden(&input);
        for _ in 0..10_000 {
            input.recording.schedule.borrow_mut().notify(Instant::now());
        }
        drive(window.0, &input);
        pump(window.0, &input);
        assert_eq!(calls.load(Ordering::Acquire), 3);
        assert_eq!(input.job.borrow().snapshot().frame_count, 3);
        assert_eq!(input.job.borrow().snapshot().axis, Some(Axis::Vertical));
        assert_eq!(
            input.job.borrow().snapshot().direction,
            Some(crate::long_capture::Direction::Down)
        );
        assert!(input.preview.borrow().is_some());
        assert!(input.output.borrow().is_none());
        command(window.0, 1);
        pump(window.0, &input);
        assert_eq!(
            input
                .output
                .borrow()
                .as_ref()
                .map(|image| (image.width, image.height)),
            Some((32, 56))
        );
        command(window.0, 1);
        assert!(!input.job.borrow().is_pending());
        assert!(input.recording.hook.borrow().is_none());
    }
}
#[test]
fn finish_before_debounce_drains_recent_hint_without_accepting_late_input() {
    unsafe {
        let (input, calls) = setup(vec![Ok(frame(4, 0))], None);
        let window = hidden(&input);
        hint(window.0, &input);
        assert!(!input.job.borrow().is_pending());
        command(window.0, 1);
        command(window.0, 1);
        input.recording.schedule.borrow_mut().notify(Instant::now()); // Draining 不接纳新提示。
        pump(window.0, &input);
        assert_eq!(calls.load(Ordering::Acquire), 3);
        assert_eq!(
            input.job.borrow().snapshot().direction,
            Some(crate::long_capture::Direction::Right)
        );
        assert_eq!(
            input
                .output
                .borrow()
                .as_ref()
                .map(|image| (image.width, image.height)),
            Some((36, 48))
        );
        assert!(input.recording.hook.borrow().is_none());
    }
}
#[test]
fn stopping_during_inflight_keeps_that_frame_but_drops_all_followup_sampling() {
    unsafe {
        let (release, gate) = mpsc::channel();
        let (input, calls) = setup(vec![Ok(frame(0, 4))], Some(gate));
        let window = hidden(&input);
        hint(window.0, &input);
        std::thread::sleep(Duration::from_millis(25));
        drive(window.0, &input);
        assert!(input.job.borrow().is_pending());
        input.recording.schedule.borrow_mut().notify(Instant::now());
        command(window.0, 112);
        release.send(()).unwrap();
        pump(window.0, &input);
        assert!(!input.recording.running());
        assert_eq!(calls.load(Ordering::Acquire), 1);
        assert_eq!(input.job.borrow().snapshot().frame_count, 2);
        super::super::handle(window.0, MESSAGE, WPARAM(7), LPARAM(0));
        pump(window.0, &input);
        assert_eq!(calls.load(Ordering::Acquire), 1);
        command(window.0, 1);
        pump(window.0, &input);
        assert!(input.output.borrow().is_some());
    }
}
#[test]
fn cancellation_retires_late_results_and_a_new_session_has_no_old_axis_or_frames() {
    unsafe {
        let (release, gate) = mpsc::channel();
        let (input, calls) = setup(vec![Ok(frame(0, 4))], Some(gate));
        let window = hidden(&input);
        hint(window.0, &input);
        std::thread::sleep(Duration::from_millis(25));
        drive(window.0, &input);
        command(window.0, 2);
        release.send(()).unwrap();
        assert!(input.job.borrow_mut().take_ready().is_none());
        assert!(input.output.borrow().is_none());
        assert!(input.recording.hook.borrow().is_none());
        assert_eq!(input.recording.schedule.borrow().phase(), Phase::Cancelled);
        drop(window);
        drop(input);
        assert!(calls.load(Ordering::Acquire) <= 1);
        let (fresh, _) = setup(vec![], None);
        let fresh_window = hidden(&fresh);
        command(fresh_window.0, 1);
        pump(fresh_window.0, &fresh);
        assert_eq!(fresh.job.borrow().snapshot().frame_count, 1);
        assert_eq!(fresh.job.borrow().snapshot().axis, None);
        assert_eq!(
            fresh
                .output
                .borrow()
                .as_ref()
                .map(|image| (image.width, image.height)),
            Some((32, 48))
        );
    }
}
#[test]
fn target_failure_and_layout_dpi_change_stop_but_preserve_the_finalizable_first_frame() {
    unsafe {
        for message in [0, WM_DISPLAYCHANGE, WM_DPICHANGED] {
            let (input, calls) = setup(vec![Err("Scroll target closed".into())], None);
            let window = hidden(&input);
            if message == 0 {
                hint(window.0, &input);
                pump(window.0, &input);
            } else {
                hint(window.0, &input);
                super::super::handle(window.0, message, WPARAM(0), LPARAM(0));
                pump(window.0, &input);
            }
            assert!(!input.recording.running());
            assert!(input.recording.hook.borrow().is_none());
            assert_eq!(calls.load(Ordering::Acquire), usize::from(message == 0));
            command(window.0, 1);
            pump(window.0, &input);
            assert!(input.output.borrow().is_some());
        }
    }
}
#[test]
fn axis_change_and_repeated_unreliable_frames_stop_without_silent_commit() {
    unsafe {
        for cross_axis in [false, true] {
            let frames = if cross_axis {
                vec![Ok(frame(0, 4)), Ok(frame(4, 4))]
            } else {
                vec![Ok(Frame::new(32, 48, vec![0; 32 * 48 * 4].into()).unwrap())]
            };
            let (input, _) = setup(frames, None);
            let window = hidden(&input);
            hint(window.0, &input);
            pump(window.0, &input);
            assert!(!input.recording.running());
            assert_eq!(
                input.job.borrow().snapshot().frame_count,
                if cross_axis { 2 } else { 1 }
            );
            command(window.0, 1);
            pump(window.0, &input);
            assert_eq!(
                input.output.borrow().as_ref().unwrap().height,
                if cross_axis { 52 } else { 48 }
            );
        }
    }
}
#[test]
fn accepted_explicit_viewfinder_move_retires_wheel_authority_permanently() {
    unsafe {
        let (input, _) = setup(
            vec![
                Ok(frame(0, 4)),
                Ok(frame(0, 8)),
                Ok(frame(0, 8)),
                Ok(frame(1, 8)),
            ],
            None,
        );
        let window = hidden(&input);
        hint(window.0, &input);
        pump(window.0, &input);
        assert_eq!(input.job.borrow().snapshot().frame_count, 3);
        command(window.0, 112);
        command(window.0, 110);
        pump(window.0, &input);
        assert!(input.job.borrow().snapshot().spatial_position.is_some());
        assert_eq!(input.movement.region().left, 1);
        assert!(input.recording.revoked.get());
        command(window.0, 112);
        assert!(!input.recording.running());
        assert!(input.recording.hook.borrow().is_none());
        assert!(
            !windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(
                GetDlgItem(Some(window.0), 112).unwrap()
            )
            .as_bool()
        );
        command(window.0, 1);
        pump(window.0, &input);
        assert!(input.output.borrow().is_some());
    }
}
