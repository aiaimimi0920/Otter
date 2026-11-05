use super::*;
use crate::{
    capture_color::SdrFrame,
    capture_dynamic::{Frame, Reason},
};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
fn outcome() -> Outcome {
    Outcome {
        frame: Frame::Sdr(SdrFrame {
            width: 1,
            height: 1,
            pixels: vec![255; 4].into(),
        }),
        reason: Some(Reason::ExplicitSdr),
        downgraded: false,
        display: None,
        diagnostic: None,
    }
}
fn settle(job: &mut CaptureJob) -> Result<Outcome, String> {
    let start = Instant::now();
    loop {
        if let Some(result) = job.take_ready() {
            return result;
        }
        assert!(start.elapsed() < Duration::from_secs(3));
        thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn worker_is_off_caller_thread_nonblocking_and_delivers_once() {
    let caller = thread::current().id();
    let (ready, entered) = mpsc::sync_channel(1);
    let (release, wait) = mpsc::sync_channel(1);
    let mut job = CaptureJob::spawn(move |_| {
        assert_ne!(thread::current().id(), caller);
        ready.send(()).unwrap();
        wait.recv_timeout(Duration::from_secs(3)).unwrap();
        Ok(outcome())
    })
    .unwrap();
    entered.recv_timeout(Duration::from_secs(3)).unwrap();
    assert!(job.pending());
    assert!(job.take_ready().is_none());
    release.send(()).unwrap();
    let output = settle(&mut job).unwrap();
    assert_eq!(output.reason, Some(Reason::ExplicitSdr));
    assert!(!job.pending());
    assert!(job.take_ready().is_none());
}
#[test]
fn cancellation_drops_late_pixels_even_if_work_returns_success() {
    let (ready, entered) = mpsc::sync_channel(1);
    let (release, wait) = mpsc::sync_channel(1);
    let output = outcome();
    let Frame::Sdr(frame) = &output.frame else {
        unreachable!()
    };
    let weak = frame.pixels.downgrade();
    let mut job = CaptureJob::spawn(move |cancel| {
        ready.send(()).unwrap();
        wait.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(cancel.load(Ordering::Acquire));
        Ok(output)
    })
    .unwrap();
    entered.recv_timeout(Duration::from_secs(3)).unwrap();
    job.cancel();
    release.send(()).unwrap();
    assert_eq!(settle(&mut job).err().as_deref(), Some("Capture cancelled"));
    assert!(weak.upgrade().is_none());
}
#[test]
fn cancellation_after_completion_still_rejects_unconsumed_result_and_drop_joins() {
    let mut job = CaptureJob::spawn(|_| Ok(outcome())).unwrap();
    let start = Instant::now();
    while !job.worker.as_ref().unwrap().is_finished() {
        assert!(start.elapsed() < Duration::from_secs(3));
        thread::sleep(Duration::from_millis(1));
    }
    job.cancel();
    assert!(settle(&mut job).is_err());
    let cleaned = Arc::new(AtomicBool::new(false));
    let cleanup = cleaned.clone();
    let (ready, entered) = mpsc::sync_channel(1);
    let job = CaptureJob::spawn(move |cancel| {
        ready.send(()).unwrap();
        let start = Instant::now();
        while !cancel.load(Ordering::Acquire) {
            assert!(start.elapsed() < Duration::from_secs(3));
            thread::sleep(Duration::from_millis(1));
        }
        cleanup.store(true, Ordering::Release);
        Ok(outcome())
    })
    .unwrap();
    entered.recv_timeout(Duration::from_secs(3)).unwrap();
    drop(job);
    assert!(cleaned.load(Ordering::Acquire));
}
#[test]
fn panic_and_error_are_terminal_without_successful_output() {
    let mut job = CaptureJob::spawn(|_| panic!("injected worker panic")).unwrap();
    assert_eq!(
        settle(&mut job).err().as_deref(),
        Some("Capture worker panicked")
    );
    assert!(!job.pending());
    let mut job = CaptureJob::spawn(|_| Err("injected capture failure".into())).unwrap();
    assert_eq!(
        settle(&mut job).err().as_deref(),
        Some("injected capture failure")
    );
}
