//! 真 TLS/原 Grant/原 HWND 点击→后台 listener→原批次→收敛→同 child update；无全局输入。
use super::*;
#[path = "application_surface_javascript_animation_tests.rs"]
mod animation;
#[path = "application_surface_javascript_canvas_tests.rs"]
mod canvas;
#[path = "application_surface_javascript_focus_tests.rs"]
mod focus;
#[path = "application_surface_javascript_hover_tests.rs"]
mod hover;
#[path = "application_surface_javascript_input_peer.rs"]
mod input_peer;
#[path = "application_surface_javascript_keyboard_tests.rs"]
mod keyboard;
#[path = "application_surface_javascript_pointer_tests.rs"]
mod pointer;
#[path = "application_surface_javascript_resize_tests.rs"]
mod resize;
#[path = "application_surface_javascript_input_setup.rs"]
mod setup;
#[path = "application_surface_javascript_text_tests.rs"]
mod text;
#[path = "application_surface_javascript_timer_tests.rs"]
mod timer;
use super::javascript::peer;
fn wait_js(app: &mut Application, reply: bool) {
    let started = std::time::Instant::now();
    if reply {
        app.surface_windows
            .iter_mut()
            .next()
            .unwrap()
            .javascript_wait_reply_for_test();
        eprintln!("JS wait reply completed in {:?}", started.elapsed());
        return;
    }
    let handle = app
        .surface_windows
        .javascript_wait_handles()
        .next()
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
fn assert_geometry_label(
    window: &crate::desktop::surface_authorized_window::AuthorizedWindow,
    expected: &str,
) {
    let label = window.javascript_text_for_test();
    let fields: Vec<_> = label.split('|').collect();
    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0], expected);
    let view = crate::surface_viewport::Viewport::from_window(window.edit_test_parent()).unwrap();
    assert_eq!(fields[1].parse::<f32>().unwrap(), view.logical()[0].round());
    assert_eq!(fields[2].parse::<f64>().unwrap(), view.ratio());
}
fn run(mode: u8) {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let _com = Apartment::new().unwrap();
    crate::surface_javascript::fixture::foreground_startup();
    let (files, store, _) = Files::new();
    let (listener, client, server, pem) = fixture_with_pem();
    let origin = format!(
        "https://localhost:{}",
        listener.local_addr().unwrap().port()
    );
    std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
    let peer = std::thread::spawn(move || input_peer::serve(listener, server, mode));
    unsafe {
        let (_class, mut app) = fixture::application();
        app.authorization.borrow_mut().origin = Some(origin.clone());
        app.authorization_owner
            .start(crate::device_authorization_runtime::Task::Renew { client, store })
            .unwrap();
        {
            let handle = app.authorization_owner.wait_handles().next().unwrap();
            assert_eq!(
                windows::Win32::System::Threading::WaitForSingleObject(
                    HANDLE(handle.as_raw_handle()),
                    5000
                ),
                WAIT_OBJECT_0
            );
        }
        assert!(matches!(
            app.authorization_owner.poll().as_slice(),
            [crate::device_authorization_owner::Event::Authorized { .. }]
        ));
        if (6..=12).contains(&mode) {
            // 在原 mount 最后一次启用输入之前设置策略，避免测试先提交一帧再断言尚未开始。
            app.authorization.borrow_mut().busy = true;
        }
        let parent = setup::mount(&mut app, &origin, files.root());
        let text_edit = matches!(mode, 14 | 15 | 18).then(|| text::child(parent));
        let _foreground = (15..=18)
            .contains(&mode)
            .then(|| crate::surface_javascript::fixture::focus_foreground(parent));
        let probe = app
            .surface_windows
            .iter_mut()
            .next()
            .unwrap()
            .javascript_probe()
            .unwrap();
        if mode == 6 {
            animation::prepare(&mut app, files.root());
        }
        if mode == 7 {
            animation::stale(&mut app, files.root(), parent);
        }
        if (8..=9).contains(&mode) {
            resize::exercise(&mut app, files.root(), parent, mode == 9);
        }
        if mode == 10 {
            canvas::exercise(&mut app, files.root());
        }
        if mode == 11 {
            timer::prepare(&mut app, files.root());
        }
        if mode == 12 {
            animation::stale(&mut app, files.root(), parent);
        }
        if (16..=17).contains(&mode) {
            focus::stale(&mut app, files.root(), parent, mode == 17);
        }
        if (3..=5).contains(&mode) {
            let window = app.surface_windows.iter_mut().next().unwrap();
            assert_geometry_label(window, "原生🙂:42:0");
        }
        if mode == 4 || mode == 5 {
            app.surface_windows
                .iter_mut()
                .next()
                .unwrap()
                .javascript_click_for_test();
            app.poll_remote_live_with(true, |o| fixture::config(o, files.root()))
                .unwrap();
            wait_js(&mut app, true);
            if mode == 5 {
                app.authorization.borrow_mut().busy = true;
                app.poll_remote_live_with(true, |o| fixture::config(o, files.root()))
                    .unwrap();
                assert!(app.surface_windows.iter_mut().next().unwrap().is_alive());
                assert_eq!(app.remote_sessions.wait_handles().count(), 0);
            }
            // listener 已运行并准备 emit；交付前改变真实客户区，必须关闭而非重跑或发旧事件。
            SetWindowPos(
                parent,
                None,
                0,
                0,
                470,
                280,
                SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER,
            )
            .unwrap();
            if mode == 5 {
                app.authorization.borrow_mut().busy = false;
            }
            app.poll_remote_live_with(true, |o| fixture::config(o, files.root()))
                .unwrap();
            let window = app.surface_windows.iter_mut().next().unwrap();
            assert!(!window.is_alive() && window.snapshot_revision() == 1);
        }
        for round in 0..if mode == 4
            || mode == 5
            || ((7..=12).contains(&mode) && mode != 11)
            || (16..=17).contains(&mode)
        {
            0
        } else if matches!(mode, 0 | 3 | 6 | 11 | 13 | 14 | 15 | 18 | 19 | 20) {
            2
        } else {
            1
        } {
            {
                if mode == 3 {
                    SetWindowPos(
                        parent,
                        None,
                        0,
                        0,
                        420 + round as i32 * 80,
                        260,
                        SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER,
                    )
                    .unwrap();
                }
                let window = app.surface_windows.iter_mut().next().unwrap();
                assert!(window.is_shown());
                assert!(window.local_edit_enabled());
                if !matches!(mode, 14 | 15 | 18 | 19 | 20) {
                    window.javascript_click_for_test();
                }
            }
            if mode == 14 {
                text::prepare(&mut app, files.root(), parent, round);
            }
            if mode == 15 {
                focus::prepare(&mut app, files.root(), parent, round);
            }
            if mode == 20 {
                hover::prepare(&mut app, files.root(), parent);
            } else if mode == 19 {
                pointer::prepare(&mut app, files.root(), parent);
            } else if mode == 18 {
                keyboard::prepare(&mut app, files.root(), parent, round);
            } else {
                app.poll_remote_live_with(true, |o| fixture::config(o, files.root()))
                    .unwrap();
                if mode != 15 {
                    assert_eq!(app.remote_sessions.wait_handles().count(), 0); // listener 只在原后台 child 执行。
                    assert!(app
                        .surface_windows
                        .iter_mut()
                        .next()
                        .unwrap()
                        .lifecycle()
                        .unwrap()
                        .confirmed_active());
                }
                if mode != 15
                    || app
                        .surface_windows
                        .iter_mut()
                        .next()
                        .unwrap()
                        .javascript_pending_for_test()
                {
                    wait_js(&mut app, true);
                }
                app.poll_remote_live_with(true, |o| fixture::config(o, files.root()))
                    .unwrap();
                if mode == 6 || mode == 11 {
                    animation::tick(&mut app, files.root());
                }
            }
            if mode == 3 {
                assert_geometry_label(
                    app.surface_windows.iter_mut().next().unwrap(),
                    &format!("原生🙂:{}:{}", if round == 0 { 42 } else { 45 }, round + 1),
                );
            }
            assert_eq!(app.remote_sessions.wait_handles().count(), 1);
            assert!(app
                .surface_windows
                .iter_mut()
                .next()
                .unwrap()
                .lifecycle()
                .is_none());
            if mode == 6 || mode == 11 {
                animation::paused(&mut app);
            }
            setup::wait_batch(&app);
            app.poll_remote_live_with(true, |o| fixture::config(o, files.root()))
                .unwrap();
            {
                let receipt = app
                    .remote_sessions
                    .surface_batch_receipt_for_test()
                    .unwrap();
                eprintln!(
                    "JS batch submission {:?}, attempted {:?}",
                    receipt
                        .submission
                        .as_ref()
                        .map(|s| (&s.stop, s.acknowledgements.len())),
                    receipt
                        .events
                        .iter()
                        .map(|event| (event.attempted(), event.revision()))
                        .collect::<Vec<_>>()
                );
                eprintln!(
                    "JS batch presentation {:?}, delivery {:?}, observations {:?}",
                    receipt.presentation,
                    receipt.delivery,
                    receipt
                        .observation
                        .as_ref()
                        .map(|result| result.as_ref().map(|value| value
                            .items
                            .iter()
                            .map(|item| item.result)
                            .collect::<Vec<_>>()))
                );
                assert_eq!(receipt.events.len(), 2);
                assert!(receipt.events.iter().all(|event| event.attempted()
                    && event.is_javascript()
                    && event.revision() == round + 1));
                if matches!(
                    receipt.presentation,
                    Err(crate::desktop::surface_authorized_window::PresentationError::Pending(_))
                ) {
                    let window = app.surface_windows.iter_mut().next().unwrap();
                    assert!(!app.remote_sessions.surface_input_ready(window.binding()));
                    assert_eq!(window.snapshot_revision(), round + 1);
                }
            }
            if mode == 2 {
                assert!(app
                    .remote_sessions
                    .surface_batch_receipt_for_test()
                    .unwrap()
                    .observation
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .items
                    .iter()
                    .all(|item| item.result.is_err()));
                let window = app.surface_windows.iter_mut().next().unwrap();
                assert_eq!(window.snapshot_revision(), 1);
                assert_eq!(window.javascript_text_for_test(), "原生🙂:42:1");
                assert!(!probe.exited());
                break;
            }
            if app
                .surface_windows
                .iter_mut()
                .next()
                .unwrap()
                .javascript_pending_for_test()
            {
                wait_js(
                    &mut app,
                    matches!(mode, 0 | 3 | 6 | 11 | 13 | 14 | 15 | 18 | 19 | 20),
                );
            }
            app.poll_remote_live_with(true, |o| fixture::config(o, files.root()))
                .unwrap();
            if mode == 1 {
                let window = app.surface_windows.iter_mut().next().unwrap();
                assert!(!window.is_alive() && !window.is_shown());
                assert_eq!(window.snapshot_revision(), 1);
                assert!(probe.exited());
                assert!(app
                    .remote_sessions
                    .surface_batch_receipt_for_test()
                    .unwrap()
                    .presentation
                    .is_err());
                break;
            }
            {
                let window = app.surface_windows.iter_mut().next().unwrap();
                assert!(std::sync::Arc::ptr_eq(
                    &probe,
                    &window.javascript_probe().unwrap()
                ));
                assert_eq!(window.snapshot_revision(), round + 2);
                let expected = format!("原生🙂:{}:{}", if round == 0 { 45 } else { 48 }, round + 1);
                if mode == 3 {
                    assert_geometry_label(window, &expected);
                } else if mode == 14 {
                    text::updated(window, parent, text_edit.unwrap(), round);
                } else if mode == 15 {
                    focus::updated(window, parent, text_edit.unwrap(), round);
                } else if mode == 18 {
                    keyboard::updated(window, parent, text_edit.unwrap(), round);
                } else {
                    assert_eq!(window.javascript_text_for_test(), expected);
                }
                assert!(app.remote_sessions.surface_input_ready(window.binding()));
                assert!(!probe.exited());
            }
            assert!(
                matches!(app.remote_sessions.surface_batch_receipt_for_test().unwrap().presentation, Ok(Some(revision)) if revision == round + 2)
            );
            if mode == 18 {
                keyboard::release(&mut app, files.root(), parent);
            }
            if mode == 6 || mode == 11 {
                animation::tick(&mut app, files.root());
            }
            for _ in 0..2 {
                app.poll_remote_live_with(true, |_| {
                    panic!("idle JS resubmitted or refetched source")
                })
                .unwrap();
            }
        }
        app.surface_windows.close_all().unwrap();
        if !app
            .surface_windows
            .iter_mut()
            .next()
            .unwrap()
            .javascript_finished()
        {
            wait_js(&mut app, false);
        }
        app.poll_remote_live_with(false, |o| fixture::config(o, files.root()))
            .unwrap();
        if app.remote_sessions.wait_handles().next().is_some() {
            fixture::wait(&app);
        }
        app.poll_remote_live_with(false, |_| panic!("inactive repeated"))
            .unwrap();
        assert!(probe.exited() && !IsWindow(Some(parent)).as_bool());
        let window = app.surface_windows.iter_mut().next().unwrap();
        assert_eq!(
            window.binding().lifecycle(),
            crate::surface_live_binding::Lifecycle::Inactive
        );
        assert!(window.javascript_finished());
        let listener = peer.join().unwrap();
        assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
        app.authorization_owner.close();
        app.remote_sessions.close();
        assert!(app.remote_sessions.poll_surfaces().is_empty() && app.remote_sessions.finished());
    }
}
#[test]
fn application_javascript_input_real_fifo_and_same_child_update_across_two_clicks() {
    run(0);
}
#[test]
fn application_javascript_event_target_phases_real_tls_grant_fifo_and_same_child() {
    run(13);
}
#[test]
fn application_javascript_native_input_real_edit_change_tls_grant_fifo_and_same_child() {
    run(14);
}
#[test]
fn application_javascript_native_focus_blur_real_edit_change_tls_grant_fifo_and_same_child() {
    run(15);
}
#[test]
fn application_javascript_native_focus_inflight_aba_is_discarded_without_replay() {
    run(16);
}
#[test]
fn application_javascript_native_focus_inflight_policy_revocation_precedes_reply() {
    run(17);
}
#[test]
fn application_javascript_native_enter_cancel_default_real_tls_grant_fifo_same_child() {
    run(18);
}
#[test]
fn application_javascript_input_update_failure_never_presents_authority_or_replays() {
    run(1);
}
#[test]
fn application_javascript_input_runtime_switch_is_rejected_and_preserves_receipt() {
    run(2);
}
#[test]
fn application_javascript_geometry_real_client_mount_click_update_and_resize_keep_fifo() {
    run(3);
}
#[test]
fn application_javascript_geometry_stale_viewport_discards_emit_without_replaying_listener() {
    run(4);
}
#[test]
fn application_javascript_geometry_accepted_but_held_emit_keeps_viewport_until_submission() {
    run(5);
}
