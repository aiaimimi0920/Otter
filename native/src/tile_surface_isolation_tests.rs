//! 产品Runtime/真实TLS/WSS/自有窗口：挂住图片超过Live5秒，或在途停止/撤权，独立源像素oracle。
use super::*;
use crate::{device_authorization_lease::Grant, tile_runtime::Runtime, tile_wire::Endpoint};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
#[path = "tile_surface_isolation_peer.rs"]
mod peer;
#[test]
fn slow_art_image_does_not_starve_live_and_three_sources_resume_as_one_current_frame() {
    run(0);
}
#[test]
fn stop_and_revocation_cancel_pending_art_and_do_not_publish_its_late_image() {
    run(1);
    run(2);
}
fn run(ending: u8) {
    use windows::Win32::System::Com::*;
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
        .ok()
        .unwrap();
    struct Com;
    impl Drop for Com {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }
    let _com = Com;
    let (listener, client, config) = fixture();
    let origin = client.origin().to_owned();
    let mut grant = Some(Grant::new(session(&origin, u64::MAX)));
    let mut lease = || grant.as_mut().unwrap().lease(&origin, 0).unwrap();
    let (control, images, art) = (lease(), lease(), lease());
    let mut output = crate::tile_tests::output();
    output.x = -30000;
    output.y = -30000;
    output.width = 6;
    let flags = peer::Signals::new();
    let server = peer::serve(
        listener,
        config,
        Endpoint::image("device-1", &output),
        flags.clone(),
    );
    let runtime = Runtime::test_start(client, control, images, art, output.clone()).unwrap();
    let window = crate::desktop::tile_test_window(runtime.shared.clone(), &output);
    let end = Instant::now() + Duration::from_secs(12);
    let mut began = None;
    let mut first = None;
    let mut weak = Vec::new();
    let mut completed = false;
    loop {
        assert!(Instant::now() < end, "{}", runtime.status());
        assert!(!runtime.finished(), "{}", runtime.status());
        window.tick().unwrap();
        crate::desktop::tile_test_pump();
        let frame = runtime.shared.lock().presented_frame.clone();
        if let Some(frame) = frame {
            assert_eq!(frame.images.len(), 2);
            assert_eq!(&*frame.images["live:one"].pixels, &[17, 83, 211, 255]);
            let picture = frame
                .images
                .iter()
                .find(|(id, _)| id.starts_with("sha256:"))
                .unwrap()
                .1;
            assert_eq!(&*picture.pixels, &[0, 0, 255, 255].repeat(4));
            let art = &frame.arts["art"];
            if art.snapshot.revision == 1 && first.is_none() {
                first = Some(frame.images["live:one"].pixels.clone());
                weak.push(picture.pixels.downgrade());
                flags.painted.store(true, Ordering::Release);
            }
            if art.snapshot.revision == 2 {
                assert_eq!(ending, 0);
                assert!(!first
                    .take()
                    .unwrap()
                    .same_storage(&frame.images["live:one"].pixels));
                assert_eq!(&*art.images["root"].pixels, &[0, 255, 0, 255].repeat(4));
                weak.push(art.images["root"].pixels.downgrade());
                weak.push(frame.images["live:one"].pixels.downgrade());
                completed = true;
                break;
            }
        }
        if flags.image_started.load(Ordering::Acquire) {
            let start = *began.get_or_insert_with(Instant::now);
            if !flags.release.load(Ordering::Acquire) {
                assert!(
                    runtime.shared.lock().frame.is_none(),
                    "new Art revision must clear old complete frame"
                );
            }
            if ending != 0 && start.elapsed() > Duration::from_millis(150) {
                break;
            }
            if ending == 0 && start.elapsed() >= Duration::from_millis(5200) {
                assert!(
                    flags.pings.load(Ordering::SeqCst) >= 2,
                    "actual media polls must continue during held HTTP"
                );
                flags.release.store(true, Ordering::Release);
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(completed, ending == 0);
    assert_eq!(
        flags.connections.load(Ordering::SeqCst),
        1,
        "no Live reconnect to hide starvation"
    );
    if ending == 2 {
        drop(grant.take());
    } else {
        runtime.request_stop();
    }
    let stopped = Instant::now();
    while !runtime.finished() {
        assert!(
            stopped.elapsed() < Duration::from_secs(3),
            "Lease cancellation must wake held Art request"
        );
        crate::desktop::tile_test_pump();
        std::thread::sleep(Duration::from_millis(5));
    }
    flags.release.store(true, Ordering::Release);
    assert!(runtime.shared.lock().frame.is_none() && runtime.shared.lock().art.is_none());
    window.tick().unwrap();
    window.hide();
    drop(window);
    drop(runtime);
    drop(first);
    drop(grant);
    flags.stop.store(true, Ordering::Release);
    server.join().unwrap();
    assert!(!weak.is_empty() && weak.iter().all(|p| p.upgrade().is_none()));
}
