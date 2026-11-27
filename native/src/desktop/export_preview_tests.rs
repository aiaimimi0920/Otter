use super::*;
use std::time::{Duration, Instant};

fn source() -> Source {
    Source {
        image: crate::desktop::image::Image {
            width: 2,
            height: 2,
            pixels: vec![0, 0, 64, 128].repeat(4).into(),
        },
        editor: Default::default(),
        opacity: 1.,
        style: Default::default(),
    }
}

fn finish(preview: &mut Preview) -> Pixels {
    let deadline = Instant::now() + Duration::from_secs(15);
    while preview.poll() {
        assert!(Instant::now() < deadline, "preview did not settle");
        std::thread::yield_now();
    }
    preview.result.take().unwrap().unwrap()
}

#[test]
fn export_preview_reinstated_format_rejects_old_result_and_new_modal_waits_for_retirement() {
    let mut a = Preview::new(source());
    let first = Format::WebpLossy(25);
    a.request(first);
    // Hold the worker inside an uninterruptible codec step deterministically.
    let (release, wait) = std::sync::mpsc::sync_channel(1);
    let slot = Slot::acquire().unwrap();
    let flag = Arc::new(AtomicBool::new(false));
    let job = std::thread::spawn(move || {
        let _slot = slot;
        wait.recv_timeout(Duration::from_secs(15)).unwrap();
        Ok(vec![99].into())
    });
    a.job = Some((first, flag.clone(), job));
    a.request(Format::WebpLossy(99));
    a.request(first);
    assert!(flag.load(Ordering::Acquire));
    assert!(a.poll() && a.result.is_none());
    release.send(()).unwrap();
    let current = finish(&mut a);
    assert_eq!(&current[8..12], b"WEBP");
    assert_eq!(
        &*current,
        &*a.source.encode(first, &AtomicBool::new(false)).unwrap()
    );

    let old = Slot::acquire().unwrap();
    let (release, wait) = std::sync::mpsc::sync_channel(1);
    let cancel = Arc::new(AtomicBool::new(false));
    a.job = Some((
        first,
        cancel.clone(),
        std::thread::spawn(move || {
            let _slot = old;
            wait.recv_timeout(Duration::from_secs(15)).unwrap();
            Ok(vec![17].into())
        }),
    ));
    drop(a);
    assert!(cancel.load(Ordering::Acquire));
    let mut b = Preview::new(source());
    assert!(b.poll() && b.job.is_none() && b.result.is_none());
    release.send(()).unwrap();
    assert_eq!(&finish(&mut b)[..8], b"\x89PNG\r\n\x1a\n");
}
