//! Recording-state tests with hidden controls/real workers; synthetic sources/notifications, no screen reads. Timer submodules separately verify real-hook cleanup.
#[path = "long_capture_recording_movement_tests.rs"]
mod movement_tests;
#[path = "long_capture_recording_overlay_tests.rs"]
mod overlay_tests;
#[path = "long_capture_recording_timer_tests.rs"]
mod timer_tests;
use super::super::tests::{hidden as hidden_dialog, input as manual_input};
use super::*;
use crate::long_capture::{CaptureMode, CaptureSource, Frame, OutputTarget};
use crate::ui_text::{format as message, text as tr};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};
fn frame(x0: u32, y0: u32) -> Frame {
    frame_with_height(x0, y0, 48)
}
fn frame_with_height(x0: u32, y0: u32, height: u32) -> Frame {
    let pixels = (0..height)
        .flat_map(|y| {
            (0..32u32).flat_map(move |x| {
                let mut value = (x + x0).wrapping_mul(7919) ^ (y + y0).wrapping_mul(104729);
                value = (value ^ (value >> 16)).wrapping_mul(0x7feb352d);
                let v = (value ^ (value >> 15)) as u8;
                [v, v / 2, 255 - v, 255]
            })
        })
        .collect::<Vec<_>>();
    Frame::new(32, height, pixels.into()).unwrap()
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
    setup_with_first(frame(0, 0), frames, gate)
}
fn setup_with_first(
    first: Frame,
    frames: Vec<std::result::Result<Frame, String>>,
    gate: Option<mpsc::Receiver<()>>,
) -> (Input, Arc<AtomicUsize>) {
    let mut input = manual_input();
    let calls = Arc::new(AtomicUsize::new(0));
    *input.job.borrow_mut() = SessionJob::start_detecting_for(
        first.clone(),
        Source {
            frames: frames.into(),
            last: first,
            calls: calls.clone(),
            gate,
        },
        OutputTarget::Document,
    )
    .unwrap();
    // Only this test construction bypasses external HWND binding; production enable flags come only from bound Targets.
    input.recording.enabled = true;
    (input, calls)
}
unsafe fn hidden(input: &Input) -> crate::desktop::Window {
    let window = hidden_dialog(input);
    // Production starts recording after overlay creation/layout; fixtures likewise exclude control-creation time from sampling periods.
    input.recording.schedule.borrow_mut().start(Instant::now());
    status(window.0, tr("long-capture-recording"));
    window
}
unsafe fn caption(hwnd: HWND) -> String {
    let mut text = [0u16; 256];
    let length = GetDlgItemTextW(hwnd, 103, &mut text) as usize;
    String::from_utf16(&text[..length]).unwrap()
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
fn periodic_sampling_recovers_after_transient_repaints_without_wheel_notifications() {
    unsafe {
        let unknown = Frame::new(32, 48, vec![0; 32 * 48 * 4].into()).unwrap();
        let mut frames = vec![Ok(unknown); 5];
        frames.extend([Ok(frame(0, 0)), Ok(frame(0, 4)), Ok(frame(0, 8))]);
        let (input, calls) = setup(frames, None);
        let window = hidden(&input);
        let deadline = Instant::now() + Duration::from_secs(5);
        while input.job.borrow().snapshot().frame_count < 3 || input.preview.borrow().is_none() {
            super::super::handle(window.0, WM_TIMER, WPARAM(TIMER), LPARAM(0));
            assert_eq!(
                caption(window.0),
                tr("long-capture-recording"),
                "sampling must not change the header"
            );
            assert!(
                input.recording.running(),
                "transient repaint must not stop recording"
            );
            assert!(
                Instant::now() < deadline,
                "dragged content was never sampled"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(calls.load(Ordering::Acquire) >= 8);
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
        assert_eq!(input.recording.schedule.borrow().phase(), Phase::Cancelled);
    }
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
fn finish_behind_a_slow_capture_still_collects_the_last_dragged_frame() {
    unsafe {
        let (release, gate) = mpsc::channel();
        let (input, calls) = setup(vec![Ok(frame(0, 4)), Ok(frame(0, 8))], Some(gate));
        let window = hidden(&input);
        input
            .recording
            .schedule
            .borrow_mut()
            .notify(Instant::now() - Duration::from_millis(25));
        drive(window.0, &input);
        assert!(input.job.borrow().is_pending());
        command(window.0, 1);
        // The actual worker stays in flight beyond the tail deadline; the final frame has not entered the sole task slot.
        std::thread::sleep(Duration::from_millis(400));
        release.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while input.output.borrow().is_none() {
            assert!(
                [tr("long-capture-last-frame"), tr("long-capture-stitching")]
                    .contains(&caption(window.0).as_str()),
                "finishing header was overwritten by a frame reply"
            );
            super::super::handle(window.0, WM_TIMER, WPARAM(TIMER), LPARAM(0));
            assert!(Instant::now() < deadline, "recording did not finish");
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(calls.load(Ordering::Acquire), 2);
        assert_eq!(
            input
                .output
                .borrow()
                .as_ref()
                .map(|image| (image.width, image.height)),
            Some((32, 56))
        );
    }
}
#[test]
fn covered_final_sample_finishes_with_the_last_trusted_pixels() {
    unsafe {
        let first = frame(0, 0);
        let (input, calls) = setup_with_first(
            first.clone(),
            vec![Err(long_capture_source::scroll::TARGET_COVERED.into())],
            None,
        );
        let window = hidden(&input);
        hint(window.0, &input);
        command(window.0, 1);
        assert!(input.finish.pending());
        pump(window.0, &input);
        assert_eq!(calls.load(Ordering::Acquire), 1);
        assert_eq!(input.job.borrow().snapshot().frame_count, 1);
        assert_eq!(input.recording.schedule.borrow().phase(), Phase::Cancelled);
        let output = input.output.borrow();
        let output = output
            .as_ref()
            .expect("finish must not require a second click");
        assert_eq!((output.width, output.height), (32, 48));
        assert_eq!(&*output.pixels, first.bytes());
    }
}

#[test]
fn document_limit_during_finish_keeps_the_dialog_open_and_last_result_finishable() {
    unsafe {
        let first = frame_with_height(0, 0, 16384);
        let (release, gate) = mpsc::channel();
        let (input, calls) = setup_with_first(
            first.clone(),
            vec![Ok(frame_with_height(0, 4, 16384))],
            Some(gate),
        );
        let window = hidden(&input);
        input
            .recording
            .schedule
            .borrow_mut()
            .notify(Instant::now() - Duration::from_millis(25));
        drive(window.0, &input);
        assert!(input.job.borrow().is_pending());
        command(window.0, 1);
        assert!(input.finish.pending());
        release.send(()).unwrap();
        pump(window.0, &input);
        assert_eq!(calls.load(Ordering::Acquire), 1);
        assert_eq!(caption(window.0), tr("long-capture-size-limit"));
        assert!(!input.recording.running());
        assert!(!input.finish.pending());
        assert!(input.output.borrow().is_none());
        assert_eq!(input.job.borrow().snapshot().frame_count, 1);
        command(window.0, 1);
        pump(window.0, &input);
        let output = input.output.borrow();
        let output = output.as_ref().unwrap();
        assert_eq!((output.width, output.height), (32, 16384));
        assert_eq!(&*output.pixels, first.bytes());
        assert_eq!(calls.load(Ordering::Acquire), 1);
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
        input.recording.schedule.borrow_mut().notify(Instant::now()); // Draining accepts no new hints.
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
        assert_eq!(caption(window.0), tr("long-capture-record-stopped"));
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
        for event in [0, WM_DISPLAYCHANGE, WM_DPICHANGED] {
            let (input, calls) = setup(vec![Err("Scroll target closed".into())], None);
            let window = hidden(&input);
            hint(window.0, &input);
            if event != 0 {
                super::super::handle(window.0, event, WPARAM(0), LPARAM(0));
            }
            pump(window.0, &input);
            assert!(!input.recording.running());
            assert!(input.recording.hook.borrow().is_none());
            assert_eq!(
                caption(window.0),
                if event == 0 {
                    message(
                        "long-capture-failed",
                        &[("error", "Scroll target closed".into())],
                    )
                } else {
                    tr("long-capture-display-changed").to_owned()
                }
            );
            assert_eq!(calls.load(Ordering::Acquire), usize::from(event == 0));
            command(window.0, 1);
            pump(window.0, &input);
            assert!(input.output.borrow().is_some());
        }
    }
}
#[test]
fn axis_change_stops_but_transient_unreliable_frames_keep_recording_without_commit() {
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
            assert_eq!(input.recording.running(), !cross_axis);
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
fn accepted_viewfinder_move_keeps_recording_and_refreshes_preview() {
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
        command(window.0, 110);
        pump(window.0, &input);
        assert!(input.job.borrow().snapshot().spatial_position.is_some());
        assert_eq!(input.movement.region().left, 1);
        assert!(input.recording.running());
        assert!(input.preview.borrow().is_some());
        command(window.0, 1);
        pump(window.0, &input);
        assert!(input.output.borrow().is_some());
    }
}
#[test]
fn moving_before_first_scroll_does_not_choose_the_page_axis() {
    unsafe {
        let (input, _) = setup(
            vec![Ok(frame(1, 0)), Ok(frame(5, 0)), Ok(frame(9, 0))],
            None,
        );
        let window = hidden(&input);
        command(window.0, 110);
        pump(window.0, &input);
        assert_eq!(input.job.borrow().snapshot().spatial_position, Some((1, 0)));
        assert_eq!(input.job.borrow().snapshot().axis, None);
        assert!(input.recording.running());
        hint(window.0, &input);
        pump(window.0, &input);
        assert_eq!(input.job.borrow().snapshot().axis, Some(Axis::Horizontal));
        assert_eq!(
            input.job.borrow().snapshot().direction,
            Some(crate::long_capture::Direction::Right)
        );
        command(window.0, 1);
        pump(window.0, &input);
        assert_eq!(
            input.output.borrow().as_ref().map(|i| (i.width, i.height)),
            Some((41, 48))
        );
    }
}
