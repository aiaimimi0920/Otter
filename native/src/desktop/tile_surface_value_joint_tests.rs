//! 原Tile Runtime/已paint HWND的离散值链；Select只注入选择回调，不向用户程序输入。
use super::*;
use std::{collections::BTreeSet, sync::Arc};
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
    let mut last_queue = serde_json::Value::Null;
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
                if done.insert("ready") {
                    f.mark(
                        "tile-values-ready.json",
                        json!({"nativePresentConfirmed":true}),
                    );
                }
                let scene: serde_json::Value = serde_json::from_slice(&art.snapshot.scene).unwrap();
                for (name, text) in [("flag-presented", "101"), ("choice-presented", "102")] {
                    if scene["children"][0]["props"]["text"] == text && done.insert(name) {
                        f.mark(
                            &format!("{name}.json"),
                            json!({"nativePresentConfirmed":true,"scene":scene}),
                        );
                    }
                }
            }
        }
        for (name, index) in [
            ("select-cancel", None),
            ("select-current", Some(0)),
            ("select-new", Some(1)),
        ] {
            if f.root.join(name).exists() && done.insert(name) {
                let keyboard = std::env::var_os("OTTER_TILE_KEYBOARD_JOINT").is_some();
                let accepted = if keyboard {
                    window.test_art_keyboard_select(index)
                } else {
                    window.test_art_select([10, 80], index)
                };
                assert_eq!(accepted, name == "select-new");
                f.mark(
                    &format!("{name}-done.json"),
                    json!({"uiQueueAccepted":accepted,"keyboardActivation":keyboard}),
                );
            }
        }
        if f.root.join("switch-on").exists()
            && !done.contains("switch-on")
            && if std::env::var_os("OTTER_TILE_KEYBOARD_JOINT").is_some() {
                window.test_art_keyboard_switch([10, 50])
            } else {
                window.test_art_click([10, 50])
            }
        {
            done.insert("switch-on");
            f.mark("switch-on-done.json", json!({"uiQueueAccepted":true}));
        }
        if f.root.join("tile-stop").exists() {
            runtime.request_stop();
        }
        if done.contains("switch-on") {
            let queue = window.test_art_keyboard_queue();
            if queue != last_queue {
                f.mark("tile-values-queue.json", queue.clone());
                last_queue = queue;
            }
        }
        if runtime.finished() {
            break;
        }
    }
    assert_eq!(
        done.len(),
        7,
        "Tile ended before value phases: {}",
        runtime.status()
    );
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
        json!({"threadsJoined":true,"controlsReleased":true,"artReleased":true,
        "discreteValues":true,"ordinaryOutputExe":false,"finalSwapChainReadback":false}),
    );
}
