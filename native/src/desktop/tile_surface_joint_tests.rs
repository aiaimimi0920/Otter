//! Actual Wall LAN to mixed Art/image Tiles, resource updates/freeze/black frames/stop/revocation; owned offscreen windows only.
use super::*;
pub(super) fn run() {
    let _com = Apartment::new();
    let f = Fixture::new();
    let mut owner = super::authorize(&f.origin, Some(&f.profile)).unwrap();
    let lease = owner.lease(&f.origin, now_ms()).unwrap();
    let images = owner.lease(&f.origin, now_ms()).unwrap();
    let art = owner.lease(&f.origin, now_ms()).unwrap();
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
    let (mut first, mut updated, mut frozen, mut black, mut resumed) =
        (false, false, false, false, false);
    let mut weak = Vec::new();
    loop {
        f.pump();
        window.tick().unwrap();
        let (presented, frozen_now, black_now, black_confirmed) = {
            let s = runtime.shared.lock();
            (
                s.presented_frame
                    .clone()
                    .filter(|_| s.visible(Instant::now())),
                s.frozen,
                s.black,
                s.black_revision.is_some() && s.black_revision == s.black_confirmed,
            )
        };
        if let Some(frame) = presented {
            assert_eq!(frame.arts.len(), 1);
            assert_eq!(frame.images.len(), 1);
            let art = frame.arts.values().next().unwrap();
            assert_eq!(art.size, [640, 480]);
            if !first && !frozen_now {
                assert_eq!(art.snapshot.revision, 1);
                let size = art.size.map(|n| n as f32);
                let p = crate::surface_presentation::Presentation::new(
                    crate::surface_scene::Scene::parse(&art.snapshot.scene).unwrap(),
                    crate::surface_viewport::style(),
                    crate::surface_viewport::context(size),
                    size,
                )
                .unwrap();
                assert_eq!(p.geometry().nodes()[0].size(), [640., 480.]);
                first = true;
                f.mark("tile-art-first.json",json!({"snapshotRevision":1,"canonicalSize":[640,480],"mixed":true,"nativePresentConfirmed":true}));
            } else if first && !updated && art.snapshot.revision > 1 && !frozen_now {
                assert_eq!(art.images.len(), 1);
                let image = art.images.values().next().unwrap();
                assert_eq!(&*image.pixels, &[0, 255, 0, 255].repeat(4));
                weak.push(image.pixels.downgrade());
                updated = true;
                f.mark("tile-art-updated.json",json!({"snapshotRevision":art.snapshot.revision,"sourcePbgra":[0,255,0,255],"nativePresentConfirmed":true}));
            } else if updated && frozen_now && !frozen {
                frozen = true;
                f.mark(
                    "tile-art-frozen.json",
                    json!({"nativePresentConfirmed":true}),
                );
            } else if black && !black_now && !frozen_now && !resumed {
                resumed = true;
                f.mark(
                    "tile-art-resumed.json",
                    json!({"nativePresentConfirmed":true,"revision":frame.key.revision}),
                );
            }
        }
        if black_confirmed && !black {
            assert!(frozen);
            black = true;
            f.mark(
                "tile-art-black.json",
                json!({"nativeBlackPresentConfirmed":true}),
            );
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
    owner.close();
    while !owner.finished() {
        owner.poll();
        f.pump();
    }
    drop(owner);
    assert!(!weak.is_empty() && weak.iter().all(|w| w.upgrade().is_none()));
    f.mark("tile-output-complete.json",json!({"art":true,"revoked":revoked,"pixelsReleased":true,"threadsJoined":true,"ordinaryOutputExe":false,"finalSwapChainReadback":false}));
}
