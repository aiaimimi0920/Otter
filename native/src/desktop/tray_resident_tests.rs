//! 常驻托盘生命周期；Shell 通知全部注入，仅操作测试自有窗口。
use super::*;

unsafe fn window() -> crate::desktop::Window {
    crate::desktop::Window(
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            w!("STATIC"),
            w!("resident tray fixture"),
            WS_POPUP,
            0,
            0,
            10,
            10,
            None,
            None,
            None,
            None,
        )
        .unwrap(),
    )
}

#[test]
fn startup_hide_restore_and_visible_callback_keep_the_resident_icon() {
    unsafe {
        let state = State::new(None, false).unwrap();
        let owner = window();
        let calls = std::cell::RefCell::new(Vec::new());
        start_with(&state, || {
            install_with(&state.tray.installed, |message| {
                calls.borrow_mut().push(message);
                true
            })
        })
        .unwrap();
        assert!(is_resident(&state) && state.tray.installed.get());
        assert!(!is_hidden(&state));
        assert_eq!(*calls.borrow(), [NIM_ADD, NIM_SETVERSION]);
        start_with(&state, || panic!("Repeated startup must not notify Shell")).unwrap();
        hide_with(owner.0, &state, || Ok(())).unwrap();
        assert!(is_hidden(&state));
        restore_with(owner.0, &state, |_, _| {}, |_| {});
        assert!(!is_hidden(&state) && state.tray.installed.get());
        assert_eq!(*calls.borrow(), [NIM_ADD, NIM_SETVERSION]);
        // 恢复状态下的图标也必须能激活宿主，而非只有 hidden=true 才接收回调。
        assert!(!IsWindowVisible(owner.0).as_bool());
        dispatch(
            owner.0,
            &state,
            CALLBACK,
            WPARAM(0),
            LPARAM(((ID << 16) | NIN_KEYSELECT) as isize),
        );
        assert!(IsWindowVisible(owner.0).as_bool());
        assert!(state.tray.installed.get());
    }
}

#[test]
fn explorer_restart_reinstalls_resident_or_hidden_icons_and_only_restores_hidden_on_failure() {
    for (resident, hidden) in [(true, false), (true, true), (false, true), (false, false)] {
        for success in [true, false] {
            let state = State::new(None, false).unwrap();
            state.tray.resident.set(resident);
            state.tray.hidden.set(hidden);
            state.tray.installed.set(true);
            let attempts = Cell::new(0);
            let restored = Cell::new(false);
            restarted_with(
                &state,
                || {
                    attempts.set(attempts.get() + 1);
                    assert!(!state.tray.installed.get());
                    install_with(&state.tray.installed, |_| success)
                },
                || restored.set(true),
            );
            assert_eq!(attempts.get(), u32::from(resident || hidden));
            assert_eq!(restored.get(), hidden && !success);
            assert_eq!(state.tray.installed.get(), (resident || hidden) && success);
            assert_eq!(state.tray.hidden.get(), hidden);
        }
    }
}

#[test]
fn failed_startup_retains_resident_recovery_without_repeating_each_message() {
    let state = State::new(None, false).unwrap();
    assert!(start_with(&state, || {
        install_with(&state.tray.installed, |_| false)
    })
    .is_err());
    assert!(is_resident(&state));
    assert!(!state.tray.installed.get() && !is_hidden(&state));
    start_with(&state, || panic!("No unbounded startup retry")).unwrap();
    restarted_with(
        &state,
        || install_with(&state.tray.installed, |_| true),
        || panic!("Visible host must not be restored on successful restart"),
    );
    assert!(state.tray.installed.get());
}
