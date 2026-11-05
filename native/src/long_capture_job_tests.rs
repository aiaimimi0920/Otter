use super::*;
use std::time::{Duration, Instant};
#[path = "long_capture_source_tests.rs"]
mod source_tests;
#[test]
fn spatial_transition_is_atomic_and_followup_capture_preview_compose_use_same_worker() {
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
    let mut job = SessionJob::start_with_capture(session, |_| Ok(frame(12))).unwrap();
    job.append(frame(4)).unwrap();
    assert!(matches!(
        ready(&mut job).completion,
        Completion::Append(Ok(Append::Added { .. }))
    ));
    job.append_at(frame(8), (i64::MAX, 0)).unwrap();
    let rejected = ready(&mut job);
    assert!(matches!(
        rejected.completion,
        Completion::SpatialAppend(Ok(SpatialAppend::Rejected(_)))
    ));
    assert_eq!(rejected.snapshot.spatial_position, None);
    assert_eq!(rejected.snapshot.frame_count, 2);
    job.compose().unwrap();
    assert!(matches!(
        ready(&mut job).completion,
        Completion::Compose(Ok(Composite { height: 20, .. }))
    ));
    job.append_at(frame(8), (1, 4)).unwrap();
    assert_eq!(job.append_at(frame(8), (1, 4)), Err(DispatchError::Busy));
    let accepted = ready(&mut job);
    assert!(matches!(
        accepted.completion,
        Completion::SpatialAppend(Ok(SpatialAppend::Added { x: 1, y: 8 }))
    ));
    assert_eq!(accepted.snapshot.spatial_position, Some((1, 8)));
    assert_eq!(accepted.snapshot.frame_count, 3);
    job.capture().unwrap();
    let captured = ready(&mut job);
    assert!(matches!(
        captured.completion,
        Completion::SpatialAppend(Ok(SpatialAppend::Added { x: 1, y: 12 }))
    ));
    assert_eq!(captured.snapshot.frame_count, 4);
    job.preview().unwrap();
    assert!(matches!(
        ready(&mut job).completion,
        Completion::SpatialPreview(Ok(SpatialComposite {
            width: 5,
            height: 28,
            ..
        }))
    ));
    job.compose().unwrap();
    assert!(matches!(
        ready(&mut job).completion,
        Completion::SpatialCompose(Ok(SpatialComposite {
            width: 3,
            height: 28,
            x: 1,
            y: 0,
            ..
        }))
    ));
    job.append_at(frame(16), (0, 4)).unwrap();
    job.cancel();
    assert!(job.take_ready().is_none());
    assert_eq!(
        job.append_at(frame(16), (0, 4)),
        Err(DispatchError::Stopped)
    );
}
#[test]
fn scroll_commands_preserve_direction_and_single_flight_without_real_input() {
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorded = seen.clone();
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
    let mut job = SessionJob::start_with_actions(session, move |mode, _| {
        recorded.lock().unwrap().push(mode);
        Ok(frame(if mode == CaptureMode::ScrollForward {
            4
        } else {
            0
        }))
    })
    .unwrap();
    assert!(job.can_scroll());
    job.scroll_capture(true).unwrap();
    assert_eq!(job.scroll_capture(false), Err(DispatchError::Busy));
    assert!(matches!(
        ready(&mut job).completion,
        Completion::Append(Ok(Append::Added { origin: 4 }))
    ));
    job.scroll_capture(false).unwrap();
    assert!(matches!(
        ready(&mut job).completion,
        Completion::Append(Ok(Append::Added { origin: 0 }))
    ));
    assert_eq!(
        *seen.lock().unwrap(),
        [CaptureMode::ScrollForward, CaptureMode::ScrollBackward]
    );
}
#[test]
fn capture_source_runs_only_on_command_in_worker_and_recovers_after_failure() {
    let caller = std::thread::current().id();
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let seen = count.clone();
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
    let mut job = SessionJob::start_with_capture(session, move |_| {
        assert_ne!(std::thread::current().id(), caller);
        if seen.fetch_add(1, Ordering::AcqRel) == 0 {
            Err("injected capture failure".into())
        } else {
            Ok(frame(4))
        }
    })
    .unwrap();
    assert_eq!(count.load(Ordering::Acquire), 0);
    job.capture().unwrap();
    assert_eq!(job.capture(), Err(DispatchError::Busy));
    let failed = ready(&mut job);
    assert!(matches!(failed.completion, Completion::CaptureFailed(_)));
    assert_eq!(failed.snapshot.frame_count, 1);
    job.capture().unwrap();
    assert!(matches!(
        ready(&mut job).completion,
        Completion::Append(Ok(Append::Added { origin: 4 }))
    ));
    assert_eq!(count.load(Ordering::Acquire), 2);
    job.cancel();
    assert_eq!(job.capture(), Err(DispatchError::Stopped));
}
#[test]
fn source_less_job_rejects_capture_without_disrupting_other_commands() {
    let mut job = job();
    assert_eq!(job.capture(), Err(DispatchError::Unavailable));
    job.compose().unwrap();
    assert!(matches!(
        ready(&mut job).completion,
        Completion::Compose(Ok(_))
    ));
}
fn frame(origin: u8) -> Frame {
    let pixels: Vec<_> = (0..16).flat_map(|y| vec![origin + y; 16]).collect();
    Frame::new(4, 16, pixels.into()).unwrap()
}
fn job() -> SessionJob {
    SessionJob::start(
        Session::new(
            Axis::Vertical,
            frame(0),
            MatchOptions {
                minimum_overlap: 8,
                maximum_shift: 8,
                maximum_mean_error: 3.0,
            },
        )
        .unwrap(),
    )
    .unwrap()
}
fn ready(job: &mut SessionJob) -> Reply {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(reply) = job.take_ready() {
            return reply;
        }
        assert!(
            Instant::now() < deadline,
            "long capture worker did not reply"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn worker_is_single_flight_and_composes_after_acknowledgement() {
    let mut job = job();
    job.append(frame(4)).unwrap();
    assert_eq!(job.append(frame(8)), Err(DispatchError::Busy));
    assert_eq!(job.compose(), Err(DispatchError::Busy));
    let reply = ready(&mut job);
    assert!(matches!(
        reply.completion,
        Completion::Append(Ok(Append::Added { origin: 4 }))
    ));
    assert_eq!(reply.snapshot.frame_count, 2);
    assert!(!job.is_pending());
    assert!(job.take_ready().is_none());
    job.compose().unwrap();
    match ready(&mut job).completion {
        Completion::Compose(Ok(image)) => assert_eq!((image.width, image.height), (4, 20)),
        _ => panic!("expected composed image"),
    }
}
#[test]
fn cancellation_discards_even_completed_output_and_stops_idle_worker() {
    let mut job = job();
    job.compose().unwrap();
    // 将真实已完成结果放入受控回复槽，确定性覆盖取消发生在发布之前的竞态。
    let reply = ready(&mut job);
    let (sender, receiver) = mpsc::sync_channel(1);
    sender.send(reply).unwrap();
    job.receiver = Some(receiver);
    job.pending = true;
    job.cancel();
    assert!(job.take_ready().is_none());
    assert_eq!(job.compose(), Err(DispatchError::Stopped));
    let worker = job.worker.take().unwrap();
    worker.join().unwrap();
    assert!(job.is_finished());
}
#[test]
fn failed_append_keeps_worker_session_usable() {
    let mut job = job();
    job.append(Frame::new(1, 1, vec![0; 4].into()).unwrap())
        .unwrap();
    let reply = ready(&mut job);
    assert!(matches!(
        reply.completion,
        Completion::Append(Err(Error::Dimensions))
    ));
    assert_eq!(reply.snapshot.frame_count, 1);
    job.append(frame(4)).unwrap();
    assert!(matches!(
        ready(&mut job).completion,
        Completion::Append(Ok(Append::Added { origin: 4 }))
    ));
}
