//! 保存单槽释放后多 Live 意图按原 owner 串行完成；只依赖实际完成/内部唤醒消息。
use super::*;
unsafe fn recycle(hwnd: HWND) {
    SendMessageW(hwnd, WM_CLOSE, None, None);
}
unsafe fn dispatch_one(app: &mut Application, deadline: Instant) {
    let mut message = MSG::default();
    loop {
        if PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            assert_ne!(message.message, WM_QUIT);
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
            app.pending_windows(false).unwrap();
            tick(app);
            return;
        }
        assert!(
            Instant::now() < deadline,
            "no internal wake for retained Live intent"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn multiple_freeze_and_recycle_intents_survive_automatic_save_and_need_no_external_input() {
    unsafe {
        let environment = Environment::new();
        for freeze in [true, false] {
            let directory = Directory::new();
            let mut app = app(&environment);
            let (a, first) = live(&mut app, true);
            let (b, second) = live(&mut app, true);
            install(&mut app, &directory, false);
            tick(&mut app);
            ready(&app);
            drain(); // 排除保存完成消息，接下来必须由归还槽的 WM_NULL 自行续进。
            if freeze {
                queue(a);
                queue(b);
            } else {
                recycle(a);
                recycle(b);
            }
            app.process_live(false).unwrap();
            assert!(IsWindow(Some(a)).as_bool() && IsWindow(Some(b)).as_bool());
            tick(&mut app); // 丢弃已过期自动候选，同时排队内部续进。
            assert!(!directory.path().exists());
            let end = Instant::now() + Duration::from_secs(5);
            while IsWindow(Some(a)).as_bool() || IsWindow(Some(b)).as_bool() {
                dispatch_one(&mut app, end);
            }
            assert!(app.live_sessions.iter().all(|session| !session.is_alive()));
            if freeze {
                assert!(app.documents.iter().any(|doc| doc.state.unit_id == first));
                assert!(app.documents.iter().any(|doc| doc.state.unit_id == second));
            } else {
                let original = &app.restored_session.as_ref().unwrap().original;
                let library = crate::session_library::decode(
                    original,
                    crate::session_library::Kind::RecycleBin,
                )
                .unwrap();
                assert_eq!(library.entries().len(), 2);
            }
            app.shutdown_live().unwrap();
            settle_auto(&mut app);
            assert!(directory.path().exists(), "checkpoint must remain enabled");
        }
    }
}

#[test]
fn second_freeze_request_is_not_consumed_while_first_worker_is_owned() {
    unsafe {
        let environment = Environment::new();
        let mut app = app(&environment);
        let (a, first) = live(&mut app, true);
        let (b, second) = live(&mut app, true);
        queue(a);
        app.process_live_freeze(false).unwrap();
        assert!(app.live_freeze_job.is_some());
        queue(b);
        app.process_live_freeze(false).unwrap();
        let end = Instant::now() + Duration::from_secs(5);
        while IsWindow(Some(a)).as_bool() || IsWindow(Some(b)).as_bool() {
            dispatch_one(&mut app, end);
        }
        for id in [first, second] {
            assert!(app.documents.iter().any(|doc| doc.state.unit_id == id));
        }
        app.shutdown_live().unwrap();
    }
}
#[test]
fn retired_live_owner_is_excluded_before_reap_but_registered_closed_owner_still_rejects_save() {
    unsafe {
        let environment = Environment::new();
        let mut app = app(&environment);
        let (_, retired) = live(&mut app, true);
        let (_, current) = live(&mut app, true);
        let index = app
            .live_sessions
            .iter()
            .position(|session| *session.unit_id() == retired)
            .unwrap();
        app.live_sessions[index].close_for_recycle().unwrap();
        assert!(app.live_sessions[index].belongs_to(&app.workspace));
        assert!(!app.live_sessions[index].is_alive());
        // 尚未发布图移除时不能静默漏存异常关闭的当前 Unit。
        assert!(app.workspace.contains(&retired));
        assert!(app.capture_session_snapshot().is_err());
        app.retain_workspace_owners();
        assert!(!app.workspace.contains(&retired));
        assert!(app.workspace.contains(&current));
        assert_eq!(
            app.live_sessions.len(),
            2,
            "no poll/reap before snapshot proof"
        );
        assert!(save_snapshot::automatic_ready(&app).unwrap());
        let (receipt, data) = app.capture_session_snapshot().unwrap();
        assert!(
            receipt.matches(&app),
            "capture/matches must use the same member set"
        );
        let encoded = data.encode(&AtomicBool::new(false)).unwrap();
        let document = crate::session_document::decode(&encoded.bytes).unwrap();
        assert!(!document
            .windows
            .iter()
            .any(|window| window.id == retired.as_str()));
        assert!(document
            .windows
            .iter()
            .any(|window| window.id == current.as_str()));
        app.shutdown_live().unwrap();
    }
}
