//! 显式 WGC→普通 Loom→回环 Art 持续负载；只用新 profile 和自有源，不代普通 UI 验收。
use super::*;
use crate::desktop::loom_result_state::Status;
use std::path::PathBuf;
use windows::Win32::Graphics::Gdi::{InvalidateRect, UpdateWindow};

unsafe fn pump(app: &mut Application) {
    let mut message = MSG::default();
    while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
        let _ = TranslateMessage(&message);
        DispatchMessageW(&message);
    }
    app.pending_windows(false).unwrap();
}
unsafe fn until(app: &mut Application, stage: &str, ready: impl Fn(&Application) -> bool) {
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        pump(app);
        if ready(app) {
            return;
        }
        assert!(Instant::now() < end, "Live Art load stage: {stage}");
        std::thread::sleep(Duration::from_millis(5));
    }
}
unsafe fn catalog(app: &Application, manifest: PathBuf) {
    assert!(manifest.is_absolute() && manifest.is_file());
    let identity = crate::loom_discovery::Config::from_lookup(|name| {
        (name == "LOOM_MANIFEST_PATH").then(|| manifest.clone().into_os_string())
    })
    .load()
    .unwrap();
    let handshake =
        crate::loom_handshake_exchange::fetch(&identity, &AtomicBool::new(false)).unwrap();
    assert_eq!(handshake.arts().len(), 1);
    assert_eq!(
        handshake.capability(0).unwrap().id,
        "owned-live-art-load-544"
    );
    app.catalog.borrow_mut().loaded =
        Some(std::rc::Rc::new(crate::desktop::capability_job::Loaded {
            identity,
            handshake,
        }));
}

#[test]
#[ignore = "explicit 600s real WGC and ordinary Loom Art load; requires isolated owned manifest"]
fn actual_wgc_ordinary_loom_art_sustained_load() {
    let input = PathBuf::from(std::env::var_os("OTTER_LIVE_ART_LOAD_INPUT").unwrap());
    assert!(input.is_absolute() && std::fs::metadata(&input).unwrap().len() <= 4096);
    let data: serde_json::Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let manifest = PathBuf::from(data["manifest"].as_str().unwrap());
    let root = input.parent().unwrap();
    unsafe {
        let environment = Environment::new();
        let _input = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
        let mut app = environment.app();
        // 测试库没有普通 main 的恢复入口；复用现有真实恢复 child 测试启动方式。
        app.live_visibility = crate::live_visibility_owner::Owner::fixture();
        catalog(&app, manifest);
        // STATIC 的白色实心客户区提供独立像素 oracle；不注入输入或取得前台。
        let source = crate::desktop::Window(
            CreateWindowExW(
                WS_EX_NOACTIVATE,
                w!("STATIC"),
                w!("Owned Live Art load source 544"),
                WINDOW_STYLE(WS_POPUP.0 | 6),
                24,
                24,
                128,
                96,
                None,
                None,
                Some(environment.instance),
                None,
            )
            .unwrap(),
        );
        let _ = ShowWindow(source.0, SW_SHOWNOACTIVATE);
        let _ = UpdateWindow(source.0);
        windows::Win32::Graphics::Dwm::DwmFlush().unwrap();
        let target = crate::live_window_target::Target::bind(source.0).unwrap();
        let original = crate::live_window_target::snapshot::Snapshot::read(&target).unwrap();
        // fixture 已预建 journal，必须先登记源；空 owner 的正常 poll 会开始退休。
        let session = app.start_live_window(target.clone(), None).unwrap();
        app.admit_live(session, false).unwrap();
        until(&mut app, "initial WGC frame", |app| {
            app.live_sessions.len() == 1 && app.live_sessions[0].snapshot_frame().unwrap().is_some()
        });
        let unit = app.live_sessions[0].unit_id().clone();
        assert!(add(&mut app, &unit).unwrap());
        assert_eq!(app.documents.len(), 2);
        assert_eq!(app.workspace.links().len(), 1);
        until(&mut app, "initial Art completion", |app| {
            app.documents[1].state.execution_results.borrow().status() == Status::Completed
        });
        let started = Instant::now();
        for cycle in 0..120 {
            let size = if cycle % 2 == 0 {
                [160, 112]
            } else {
                [128, 96]
            };
            SetWindowPos(
                source.0,
                None,
                0,
                0,
                size[0],
                size[1],
                SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER,
            )
            .unwrap();
            assert!(InvalidateRect(Some(source.0), None, true).as_bool());
            let _ = UpdateWindow(source.0);
            until(&mut app, "new WGC size", |app| {
                app.live_sessions[0]
                    .snapshot_frame()
                    .unwrap()
                    .is_some_and(|frame| {
                        [frame.resource.width, frame.resource.height]
                            == [size[0] as u32, size[1] as u32]
                    })
            });
            let stamp = app.documents[1]
                .state
                .execution_results
                .borrow()
                .output_stamp();
            let child = &app.documents[1];
            crate::desktop::execution_start::request(child.window.0, &child.state).unwrap();
            until(&mut app, "new formal Art output", |app| {
                let results = app.documents[1].state.execution_results.borrow();
                assert_ne!(results.status(), Status::Failed, "Art execution failed");
                results.status() == Status::Completed && !stamp.matches(&results)
            });
            let results = app.documents[1].state.execution_results.borrow();
            // 正式图片按现有图契约提供两个共享像素端口，不能误断言只有 wire 原端口。
            assert_eq!(results.ports().len(), 2);
            let image = results.image_source("output_image").unwrap();
            let image = image.image.image();
            assert!(results.ports().iter().all(|port| {
                matches!(port.name.as_str(), "output" | "output_image")
                    && image
                        .pixels
                        .same_storage(&port.value.image().unwrap().pixels)
            }));
            assert_eq!(
                [image.width, image.height],
                [size[0] as u32, size[1] as u32]
            );
            assert_eq!(
                image
                    .pixels
                    .chunks_exact(4)
                    .find(|pixel| *pixel != [255; 4]),
                None
            );
            drop(results);
            // 不跨周期保留旧帧/输出；原单槽准备/执行 owner 必须在每次请求后回收。
            until(&mut app, "workers reclaimed", |app| {
                app.input_jobs.is_empty() && app.execution_jobs.is_empty()
            });
            assert!(app.live_sessions[0].is_alive());
            assert!(!app.live_sessions[0].input_enabled());
            let next = started + Duration::from_secs((cycle + 1) * 5);
            while Instant::now() < next {
                pump(&mut app);
                std::thread::sleep(Duration::from_millis(20));
            }
            assert!(
                started.elapsed() < Duration::from_secs(630),
                "load batch deadline"
            );
            println!("native-live-art-load: cycle={cycle} width={} height={} formal=true workers-reclaimed=true elapsed-ms={}", size[0], size[1], started.elapsed().as_millis());
        }
        app.shutdown_live().unwrap();
        assert!(app.live_sessions.is_empty());
        assert!(app.live_visibility.finished());
        let restored = crate::live_window_target::snapshot::Snapshot::read(&target).unwrap();
        assert_eq!(restored.extended_style, original.extended_style);
        assert_eq!(restored.visible, original.visible);
        assert_eq!(
            restored.placement.rcNormalPosition,
            original.placement.rcNormalPosition
        );
        app.shutdown();
        std::fs::write(
            root.join("load-complete.json"),
            serde_json::to_vec(&serde_json::json!({
                "cycles":120,"seconds":started.elapsed().as_secs_f64(),"realWgc":true,
                "ordinaryLoom":true,"formalPixelsVerified":true,"sourceRestored":true,
                "ordinaryOtter":false,"ordinaryUiAcceptance":false
            }))
            .unwrap(),
        )
        .unwrap();
    }
}
