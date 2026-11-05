use super::*;
#[derive(Default)]
struct State {
    position: (i32, i32),
    pending: Option<(i32, i32)>,
    commits: Vec<bool>,
}
struct Source(Arc<std::sync::Mutex<State>>);
impl CaptureSource for Source {
    fn can_move(&self) -> bool {
        true
    }
    fn capture(&mut self, _: CaptureMode, _: &AtomicBool) -> Result<Frame, String> {
        Ok(frame(self.0.lock().unwrap().position.1 as u8))
    }
    fn prepare_move(&mut self, movement: (i32, i32), cancel: &AtomicBool) -> Result<Frame, String> {
        let mut state = self.0.lock().unwrap();
        let next = (state.position.0 + movement.0, state.position.1 + movement.1);
        state.pending = Some(next);
        if movement.0 == 99 {
            return Err("injected capture failure".into());
        }
        if movement.0 == 98 {
            cancel.store(true, Ordering::Release);
        }
        Ok(frame(next.1 as u8))
    }
    fn finish_move(&mut self, accepted: bool) {
        let mut state = self.0.lock().unwrap();
        let candidate = state.pending.take().unwrap();
        if accepted {
            state.position = candidate;
        }
        state.commits.push(accepted);
    }
}
fn source_job(state: Arc<std::sync::Mutex<State>>) -> SessionJob {
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
    SessionJob::start_with_source(session, Source(state)).unwrap()
}
#[test]
fn source_region_commits_only_after_accepted_frames_and_rolls_back_every_failure() {
    let state = Arc::new(std::sync::Mutex::new(State::default()));
    let mut job = source_job(state.clone());
    assert!(job.can_move());
    assert!(!job.can_scroll());
    job.move_capture(0, 0).unwrap();
    let unchanged = ready(&mut job);
    assert!(matches!(
        unchanged.completion,
        Completion::Append(Ok(Append::Duplicate))
    ));
    assert_eq!(unchanged.snapshot.spatial_position, None);
    job.move_capture(99, 0).unwrap();
    assert!(matches!(
        ready(&mut job).completion,
        Completion::CaptureFailed(_)
    ));
    job.move_capture(100, 4).unwrap();
    let rejected = ready(&mut job);
    assert!(matches!(
        rejected.completion,
        Completion::SpatialAppend(Ok(SpatialAppend::Rejected(_)))
    ));
    assert_eq!(rejected.snapshot.spatial_position, None);
    assert_eq!(state.lock().unwrap().position, (0, 0));
    job.move_capture(1, 4).unwrap();
    assert_eq!(job.move_capture(0, 4), Err(DispatchError::Busy));
    assert_eq!(ready(&mut job).snapshot.spatial_position, Some((1, 4)));
    assert_eq!(state.lock().unwrap().position, (1, 4));
    job.capture().unwrap();
    assert!(matches!(
        ready(&mut job).completion,
        Completion::SpatialAppend(Ok(SpatialAppend::Duplicate))
    ));
    job.move_capture(100, 4).unwrap();
    ready(&mut job);
    assert_eq!(state.lock().unwrap().position, (1, 4));
    job.move_capture(0, 4).unwrap();
    assert_eq!(ready(&mut job).snapshot.spatial_position, Some((1, 8)));
    let state = state.lock().unwrap();
    assert_eq!(state.position, (1, 8));
    assert!(state.pending.is_none());
    assert_eq!(state.commits, [true, false, false, true, false, true]);
}
#[test]
fn cancellation_after_preparation_discards_pending_region_and_publishes_nothing() {
    let state = Arc::new(std::sync::Mutex::new(State::default()));
    let mut job = source_job(state.clone());
    job.move_capture(98, 4).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !job.is_finished() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(job.take_ready().is_none());
    assert_eq!(job.move_capture(0, 4), Err(DispatchError::Stopped));
    let state = state.lock().unwrap();
    assert_eq!(state.position, (0, 0));
    assert!(state.pending.is_none());
    assert_eq!(state.commits, [false]);
}
#[test]
fn existing_callback_source_does_not_claim_viewfinder_capability() {
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
    let mut job = SessionJob::start_with_capture(session, |_| Ok(frame(4))).unwrap();
    assert!(!job.can_move());
    assert_eq!(job.move_capture(1, 1), Err(DispatchError::Unavailable));
    job.compose().unwrap();
    assert!(matches!(
        ready(&mut job).completion,
        Completion::Compose(Ok(_))
    ));
}
