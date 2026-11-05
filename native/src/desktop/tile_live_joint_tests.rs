//! 真实Source/Loom LAN到原生Tile的混合布局；独立固定像素oracle，非最终交换链读回。
use super::*;
use crate::{
    tile_live_packet::WallMediaFormat,
    tile_live_socket::{Interrupts, Receiver},
};
pub(super) fn run() {
    let _com = Apartment::new();
    let f = Fixture::new();
    let mut owner = super::authorize(&f.origin, Some(&f.profile)).unwrap();
    let lease = owner.lease(&f.origin, now_ms()).unwrap();
    let images = owner.lease(&f.origin, now_ms()).unwrap();
    let art = owner.lease(&f.origin, now_ms()).unwrap();
    let png_lease = owner.lease(&f.origin, now_ms()).unwrap();
    let client = png_lease
        .prepare_client(&config(&f.origin, Some(&f.profile)).unwrap())
        .unwrap();
    let mut output = crate::tile_tests::output();
    output.x = -30000;
    output.y = -30000;
    output.width = 4;
    let runtime = Runtime::start(
        config(&f.origin, Some(&f.profile)).unwrap(),
        lease,
        images,
        art,
        output.clone(),
    )
    .unwrap();
    let window = crate::desktop::tile_test_window(runtime.shared.clone(), &output);
    let revoked = std::env::var_os("OTTER_TILE_JOINT_REVOKE").is_some();
    let mut first = false;
    let mut updated = false;
    let mut frozen = false;
    let mut black = false;
    let mut resumed = false;
    let mut first_pixel = None;
    let mut weak = Vec::new();
    loop {
        f.pump();
        window.tick().unwrap();
        let snapshot = {
            let s = runtime.shared.lock();
            s.presented_frame
                .clone()
                .filter(|_| s.visible(Instant::now()))
        };
        if let Some(frame) = snapshot {
            let s = runtime.shared.lock();
            assert_eq!(frame.images.len(), 2);
            let live_id = &frame
                .layout
                .placements
                .iter()
                .find(|p| p.source.kind == "live")
                .unwrap()
                .source
                .id;
            let image = &frame.images[live_id];
            assert_eq!((image.width, image.height), (2, 2));
            assert_eq!(&*image.pixels, &[17, 83, 211, 255].repeat(4));
            if weak.len() < 128 {
                weak.push(image.pixels.downgrade());
            }
            if !first && !s.frozen {
                drop(s);
                let interrupts = Interrupts::default();
                let mut png = Receiver::connect(
                    &client,
                    &png_lease,
                    &frame.key,
                    live_id,
                    WallMediaFormat::Png,
                    &interrupts,
                )
                .unwrap();
                let start = Instant::now();
                loop {
                    png.poll(|| false).unwrap();
                    if let Some(decoded) = png
                        .queue
                        .advance(runtime.shared.lock().clock.estimate(Instant::now()))
                    {
                        assert_eq!((decoded.image.width, decoded.image.height), (2, 2));
                        assert_eq!(&*decoded.image.pixels, &[17, 83, 211, 255].repeat(4));
                        weak.push(decoded.image.pixels.downgrade());
                        break;
                    }
                    assert!(start.elapsed() < Duration::from_secs(3));
                    f.pump();
                }
                drop(png);
                drop(interrupts);
                first_pixel = Some(image.pixels.clone());
                first = true;
                f.mark("tile-live-first.json",json!({"revision":frame.key.revision,"mixedImages":2,"rawPbgra":[17,83,211,255],"pngPbgra":[17,83,211,255],"nativePresentConfirmed":true}));
            } else if first
                && !updated
                && !s.frozen
                && first_pixel
                    .as_ref()
                    .is_some_and(|p| !p.same_storage(&image.pixels))
            {
                updated = true;
                first_pixel = None;
                f.mark(
                    "tile-live-updated.json",
                    json!({"actualNewDecodedFrame":true,"nativePresentConfirmed":true}),
                );
            } else if updated && s.frozen && !frozen {
                frozen = true;
                f.mark(
                    "tile-live-frozen.json",
                    json!({"nativePresentConfirmed":true}),
                );
            } else if black && !s.black && !s.frozen && !resumed {
                resumed = true;
                f.mark(
                    "tile-live-resumed.json",
                    json!({"revision":frame.key.revision,"nativePresentConfirmed":true}),
                );
            }
        }
        {
            let s = runtime.shared.lock();
            if s.black_revision.is_some() && s.black_revision == s.black_confirmed && !black {
                assert!(frozen);
                black = true;
                f.mark(
                    "tile-live-black.json",
                    json!({"nativeBlackPresentConfirmed":true}),
                );
            }
        }
        if f.root.join("tile-stop").exists() {
            assert!(resumed || revoked);
            runtime.request_stop();
        }
        if runtime.finished() {
            break;
        }
    }
    assert!(first && updated);
    if !revoked {
        assert!(frozen && black && resumed);
    }
    window.tick().unwrap();
    window.hide();
    drop(window);
    drop(runtime);
    drop(first_pixel);
    png_lease.revoke();
    drop(png_lease);
    owner.close();
    while !owner.finished() {
        owner.poll();
        f.pump();
    }
    drop(owner);
    assert!(!weak.is_empty() && weak.iter().all(|w| w.upgrade().is_none()));
    f.mark("tile-output-complete.json",json!({"live":true,"revoked":revoked,"pixelsReleased":true,"threadsJoined":true,"ordinaryOutputExe":false,"finalSwapChainReadback":false}));
}
