//! 隐藏 Application 消费最后通知后，用生产 deadline 回收扩展任务；watchdog 只报告失败。
use super::super::execution_test_pump::Pump;
use super::*;
use crate::desktop::{
    capability_job::Source,
    extension_catalog_job::tests::SERIAL,
    extension_command_route::OCR_OVERLAY,
    extension_commands,
    loom_execution_tasks::reap_fixture::{Gate, WAIT},
};
use crate::loom_extension_session::test_peer as peer;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc,
};

unsafe fn consume_last_wake(app: &mut Application) {
    let mut message = MSG::default();
    assert!(PeekMessageW(&mut message, None, WM_APP + 92, WM_APP + 92, PM_REMOVE).as_bool());
    app.pending_windows(false).unwrap();
    // 清除创建/请求留下的消息，不能把旧 WM_NULL 当作截止唤醒。
    for _ in 0..256 {
        if !PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            return;
        }
        assert_ne!(message.message, WM_QUIT);
        if !message.hwnd.is_invalid() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    panic!("extension test message queue did not drain");
}

#[test]
fn catalog_revoke_and_reap_continue_after_the_last_notification() {
    let _serial = SERIAL.lock().unwrap();
    let _apartment = crate::desktop::Apartment::new().unwrap();
    unsafe {
        let instance = crate::desktop::workspace_window_fixture::register();
        let document = DocumentWindow::create(instance, state(), false).unwrap();
        let mut app = Application::new(instance, document);
        let wake = app.execution_wake.callback();
        let terminal_wake = wake.clone();
        let gate = Gate::new(move || terminal_wake());
        let tail = gate.notify.clone();
        let (first, noticed) = mpsc::channel();
        let count = AtomicUsize::new(0);
        app.extensions.test_set_notify(Arc::new(move || {
            if count.fetch_add(1, Ordering::AcqRel) == 0 {
                wake();
                first.send(()).unwrap();
            } else {
                tail();
            }
        }));
        let (close, waiting) = mpsc::channel();
        let (identity, server) = peer::peer(move |p| {
            peer::handshake(p, peer::handshake_data());
            waiting.recv_timeout(WAIT).unwrap();
            p.close(None).unwrap();
        });
        app.extensions.test_start(Source::Fixed(identity));
        noticed.recv_timeout(WAIT).unwrap();
        consume_last_wake(&mut app);
        assert!(app.extensions.view.borrow().snapshot.is_some());
        assert!(app.extensions.deadline().is_none());
        close.send(()).unwrap();
        gate.noticed.recv_timeout(WAIT).unwrap();
        consume_last_wake(&mut app);
        assert!(app.extensions.view.borrow().snapshot.is_none());
        assert!(app.extensions.deadline().is_some());
        let pump = Pump::new();
        while pump.step(&mut app) != WM_NULL {}
        assert!(app.extensions.deadline().is_some(), "tail is still blocked");
        gate.release.send(()).unwrap();
        app.extensions.test_wait_finished();
        while app.extensions.deadline().is_some() {
            pump.step(&mut app);
        }
        assert!(app.extensions.view.borrow().snapshot.is_none());
        assert!(gate.noticed.try_recv().is_err());
        server.join().unwrap();
    }
}

#[test]
fn failed_and_cancelled_commands_settle_after_the_last_notification() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    unsafe {
        let instance = crate::desktop::workspace_window_fixture::register();
        let document = DocumentWindow::create(instance, state(), false).unwrap();
        let mut app = Application::new(instance, document);
        // 复用宿主，避免两次销毁末窗口的 WM_QUIT 污染第二轮等待。
        for cancelled in [false, true] {
            let source = crate::loom_extension_snapshot::tests::fixture();
            app.extensions.view.borrow_mut().snapshot = Some(Arc::new(
                crate::loom_extension_snapshot::Snapshot::parse(
                    &serde_json::to_vec(&source).unwrap(),
                )
                .unwrap(),
            ));
            let wake = app.execution_wake.callback();
            let gate = Gate::new(move || wake());
            app.extension_commands.test_set_notify(gate.notify.clone());
            let (authorized, waiting) = mpsc::channel();
            let (finish, finishing) = mpsc::channel();
            let (identity, server) = peer::peer(move |p| {
                peer::handshake(p, peer::handshake_data());
                assert_eq!(peer::read(p)["method"], "loom.extension.command.authorize");
                authorized.send(()).unwrap();
                finishing.recv_timeout(WAIT).unwrap();
                if cancelled {
                    assert!(p.read().is_err());
                } else {
                    p.close(None).unwrap();
                }
            });
            let doc = &app.documents[0];
            extension_commands::request_command(doc.window.0, &doc.state, OCR_OVERLAY).unwrap();
            app.extension_commands
                .process_with(&app.documents, &app.naming_document, || {
                    Source::Fixed(identity)
                });
            waiting.recv_timeout(WAIT).unwrap();
            assert!(app.extension_commands.deadline().is_none());
            assert!(extension_commands::busy(&app.documents[0].state));
            if cancelled {
                app.extensions.view.borrow_mut().snapshot = None;
                app.pending_windows(false).unwrap();
            }
            finish.send(()).unwrap();
            gate.noticed.recv_timeout(WAIT).unwrap();
            consume_last_wake(&mut app);
            assert!(extension_commands::busy(&app.documents[0].state));
            assert!(app.extension_commands.deadline().is_some());
            let pump = Pump::new();
            while pump.step(&mut app) != WM_NULL {}
            assert!(extension_commands::busy(&app.documents[0].state));
            gate.release.send(()).unwrap();
            app.extension_commands.test_wait_finished();
            while extension_commands::busy(&app.documents[0].state) {
                pump.step(&mut app);
            }
            assert!(app.extension_commands.deadline().is_none());
            assert!(app.documents[0]
                .state
                .extension_document
                .borrow()
                .store
                .items()
                .is_empty());
            assert!(gate.noticed.try_recv().is_err());
            server.join().unwrap();
        }
    }
}
