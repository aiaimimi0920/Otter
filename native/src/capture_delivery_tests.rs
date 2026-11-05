use super::*;
use crate::{capture_color::ScRgb, capture_dynamic::Frame, capture_job::CaptureJob};
use std::{
    fs,
    time::{Duration, Instant},
};

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        Self(
            PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi")
                .join(format!("otter-delivery-{:?}", unsafe {
                    windows::Win32::System::Com::CoCreateGuid().unwrap()
                })),
        )
    }
    fn entries(&self) -> usize {
        fs::read_dir(self.0.join("clipboard_cache"))
            .unwrap()
            .count()
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let cache = self.0.join("clipboard_cache");
        if cache.exists() {
            for file in fs::read_dir(&cache).unwrap() {
                fs::remove_file(file.unwrap().path()).unwrap();
            }
            fs::remove_dir(cache).unwrap();
        }
        if self.0.exists() {
            fs::remove_dir(&self.0).unwrap();
        }
    }
}
fn outcome(hdr: bool) -> Outcome {
    let bytes: Vec<_> = [0x4000u16; 4]
        .iter()
        .flat_map(|n| n.to_le_bytes())
        .collect();
    let frame = ScRgb::new(&bytes, 1, 1, 8).unwrap();
    let cancel = AtomicBool::new(false);
    let preview = frame.to_sdr(320.0, 1.0, &cancel).unwrap();
    Outcome {
        frame: if hdr {
            Frame::Hdr {
                image: frame.to_hdr(0.01, 1000.0, 1.0, &cancel).unwrap(),
                preview,
            }
        } else {
            Frame::Sdr(preview)
        },
        reason: Some(Reason::AutoSdr),
        downgraded: false,
        display: None,
        diagnostic: Some("preserved diagnostic".into()),
    }
}
#[test]
fn delivery_preserves_preview_storage_metadata_and_publishes_only_after_acceptance() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let source = outcome(true);
    let Frame::Hdr { image, preview } = &source.frame else {
        unreachable!()
    };
    let weak = image.rgb16_be.downgrade();
    let pixels = preview.pixels.clone();
    let staged =
        PreparedCapture::prepare(source, &directory.0, "capture", Default::default(), &cancel)
            .unwrap();
    assert!(weak.upgrade().is_none());
    assert_eq!(directory.entries(), 2);
    let captured = staged.commit(&cancel, || true).unwrap();
    assert!(captured.preview.pixels.same_storage(&pixels));
    assert_eq!(captured.backend, "wgc");
    assert_eq!(captured.preview.pixels[0], 188);
    assert!(captured.original.unwrap().is_file());
    assert_eq!(directory.entries(), 3);
    assert_eq!(captured.reason, Some(Reason::AutoSdr));
    assert_eq!(captured.diagnostic.as_deref(), Some("preserved diagnostic"));
}
#[test]
fn rejected_owner_and_late_worker_cancellation_remove_unpublished_hdr() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let staged = PreparedCapture::prepare(
        outcome(true),
        &directory.0,
        "stale",
        Default::default(),
        &cancel,
    )
    .unwrap();
    assert!(staged.commit(&cancel, || false).is_err());
    assert_eq!(directory.entries(), 1);
    let profile = directory.0.clone();
    let mut job = CaptureJob::spawn(move |cancel| {
        PreparedCapture::prepare(
            outcome(true),
            &profile,
            "cancel",
            Default::default(),
            cancel,
        )
        .map_err(|error| error.to_string())
    })
    .unwrap();
    let start = Instant::now();
    while !job.worker.as_ref().unwrap().is_finished() {
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(directory.entries(), 2);
    job.cancel();
    assert!(job.take_ready().unwrap().is_err());
    assert_eq!(directory.entries(), 1);
}
#[test]
fn sdr_delivery_needs_no_cache_and_cancellation_inside_acceptance_is_terminal() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let staged = PreparedCapture::prepare(
        outcome(false),
        &directory.0,
        "sdr",
        Default::default(),
        &cancel,
    )
    .unwrap();
    assert!(staged.commit(&cancel, || true).unwrap().original.is_none());
    assert!(!directory.0.exists());
    let staged = PreparedCapture::prepare(
        outcome(false),
        &directory.0,
        "cancel",
        Default::default(),
        &cancel,
    )
    .unwrap();
    assert!(staged
        .commit(&cancel, || {
            cancel.store(true, Ordering::Release);
            true
        })
        .is_err());
    cancel.store(false, Ordering::Release);
    let mut source = outcome(false);
    let Frame::Sdr(preview) = &mut source.frame else {
        unreachable!()
    };
    preview.width = 2;
    assert!(
        PreparedCapture::prepare(source, &directory.0, "invalid", Default::default(), &cancel)
            .is_err()
    );
    assert!(!directory.0.exists());
}
