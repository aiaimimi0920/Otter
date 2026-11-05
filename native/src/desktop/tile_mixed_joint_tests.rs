//! 三类来源同一实际呈现帧的独立像素oracle与生命周期；不采样用户桌面。
use super::*;
use crate::tile_shared::Frame;

fn colors(frame: &Frame) -> [[u8; 4]; 3] {
    assert_eq!((frame.size.width, frame.size.height), (6, 2));
    assert_eq!(frame.images.len(), 2);
    assert_eq!(frame.arts.len(), 1);
    let image = |kind| {
        let id = &frame
            .layout
            .placements
            .iter()
            .find(|p| p.source.kind == kind)
            .unwrap()
            .source
            .id;
        &frame.images[id]
    };
    let art = frame.arts.values().next().unwrap();
    assert_eq!(art.size, [640, 480]);
    assert_eq!(art.images.len(), 1);
    [
        image("image"),
        image("live"),
        art.images.values().next().unwrap(),
    ]
    .map(|image| {
        assert_eq!((image.width, image.height), (2, 2));
        let color: [u8; 4] = image.pixels[..4].try_into().unwrap();
        assert_eq!(&*image.pixels, &color.repeat(4));
        color
    })
}
pub(super) fn run() {
    let _com = Apartment::new();
    let f = Fixture::new();
    let mut owner = super::authorize(&f.origin, Some(&f.profile)).unwrap();
    let mut output = crate::tile_tests::output();
    output.x = -30000;
    output.y = -30000;
    output.width = 6;
    let runtime = Runtime::start(
        config(&f.origin, Some(&f.profile)).unwrap(),
        owner.lease(&f.origin, now_ms()).unwrap(),
        owner.lease(&f.origin, now_ms()).unwrap(),
        owner.lease(&f.origin, now_ms()).unwrap(),
        output.clone(),
    )
    .unwrap();
    let window = crate::desktop::tile_test_window(runtime.shared.clone(), &output);
    let revoked = std::env::var_os("OTTER_TILE_JOINT_REVOKE").is_some();
    let lifecycle = f.root.join("tile-mixed-lifecycle").exists();
    let (mut first, mut updated, mut frozen, mut black, mut resumed) =
        (false, false, false, false, false);
    let mut weak = Vec::new();
    let mut frozen_frame = None;
    // Oracle为独立手写PBGRA，非调用生产转换来产生expected。
    let before = [[0, 0, 255, 255], [17, 83, 211, 255], [0, 255, 0, 255]];
    let after = [[255, 0, 0, 255], [193, 29, 71, 255], [255, 255, 0, 255]];
    let expected = if lifecycle { before } else { after };
    let mut status = None;
    loop {
        f.pump();
        window.tick().unwrap();
        let s = runtime.shared.lock();
        let current = (
            s.status,
            s.desired.as_ref().map(|k| k.revision),
            s.frozen,
            s.black,
            s.frame.is_some(),
            s.presented_frame.is_some(),
            s.art.is_some(),
        );
        if status != Some(current) {
            f.mark("tile-mixed-status.json",json!({"status":current.0,"revision":current.1,"frozen":current.2,"black":current.3,"frameReady":current.4,"presented":current.5,"artReady":current.6}));
            status = Some(current);
        }
        if let Some(frame) = s
            .presented_frame
            .clone()
            .filter(|_| s.visible(Instant::now()))
        {
            let actual = colors(&frame);
            if weak.len() + 3 <= 128 {
                weak.extend(frame.images.values().map(|image| image.pixels.downgrade()));
                weak.extend(
                    frame
                        .arts
                        .values()
                        .flat_map(|art| art.images.values().map(|image| image.pixels.downgrade())),
                );
            }
            if !first && !s.frozen && actual == before {
                assert_eq!(frame.key.revision, 2);
                first = true;
                f.mark(
                    "tile-mixed-first.json",
                    json!({"pbgra":actual,"revision":2,"nativePresentConfirmed":true}),
                );
            } else if first && !updated && !s.frozen && actual == after {
                assert_eq!(frame.key.revision, 3);
                updated = true;
                f.mark(
                    "tile-mixed-updated.json",
                    json!({"pbgra":actual,"revision":3,"nativePresentConfirmed":true}),
                );
            } else if (updated || (lifecycle && first)) && s.frozen && !frozen {
                assert_eq!(actual, expected);
                frozen_frame = Some(frame.clone());
                frozen = true;
                f.mark(
                    "tile-mixed-frozen.json",
                    json!({"nativePresentConfirmed":true}),
                );
            } else if frozen && s.frozen {
                assert!(std::sync::Arc::ptr_eq(
                    frozen_frame.as_ref().unwrap(),
                    &frame
                ));
            } else if black && !s.black && !s.frozen && !resumed {
                assert_eq!(actual, expected);
                resumed = true;
                f.mark(
                    "tile-mixed-resumed.json",
                    json!({"pbgra":actual,"nativePresentConfirmed":true}),
                );
            }
        }
        if s.black_revision.is_some() && s.black_revision == s.black_confirmed && !black {
            assert!(frozen && s.frame.is_none() && s.presented_frame.is_none());
            frozen_frame = None;
            black = true;
            f.mark(
                "tile-mixed-black.json",
                json!({"nativeBlackPresentConfirmed":true}),
            );
        }
        drop(s);
        if f.root.join("tile-stop").exists() {
            assert!(!revoked && if lifecycle { resumed } else { updated });
            runtime.request_stop();
        }
        if runtime.finished() {
            break;
        }
    }
    assert!(first && (updated || lifecycle));
    if lifecycle {
        assert!(frozen && black && resumed);
    }
    window.tick().unwrap();
    window.hide();
    drop(window);
    drop(frozen_frame);
    let shared = std::sync::Arc::downgrade(&runtime.shared);
    drop(runtime);
    owner.close();
    while !owner.finished() {
        owner.poll();
        f.pump();
    }
    drop(owner);
    assert!(shared.upgrade().is_none());
    assert!(!weak.is_empty() && weak.iter().all(|w| w.upgrade().is_none()));
    f.mark("tile-output-complete.json",json!({"threeSources":true,"changed":updated,"lifecycle":lifecycle,"revoked":revoked,"pixelsReleased":true,"sharedReleased":true,"threadsJoined":true,"ordinaryOutputExe":false,"finalSwapChainReadback":false}));
}
