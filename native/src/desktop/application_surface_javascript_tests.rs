//! 真实 Application 授权/attach/取码/隐藏 DOM/激活/关闭；显式测试报告，不开放默认 JS。
use super::*;
#[path = "application_surface_javascript_peer.rs"]
pub(super) mod peer;
fn wait_script(app: &Application, reply: bool) {
    let handle = app
        .surface_windows
        .javascript_wait_handles()
        .nth(usize::from(reply))
        .unwrap();
    assert_eq!(
        unsafe {
            windows::Win32::System::Threading::WaitForSingleObject(
                HANDLE(handle.as_raw_handle()),
                8000,
            )
        },
        WAIT_OBJECT_0
    );
}
fn run(advertised: bool, revoke: bool, corrupt: bool) {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let _com = Apartment::new().unwrap();
    let (files, store, _) = Files::new();
    let (listener, client, server, pem) = fixture_with_pem();
    let origin = format!(
        "https://localhost:{}",
        listener.local_addr().unwrap().port()
    );
    std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
    let (download_tx, download_rx) = std::sync::mpsc::channel();
    let server = std::thread::spawn(move || {
        peer::serve(listener, server, advertised, revoke, corrupt, download_rx)
    });
    unsafe {
        let (_class, mut app) = fixture::application();
        app.authorization.borrow_mut().origin = Some(origin.clone());
        app.authorization_owner
            .start(crate::device_authorization_runtime::Task::Renew { client, store })
            .unwrap();
        assert_eq!(
            windows::Win32::System::Threading::WaitForSingleObject(
                HANDLE(
                    app.authorization_owner
                        .wait_handles()
                        .next()
                        .unwrap()
                        .as_raw_handle()
                ),
                5000
            ),
            WAIT_OBJECT_0
        );
        assert!(matches!(
            app.authorization_owner.poll().as_slice(),
            [crate::device_authorization_owner::Event::Authorized { .. }]
        ));
        let now = crate::desktop::surface_windows::now_ms().unwrap();
        let request = crate::surface_attach_runtime::Request::new(
            "art:test".into(),
            "node:test".into(),
            if advertised {
                peer::report()
            } else {
                crate::desktop::surface_capabilities::report().unwrap()
            },
            crate::live_relay_http::Persistence::Temporary,
        )
        .unwrap();
        app.remote_sessions
            .start_attach(
                fixture::config(&origin, files.root()).unwrap(),
                app.authorization_owner.lease(&origin, now).unwrap(),
                std::sync::Arc::new(request),
            )
            .unwrap();
        let reads = std::cell::Cell::new(0);
        let mut config = |requested: &str| {
            reads.set(reads.get() + 1);
            fixture::config(requested, files.root())
        };
        fixture::wait(&app);
        if advertised {
            assert!(app.remote_sessions.poll_surfaces().is_empty());
            app.admit_surface_results().unwrap();
            assert_eq!(app.surface_windows.wait_handles().len(), 1);
            assert_eq!(app.surface_windows.wait_budget(), 3); // Runtime 尚未启动也必须预留。
            assert!(crate::desktop::application::live::fits_wait_budget(
                15,
                app.surface_windows.wait_budget()
            ));
            assert!(!crate::desktop::application::live::fits_wait_budget(
                15,
                app.surface_windows.wait_budget() + 3
            ));
        }
        let admitted = app.poll_remote_live_with(false, &mut config);
        if !advertised {
            assert!(admitted.is_err()); // HTTP owner 已拒绝未协商 runtime；不得绕过成可显示场景。
            app.admit_surface_results().unwrap();
            assert!(app.surface_windows.iter_mut().next().is_none());
            assert!(
                app.surface_windows
                    .take_failure()
                    .unwrap()
                    .outcome
                    .uncertain
            );
            assert_eq!(reads.get(), 0);
        } else {
            admitted.unwrap();
            let parent = {
                let window = app.surface_windows.iter_mut().next().unwrap();
                assert!(window.is_alive() && !window.is_shown());
                assert_eq!(
                    window.binding().lifecycle(),
                    crate::surface_live_binding::Lifecycle::Mounted
                );
                assert!(window.lifecycle().is_some());
                let parent = window.edit_test_parent();
                // 唯一可见路径也只移到屏幕外且不激活；不向用户全局发送输入。
                SetWindowPos(
                    parent,
                    None,
                    -32000,
                    -32000,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
                )
                .unwrap();
                parent
            };
            assert_eq!(app.remote_sessions.wait_handles().count(), 0);
            assert_eq!(app.surface_windows.wait_handles().len(), 3);
            download_tx.send(()).unwrap();
            wait_script(&app, !corrupt);
            app.poll_remote_live_with(false, &mut config).unwrap();
            if !corrupt {
                let probe = {
                    let window = app.surface_windows.iter_mut().next().unwrap();
                    assert_eq!(window.javascript_text_for_test(), "原生🙂:42:0");
                    assert!(!window.is_shown());
                    assert!(window.lifecycle().is_none());
                    window.javascript_probe().unwrap()
                };
                fixture::wait(&app);
                app.poll_remote_live_with(false, &mut config).unwrap();
                if app
                    .surface_windows
                    .iter_mut()
                    .next()
                    .unwrap()
                    .javascript_pending_for_test()
                {
                    wait_script(&app, true);
                }
                app.poll_remote_live_with(true, &mut config).unwrap();
                {
                    let window = app.surface_windows.iter_mut().next().unwrap();
                    assert!(window.is_shown() && window.lifecycle().unwrap().confirmed_active());
                    assert_eq!(window.snapshot_revision(), 1);
                    assert!(window
                        .sync_input(
                            true,
                            true,
                            crate::desktop::surface_windows::now_ms().unwrap()
                        )
                        .unwrap());
                    assert!(!window.has_pending_click());
                    SendMessageW(parent, WM_PAINT, None, None);
                    assert!(window.take_error().is_none());
                }
                assert_eq!(reads.get(), 2); // source + active，空闲不得重复取码/配置。
                if revoke {
                    app.authorization_owner.close();
                    wait_script(&app, false); // 没有 Application refresh 也已结束 child。
                    assert!(probe.exited());
                    app.poll_remote_live_with(false, |_| panic!("revoked JS read configuration"))
                        .unwrap();
                } else {
                    app.surface_windows.close_all().unwrap();
                    assert!(app.surface_windows.take_closed().is_none()); // 未收割线程不许移除 owner。
                    assert_eq!(app.surface_windows.wait_handles().len(), 1);
                    wait_script(&app, false);
                    app.poll_remote_live_with(false, &mut config).unwrap();
                    fixture::wait(&app);
                    app.poll_remote_live_with(false, |_| panic!("closed JS restarted"))
                        .unwrap();
                    assert!(probe.exited());
                }
            } else {
                fixture::wait(&app); // 摘要失败只走 inactive，不得 active 或显示 authority scene。
                app.poll_remote_live_with(false, |_| panic!("corrupt script retried"))
                    .unwrap();
            }
            let window = app.surface_windows.iter_mut().next().unwrap();
            assert!(!window.is_alive() && !window.is_shown() && !IsWindow(Some(parent)).as_bool());
            assert!(window.javascript_finished());
            assert!(!window.lifecycle().unwrap().disposed());
            assert_eq!(
                window.binding().lifecycle(),
                if revoke {
                    crate::surface_live_binding::Lifecycle::Active
                } else {
                    crate::surface_live_binding::Lifecycle::Inactive
                }
            );
            assert!(app.surface_windows.wait_handles().is_empty());
            let settled = app.surface_windows.settle_shutdown(
                &mut app.remote_sessions,
                crate::desktop::surface_windows::now_ms().unwrap(),
            );
            assert_eq!(settled.is_ok(), !revoke);
        }
        let listener = server.join().unwrap();
        assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
        app.authorization_owner.close();
        app.remote_sessions.close();
        assert!(app.remote_sessions.poll_surfaces().is_empty() && app.remote_sessions.finished());
    }
}
#[test]
fn application_javascript_tls_mount_active_show_and_close_retains_owner_until_reaped() {
    run(true, false, false);
}
#[test]
fn application_javascript_idle_grant_revocation_reaps_without_ui_refresh() {
    run(true, true, false);
}
#[test]
fn application_javascript_unadvertised_runtime_never_fetches_or_activates() {
    run(false, false, false);
}
#[test]
fn application_javascript_corrupt_source_closes_without_active_or_fallback_display() {
    run(true, false, true);
}
