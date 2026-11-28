//! Resident-tray lifecycle; all Shell notifications are injected, operating only test-owned windows.
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
        state.canvas.borrow_mut().pinned = true;
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
        // Icons in restored state must also activate hosts, not accept callbacks only when hidden=true.
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
fn empty_host_stays_hidden_after_mouse_or_keyboard_tray_activation() {
    unsafe {
        let state = State::new(None, false).unwrap();
        let owner = window();
        start_with(&state, || install_with(&state.tray.installed, |_| true)).unwrap();
        for hidden in [false, true] {
            state.tray.hidden.set(hidden);
            for event in [NIN_SELECT, NIN_KEYSELECT] {
                dispatch(
                    owner.0,
                    &state,
                    CALLBACK,
                    WPARAM(0),
                    LPARAM(((ID << 16) | event) as isize),
                );
                assert!(!IsWindowVisible(owner.0).as_bool());
                assert_eq!(is_hidden(&state), hidden);
                assert!(is_resident(&state) && state.tray.installed.get());
                let canvas = state.canvas.borrow();
                assert!(!canvas.pinned && canvas.image.is_none());
            }
        }
        let (menu, _) = build_menu(&state).unwrap();
        assert_eq!(GetMenuItemCount(Some(menu.0)), 12);
        let items = crate::native_menu::snapshot(menu.0).unwrap();
        let language = items
            .iter()
            .find(|item| item.label == crate::ui_text::text("tray-language"))
            .expect("empty hosts retain the language submenu");
        assert!(language.enabled);
        assert_eq!(language.children.as_ref().unwrap().len(), 2);
    }
}

#[test]
fn tray_activation_still_restores_image_documents_and_pins_without_images() {
    unsafe {
        for pinned in [false, true] {
            let state = State::new(None, false).unwrap();
            {
                let mut canvas = state.canvas.borrow_mut();
                canvas.pinned = pinned;
                if !pinned {
                    canvas.image = Some(crate::desktop::image::Image {
                        width: 1,
                        height: 1,
                        pixels: vec![17, 83, 211, 255].into(),
                    });
                }
            }
            let owner = window();
            start_with(&state, || install_with(&state.tray.installed, |_| true)).unwrap();
            for event in [NIN_SELECT, NIN_KEYSELECT] {
                hide_with(owner.0, &state, || Ok(())).unwrap();
                dispatch(
                    owner.0,
                    &state,
                    CALLBACK,
                    WPARAM(0),
                    LPARAM(((ID << 16) | event) as isize),
                );
                assert!(IsWindowVisible(owner.0).as_bool());
                assert!(!is_hidden(&state) && state.tray.installed.get());
            }
        }
    }
}

#[test]
fn explorer_restart_reinstalls_resident_or_hidden_icons_and_only_recovers_on_failure() {
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
            assert_eq!(restored.get(), (resident || hidden) && !success);
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
