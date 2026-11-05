//! 无handler本地变值→同源换快照/新权威→原按钮真实Art正式结果；只操作自有离屏HWND。
use super::*;
use std::{collections::BTreeSet, sync::Arc};
pub(super) fn run() {
    let _com = Apartment::new();
    let f = Fixture::new();
    let mut owner = super::authorize(&f.origin, Some(&f.profile)).unwrap();
    let a = owner.lease(&f.origin, now_ms()).unwrap();
    let b = owner.lease(&f.origin, now_ms()).unwrap();
    let c = owner.lease(&f.origin, now_ms()).unwrap();
    let mut output = crate::tile_tests::output();
    (output.x, output.y, output.width, output.height) = (-30000, -30000, 640, 480);
    let runtime = Runtime::start(
        config(&f.origin, Some(&f.profile)).unwrap(),
        a,
        b,
        c,
        output.clone(),
    )
    .unwrap();
    let window = crate::desktop::tile_test_window(runtime.shared.clone(), &output);
    let mut done = BTreeSet::new();
    let mut weak = None;
    let keyboard = std::env::var_os("OTTER_TILE_KEYBOARD_JOINT").is_some();
    loop {
        f.pump();
        window.tick().unwrap();
        let scene = {
            let s = runtime.shared.lock();
            s.presented_frame
                .as_ref()
                .filter(|_| s.visible(Instant::now()))
                .map(|frame| {
                    let art = frame.arts.values().next().unwrap();
                    weak = Some(Arc::downgrade(art));
                    serde_json::from_slice::<serde_json::Value>(&art.snapshot.scene).unwrap()
                })
        };
        if let Some(scene) = scene {
            let label = scene["children"][0]["props"]["text"].as_str().unwrap();
            if done.insert("ready") {
                f.mark("local-ready.json", json!({"nativePresentConfirmed":true}));
            }
            for (stage, expected) in [
                (
                    "same-source",
                    json!({"switch":false,"select":0,"slider":100.}),
                ),
                ("new-source", json!({"switch":true,"select":1,"slider":20.})),
            ] {
                if label == stage && done.insert(stage) {
                    let values = window.test_art_local_values();
                    assert_eq!(values, expected);
                    f.mark(
                        &format!("local-{stage}.json"),
                        json!({"values":values,"scene":scene,"nativePresentConfirmed":true}),
                    );
                }
            }
            if label == "submitted" && done.insert("result") {
                f.mark(
                    "local-result.json",
                    json!({"scene":scene,"nativePresentConfirmed":true}),
                );
            }
        }
        if f.root.join("local-edit").exists() && done.insert("edit") {
            window.test_art_text([20, 60], "仅本地🙂", false, true);
            if keyboard {
                assert!(!window.test_art_keyboard_switch([20, 110]));
                assert!(!window.test_art_keyboard_select(Some(1)));
                window.test_art_keyboard_slider_end();
            } else {
                assert!(!window.test_art_click([20, 110]));
                assert!(!window.test_art_select([20, 145], Some(1)));
                window.test_art_slider_message(WM_LBUTTONDOWN, [295, 185]);
                window.test_art_slider_message(WM_LBUTTONUP, [295, 185]);
            }
            assert_eq!(
                window.test_art_local_values(),
                json!({"switch":true,"select":1,"slider":100.})
            );
            if keyboard {
                assert!(!window.test_art_keyboard_switch([20, 110]));
                assert!(!window.test_art_keyboard_select(Some(0)));
            } else {
                assert!(!window.test_art_click([20, 110]));
                assert!(!window.test_art_select([20, 145], None));
                assert!(!window.test_art_select([20, 145], Some(0)));
            }
            let values = window.test_art_local_values();
            assert_eq!(values, json!({"switch":false,"select":0,"slider":100.}));
            assert_eq!(runtime.shared.lock().actions.test_offered, 0);
            assert!(runtime.shared.lock().actions.pending.is_empty());
            f.mark(
                "local-edited.json",
                json!({"values":values,"queuedActions":0,"keyboard":keyboard}),
            );
        }
        if f.root.join("local-submit").exists() && done.insert("submit") {
            let accepted = if keyboard {
                window.test_art_local_submit()
            } else {
                window.test_art_click([20, 230])
            };
            f.mark(
                "local-submitted.json",
                json!({"uiQueueAccepted":accepted,"queue":window.test_art_keyboard_queue()}),
            );
            assert!(accepted);
        }
        if f.root.join("tile-stop").exists() {
            runtime.request_stop();
        }
        if runtime.finished() {
            break;
        }
    }
    assert_eq!(done.len(), 6, "{}", runtime.status());
    window.tick().unwrap();
    assert_eq!(window.test_art_local_values(), json!({}));
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
        json!({"threadsJoined":true,"artReleased":true,"localValuesCleared":true,
        "ordinaryOutputExe":false,"finalSwapChainReadback":false}),
    );
}
