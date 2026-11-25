use super::*;
use std::sync::{atomic::AtomicUsize, mpsc};
use std::time::{Duration, Instant};
struct Source {
    next: Frame,
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
    calls: Arc<AtomicUsize>,
}
impl CaptureSource for Source {
    fn capture(&mut self, mode: CaptureMode, _: &AtomicBool) -> Result<Frame, String> {
        assert_eq!(mode, CaptureMode::Current);
        self.calls.fetch_add(1, Ordering::AcqRel);
        self.entered.send(()).unwrap();
        self.release
            .recv_timeout(Duration::from_secs(5))
            .map_err(|error| error.to_string())?;
        Ok(self.next.clone())
    }
}
fn frame(origin: u8) -> Frame {
    Frame::new(
        4,
        16,
        (0..16)
            .flat_map(|row| vec![row + origin; 16])
            .collect::<Vec<_>>()
            .into(),
    )
    .unwrap()
}
#[test]
fn detecting_job_is_single_flight_and_cancel_discards_late_reply_before_new_session() {
    let calls = Arc::new(AtomicUsize::new(0));
    let (entered, wait) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let mut old = SessionJob::start_detecting(
        frame(0),
        Source {
            next: frame(4),
            entered,
            release: gate,
            calls: calls.clone(),
        },
    )
    .unwrap();
    assert_eq!(old.snapshot().axis, None);
    old.capture().unwrap();
    wait.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(old.capture(), Err(DispatchError::Busy));
    assert_eq!(old.compose(), Err(DispatchError::Busy));
    assert_eq!(old.scroll_capture(true), Err(DispatchError::Unavailable));
    old.cancel();
    release.send(()).unwrap();
    assert!(old.take_ready().is_none());
    drop(old);
    assert_eq!(calls.load(Ordering::Acquire), 1);
    let (entered, _wait) = mpsc::channel();
    let (_release, gate) = mpsc::channel();
    let mut fresh = SessionJob::start_detecting(
        frame(20),
        Source {
            next: frame(24),
            entered,
            release: gate,
            calls,
        },
    )
    .unwrap();
    assert_eq!(fresh.snapshot().frame_count, 1);
    assert_eq!(fresh.snapshot().axis, None);
    fresh.compose().unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(reply) = fresh.take_ready() {
            assert_eq!(reply.snapshot.axis, None);
            assert_eq!(reply.snapshot.direction, None);
            let Completion::Compose(Ok(output)) = reply.completion else {
                panic!("expected final image");
            };
            assert_eq!(&output.pixels[..4], &[20; 4]);
            break;
        }
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(1));
    }
}
