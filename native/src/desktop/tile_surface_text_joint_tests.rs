//! 真正Tile Runtime/原生EDIT→串行worker→Loom；消息只发给自有离屏窗口。
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
    let mut last_trace = serde_json::Value::Null;
    loop {
        f.pump();
        window.tick().unwrap();
        if done.contains("edit") {
            let trace = window.test_art_keyboard_queue();
            if trace != last_trace {
                eprintln!("tile-text-joint-state: {trace}");
                last_trace = trace;
            }
        }
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
                        "tile-text-ready.json",
                        json!({"nativePresentConfirmed":true}),
                    );
                }
                if scene["children"][0]["props"]["text"] == "submitted" && done.insert("result") {
                    f.mark(
                        "tile-text-presented.json",
                        json!({"scene":scene,"nativePresentConfirmed":true}),
                    );
                }
            }
        }
        if f.root.join("text-request.json").exists() && done.insert("edit") {
            let request: serde_json::Value =
                serde_json::from_slice(&std::fs::read(f.root.join("text-request.json")).unwrap())
                    .unwrap();
            let keyboard = std::env::var_os("OTTER_TILE_KEYBOARD_JOINT").is_some();
            let scrolling = request["scrolling"].as_bool().unwrap_or(false);
            let scrollbar = request["scrollbar"].as_str();
            window.test_art_text(
                [20, 60],
                request["text"].as_str().unwrap(),
                request["ime"].as_bool().unwrap(),
                !keyboard && scrollbar.is_none(),
            );
            assert!(if keyboard {
                window.test_art_text_keyboard_submit()
            } else {
                if let Some(mode) = scrollbar {
                    assert!(window.test_art_scrollbar_to_end(mode));
                } else if scrolling {
                    assert!(window.test_art_scroll([20, 20], 1, 10000.));
                }
                window.test_art_click(if scrolling { [20, 445] } else { [20, 125] })
            });
            f.mark("tile-text-submitted.json",json!({"actualOwnEditNotifications":true,"syntheticImeEdges":request["ime"],"noUserInputInjection":true,"keyboardActivation":keyboard,"scrolling":scrolling,"scrollbar":scrollbar}));
        }
        if f.root.join("tile-stop").exists() {
            runtime.request_stop();
        }
        if runtime.finished() {
            break;
        }
    }
    assert_eq!(done.len(), 3, "{}", runtime.status());
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
    f.mark("tile-output-complete.json",json!({"threadsJoined":true,"artReleased":true,"ordinaryOutputExe":false,"finalSwapChainReadback":false}));
}
