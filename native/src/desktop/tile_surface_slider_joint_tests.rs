//! 原Tile/已paint HWND实际消息→唯一worker；拖动跨本手势权威value变化，独立Loom父门核业务。
use super::*;
use std::{collections::BTreeSet, sync::Arc};
use windows::Win32::UI::WindowsAndMessaging::{WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE};
pub(super) fn run() {
    let _com = Apartment::new();
    let f = Fixture::new();
    let mut owner = super::authorize(&f.origin, Some(&f.profile)).unwrap();
    let lease = owner.lease(&f.origin, now_ms()).unwrap();
    let images = owner.lease(&f.origin, now_ms()).unwrap();
    let art = owner.lease(&f.origin, now_ms()).unwrap();
    let mut output = crate::tile_tests::output();
    (output.x, output.y, output.width, output.height) = (-30000, -30000, 640, 480);
    let runtime = Runtime::start(
        config(&f.origin, Some(&f.profile)).unwrap(),
        lease,
        images,
        art,
        output.clone(),
    )
    .unwrap();
    let window = crate::desktop::tile_test_window(runtime.shared.clone(), &output);
    let mut done = BTreeSet::new();
    let mut weak = None;
    loop {
        f.pump();
        window.tick().unwrap();
        {
            let s = runtime.shared.lock();
            if let Some(frame) = s
                .presented_frame
                .as_ref()
                .filter(|_| s.visible(Instant::now()))
            {
                let art = frame.arts.values().next().unwrap();
                weak = Some(Arc::downgrade(art));
                let scene: serde_json::Value = serde_json::from_slice(&art.snapshot.scene).unwrap();
                if done.insert("ready") {
                    f.mark(
                        "tile-slider-ready.json",
                        json!({"nativePresentConfirmed":true}),
                    );
                }
                for (name, value) in [
                    ("slider-first-presented", 50),
                    ("slider-final-presented", 100),
                ] {
                    if scene["children"][1]["props"]["value"].as_f64() == Some(f64::from(value))
                        && done.insert(name)
                    {
                        f.mark(
                            &format!("{name}.json"),
                            json!({"scene":scene,"nativePresentConfirmed":true}),
                        );
                    }
                }
            }
        }
        if f.root.join("slider-start").exists() && done.insert("start") {
            window.test_art_slider_message(WM_LBUTTONDOWN, [5, 60]);
            window.test_art_slider_message(WM_MOUSEMOVE, [150, 60]);
            assert!(window.test_art_slider_active());
            f.mark("slider-started.json", json!({"actualOwnHwndMessages":true}));
        }
        if f.root.join("slider-finish").exists() && done.insert("finish") {
            // 已接受的第一input改变了props.value，不能退休仍有效的同一拖动。
            assert!(window.test_art_slider_active());
            let keyboard = std::env::var_os("OTTER_TILE_KEYBOARD_JOINT").is_some();
            if keyboard {
                window.test_art_keyboard_slider_end();
            } else {
                for x in 151..295 {
                    window.test_art_slider_message(WM_MOUSEMOVE, [x, 60]);
                }
                window.test_art_slider_message(WM_LBUTTONUP, [295, 60]);
            }
            assert!(!window.test_art_slider_active());
            f.mark("slider-finished.json",json!({"sourceValueChangedDuringDrag":true,"captureReleased":true,"sampleCount":if keyboard {1}else{144},"keyboardActivation":keyboard}));
        }
        if f.root.join("tile-stop").exists() {
            runtime.request_stop();
        }
        if runtime.finished() {
            break;
        }
    }
    assert_eq!(done.len(), 5, "{}", runtime.status());
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
    assert!(weak.unwrap().upgrade().is_none());
    f.mark(
        "tile-output-complete.json",
        json!({"threadsJoined":true,"artReleased":true,
        "sourceResolutionSlider":true,"ordinaryOutputExe":false,"finalSwapChainReadback":false}),
    );
}
