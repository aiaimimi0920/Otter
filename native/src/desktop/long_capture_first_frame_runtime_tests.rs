//! Recording binding, first-frame, and occlusion gates; sample only owned real HWNDs.
use super::*;
use crate::long_capture::{Reply, SessionJob};
use std::sync::atomic::AtomicBool;
use windows::Win32::Graphics::Dwm::DwmFlush;

fn ready(job: &mut SessionJob) -> Reply {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(reply) = job.take_ready() {
            return reply;
        }
        assert!(Instant::now() < end, "capture reply timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[ignore = "explicit visible desktop gate, using only owned source and cover windows"]
fn foreign_root_cover_rejects_capture_and_scroll_until_removed() {
    unsafe {
        initialize_process_dpi();
        let mut fixture = fixture::Fixture::start();
        let region = fixture.region;
        let sample_start = || presented_sample(region, fixture::pixel(136, 136));
        let _ = sample_start();
        let (mut job, target) = long_capture_source::start_recording(region).unwrap();
        let reason = long_capture_source::scroll::TARGET_COVERED;
        for (partial, rect) in [(true, corner_cover(region)), (false, region)] {
            let cover = show_cover(rect, false);
            if partial {
                assert_center_is_source(region, fixture.hwnd);
                assert!(matches!(
                    long_capture_source::start_recording(region),
                    Err(error) if error == reason
                ));
            }
            job.capture().unwrap();
            let reply = ready(&mut job);
            assert!(
                matches!(reply.completion, Completion::CaptureFailed(error) if error == reason)
            );
            assert_eq!(reply.snapshot.frame_count, 1);
            assert_eq!(
                target.step(Axis::Vertical, true, &AtomicBool::new(false)),
                Err(reason.into())
            );
            drop(cover);
            DwmFlush().unwrap();
            let _ = sample_start();
        }
        fixture.wheel(Axis::Vertical, true);
        let _ = presented_sample(region, fixture::pixel(136, 144));
        job.capture().unwrap();
        let resumed = ready(&mut job);
        assert!(matches!(resumed.completion, Completion::Append(Ok(_))));
        assert_eq!(resumed.snapshot.frame_count, 2);
        drop(job);
        // Same selection, but displayed content has scrolled once. New recording accepts current pixels only, never reusing selection snapshots.
        let (mut fresh, _) = long_capture_source::start_recording(region).unwrap();
        fresh.compose().unwrap();
        let first = ready(&mut fresh);
        assert_eq!(first.snapshot.frame_count, 1);
        let Completion::Compose(Ok(output)) = first.completion else {
            panic!("expected one checked initial frame");
        };
        assert_eq!(
            (output.width, output.height),
            (region.width as u32, region.height as u32)
        );
        assert_eq!(
            output.pixels.len(),
            region.width as usize * region.height as usize * 4
        );
        for (index, pixel) in output.pixels.chunks_exact(4).enumerate() {
            let (x, y) = (index as i32 % region.width, index as i32 / region.width);
            assert_eq!(pixel, fixture::pixel(136 + x, 144 + y));
        }
    }
}
