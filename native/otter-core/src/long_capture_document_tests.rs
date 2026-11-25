//! Desktop output admission across the worker, including rollback of a pending viewfinder move.
use super::*;
use std::sync::Mutex;

fn frame(width: u32, height: u32, x0: u32, y0: u32) -> Frame {
    let pixels = (0..height)
        .flat_map(|y| {
            (0..width).flat_map(move |x| {
                let seed = (x + x0).wrapping_mul(7919) ^ (y + y0).wrapping_mul(104729);
                let v = (seed ^ (seed >> 16)).wrapping_mul(0x7feb352d);
                [(v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, 255]
            })
        })
        .collect::<Vec<_>>();
    Frame::new(width, height, pixels.into()).unwrap()
}
fn options() -> MatchOptions {
    MatchOptions {
        minimum_overlap: 4,
        maximum_shift: 960,
        maximum_mean_error: 3.0,
    }
}

#[test]
fn document_job_accepts_the_exact_axis_limit_and_preflights_scroll_before_input() {
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let make = |delta| match axis {
            Axis::Horizontal => frame(1024, 12, delta, 0),
            Axis::Vertical => frame(12, 1024, 0, delta),
        };
        let session = Session::new(axis, make(0), options())
            .unwrap()
            .with_output(OutputTarget::Document)
            .unwrap();
        let mut job = SessionJob::start_with_actions(session, |_, _| {
            panic!("size admission must happen before the source moves")
        })
        .unwrap();
        for i in 1..=16 {
            job.append(make(i * 960)).unwrap();
            assert!(
                matches!(ready(&mut job).completion, Completion::Append(Ok(Append::Added { origin })) if origin == i64::from(i) * 960)
            );
        }
        let before = job.snapshot();
        job.scroll_capture(true).unwrap();
        assert!(matches!(
            ready(&mut job).completion,
            Completion::Append(Err(Error::DocumentLimit))
        ));
        // Passive capture has no preflight, but must still roll back the actual candidate.
        job.append(make(16 * 960 + 1)).unwrap();
        assert!(matches!(
            ready(&mut job).completion,
            Completion::Append(Err(Error::DocumentLimit))
        ));
        assert_eq!(job.snapshot(), before);
        job.append(make(16 * 960)).unwrap();
        assert!(matches!(
            ready(&mut job).completion,
            Completion::Append(Ok(Append::Duplicate))
        ));
        job.compose().unwrap();
        let Completion::Compose(Ok(output)) = ready(&mut job).completion else {
            panic!("previous result must still compose")
        };
        let (width, height) = match axis {
            Axis::Horizontal => (16384, 12),
            Axis::Vertical => (12, 16384),
        };
        assert_eq!((output.width, output.height), (width, height));
        assert_eq!(&*output.pixels, frame(width, height, 0, 0).bytes());
    }
}

#[derive(Default)]
struct Position {
    accepted: (i32, i32),
    pending: Option<(i32, i32)>,
    finish: Vec<bool>,
}
struct Viewfinder(Arc<Mutex<Position>>, u32);
impl CaptureSource for Viewfinder {
    fn capture(&mut self, _: CaptureMode, _: &AtomicBool) -> Result<Frame, String> {
        let (x, y) = self.0.lock().unwrap().accepted;
        Ok(frame(12, self.1, x as u32, y as u32))
    }
    fn can_move(&self) -> bool {
        true
    }
    fn prepare_move(&mut self, (dx, dy): (i32, i32), _: &AtomicBool) -> Result<Frame, String> {
        let mut state = self.0.lock().unwrap();
        let (x, y) = (state.accepted.0 + dx, state.accepted.1 + dy);
        state.pending = Some((x, y));
        Ok(frame(12, self.1, x as u32, y as u32))
    }
    fn finish_move(&mut self, accepted: bool) {
        let mut state = self.0.lock().unwrap();
        let candidate = state.pending.take().unwrap();
        state.finish.push(accepted);
        if accepted {
            state.accepted = candidate;
        }
    }
}

#[test]
fn detecting_job_rejects_oversized_first_frame_and_first_motion_without_locking_axis() {
    let state = Arc::new(Mutex::new(Position::default()));
    assert!(SessionJob::start_detecting_for(
        frame(12, 16385, 0, 0),
        Viewfinder(state.clone(), 16385),
        OutputTarget::Document
    )
    .is_err());
    let mut job = SessionJob::start_detecting_for(
        frame(12, 16384, 0, 0),
        Viewfinder(state, 16384),
        OutputTarget::Document,
    )
    .unwrap();
    let before = job.snapshot();
    job.append(frame(12, 16384, 0, 1)).unwrap();
    assert!(matches!(
        ready(&mut job).completion,
        Completion::Append(Err(Error::DocumentLimit))
    ));
    assert_eq!(job.snapshot(), before);
    job.compose().unwrap();
    assert!(matches!(
        ready(&mut job).completion,
        Completion::Compose(Ok(Composite {
            width: 12,
            height: 16384,
            ..
        }))
    ));
}

#[test]
fn viewfinder_rejects_oversized_crop_but_accepts_oversized_sparse_envelope() {
    let state = Arc::new(Mutex::new(Position::default()));
    let mut job = SessionJob::start_detecting_for(
        frame(12, 16380, 0, 0),
        Viewfinder(state.clone(), 16380),
        OutputTarget::Document,
    )
    .unwrap();
    job.move_capture(0, 4).unwrap();
    assert_eq!(ready(&mut job).snapshot.spatial_position, Some((0, 4)));
    let before = job.snapshot();
    job.move_capture(0, 1).unwrap();
    assert!(matches!(
        ready(&mut job).completion,
        Completion::SpatialAppend(Err(Error::DocumentLimit))
    ));
    assert_eq!(job.snapshot(), before);
    assert_eq!(state.lock().unwrap().accepted, (0, 4));
    job.capture().unwrap();
    assert!(matches!(
        ready(&mut job).completion,
        Completion::SpatialAppend(Ok(SpatialAppend::Duplicate))
    ));
    job.move_capture(1, 1).unwrap();
    let accepted = ready(&mut job);
    assert!(matches!(
        accepted.completion,
        Completion::SpatialAppend(Ok(SpatialAppend::Added { x: 1, y: 5 }))
    ));
    assert_eq!(state.lock().unwrap().finish, [true, false, true]);
    job.compose().unwrap();
    let Completion::SpatialCompose(Ok(output)) = ready(&mut job).completion else {
        panic!("expected largest complete crop")
    };
    assert_eq!(
        (output.x, output.y, output.width, output.height),
        (0, 5, 13, 16379)
    );
    // The wider common strip wins by area over the taller 12-pixel strip.
    assert_eq!(&*output.pixels, frame(13, 16379, 0, 5).bytes());
}
