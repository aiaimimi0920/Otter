//! 实际Tile Runtime/已paint Presentation/自有HWND输入与原生选择接缝；管理员不代发event。
use super::*;
use std::{collections::BTreeMap, sync::Arc};
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
    let mut done = std::collections::BTreeSet::new();
    let mut weak = None;
    let mut diagnostic = serde_json::Value::Null;
    let mut not_ready = BTreeMap::new();
    loop {
        f.pump();
        window.tick().unwrap();
        if let Some(request) = window.test_identification_presented() {
            if done.insert("identified") {
                f.mark(
                    "tile-actions-identified.json",
                    json!({"requestId":request,"nativeVisiblePaint":true}),
                );
            }
        }
        if f.root.join("tile-actions-dismiss").exists() && done.insert("dismissed") {
            window.test_identification_escape();
            assert!(window.alive());
        }
        let presented = {
            let s = runtime.shared.lock();
            s.presented_frame
                .as_ref()
                .filter(|_| s.visible(Instant::now()))
                .map(|frame| frame.arts.values().next().unwrap().clone())
        };
        // 先释放共享锁，再解析场景和发布文件信号；回执仍只描述实际呈现的快照。
        if let Some(art) = presented {
            if done.contains("dismissed") && done.insert("resumed") {
                f.mark("tile-actions-resumed.json", json!({"view":art.view,"generation":art.generation,"nativePresentConfirmed":true}));
            }
            weak = Some(Arc::downgrade(&art));
            if done.insert("ready") {
                f.mark(
                    "tile-actions-ready.json",
                    json!({"nativePresentConfirmed":true,"canonicalSize":art.size}),
                );
            }
            if !done.contains("result") {
                let scene: serde_json::Value = serde_json::from_slice(&art.snapshot.scene).unwrap();
                if scene["children"][0]["props"]["text"] == "101" && done.insert("result") {
                    f.mark(
                        "tile-actions-result-presented.json",
                        json!({"snapshotText":"101","nativePresentConfirmed":true}),
                    );
                }
            }
        }
        for name in ["click-reject", "click-approve", "click-slow"] {
            if f.root.join(name).exists() && !done.contains(name) {
                let accepted = window.test_art_click(if name == "click-slow" {
                    [20, 170]
                } else {
                    [20, 60]
                });
                // 测试信号不是用户手势受理回执；仅未进入UI队列时等下一实际paint，不重放已受理写。
                if accepted {
                    done.insert(name);
                    f.mark(
                        &format!("{name}-queued.json"),
                        json!({"uiQueueAccepted":true}),
                    );
                } else {
                    let current = {
                        let s = runtime.shared.lock();
                        (
                            s.actions.notice,
                            s.visible(Instant::now()),
                            s.generation,
                            s.presented.as_ref().map(|v| v.0.revision),
                        )
                    };
                    if not_ready.insert(name, current) != Some(current) {
                        let (notice, visible, generation, revision) = current;
                        f.mark(&format!("{name}-not-ready.json"), json!({"uiQueueAccepted":false,"notice":notice,"nativeVisible":visible,"generation":generation,"frameRevision":revision}));
                    }
                }
            }
        }
        for (name, choice) in [
            ("choice-none", None),
            (
                "choice-reject",
                Some(crate::surface_confirmation::Choice::Reject),
            ),
            (
                "choice-approve",
                Some(crate::surface_confirmation::Choice::Approve),
            ),
            (
                "choice-approve-slow",
                Some(crate::surface_confirmation::Choice::Approve),
            ),
            ("cancel-running", None),
        ] {
            if f.root.join(name).exists() && !done.contains(name) && window.test_art_choice(choice)
            {
                done.insert(name);
                f.mark(
                    &format!("{name}-done.json"),
                    json!({"explicitTestChoice":true}),
                );
            }
        }
        let (notice, offered, dispatch, states) = {
            let s = runtime.shared.lock();
            (
                s.actions.notice,
                s.actions.test_offered,
                s.actions.test_dispatch.clone(),
                s.actions.states.values().cloned().collect::<Vec<_>>(),
            )
        };
        let current = json!({"notice":notice,"offered":offered,"dispatch":dispatch,
            "states":states.iter().map(|s|json!({"revision":s.snapshot.revision,"sequence":s.sequence,
                "pending":s.activity.pending.len(),"confirmations":s.activity.confirmations.len(),"failure":s.activity.failure})).collect::<Vec<_>>()});
        if current != diagnostic {
            f.mark("tile-action-client-status.json", current.clone());
            diagnostic = current;
        }
        if f.root.join("tile-stop").exists() {
            runtime.request_stop();
        }
        if runtime.finished() {
            break;
        }
    }
    assert_eq!(
        done.len(),
        13,
        "Tile ended before expected phases: {}",
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
    f.mark("tile-output-complete.json",json!({"threadsJoined":true,"controlsReleased":true,"artReleased":true,"ordinaryOutputExe":false}));
}
