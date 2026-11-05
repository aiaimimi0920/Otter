//! 隐藏 HWND 测试菜单和模态事务；不读取真实身份、不访问用户选择的服务。
use super::*;
use crate::desktop::Window;
#[path = "device_projection_inbox_ui_tests.rs"]
mod projection_inbox;
#[test]
fn surface_resume_menu_requires_candidate_and_queues_original_batch_id() {
    unsafe {
        let window = window();
        let state = State::new(None, false).unwrap();
        state.authorization.borrow_mut().origin = Some("https://localhost:1234".into());
        for available in [false, true] {
            state.authorization.borrow_mut().continuation = available.then(|| "batch:first".into());
            let menu = CreatePopupMenu().unwrap();
            append_menu(menu, &state).unwrap();
            assert_eq!(
                GetMenuState(menu, RESUME_SURFACE as u32, MF_BYCOMMAND) & MF_GRAYED.0 == 0,
                available
            );
            DestroyMenu(menu).unwrap();
        }
        execute(window.0, &state, RESUME_SURFACE).unwrap();
        assert!(
            matches!(state.authorization.borrow_mut().request.take(), Some(Request::ResumeSurface(id)) if id == "batch:first")
        );
        assert!(crate::desktop::application::handles(RESUME_SURFACE));
        assert!(crate::desktop::group_lock::allows(RESUME_SURFACE));
        state.exit_requested.set(true);
        assert!(queue(
            window.0,
            &state,
            Request::ResumeSurface("batch:first".into())
        )
        .is_err());
        assert!(state.authorization.borrow().request.is_none());
    }
}
fn recovery_candidate() -> std::sync::Arc<crate::live_relay_http::RecoveryCandidate> {
    std::sync::Arc::new(crate::live_relay_http::RecoveryCandidate::fixture())
}
#[test]
fn surface_confirmation_menu_queues_only_explicit_selected_event() {
    unsafe {
        let window = window();
        let state = State::new(None, false).unwrap();
        state.authorization.borrow_mut().origin = Some("https://localhost:1234".into());
        for available in [false, true] {
            state.authorization.borrow_mut().confirmation = available.then(|| "event:test".into());
            let menu = CreatePopupMenu().unwrap();
            append_menu(menu, &state).unwrap();
            assert_eq!(
                GetMenuState(menu, CONFIRM_SURFACE as u32, MF_BYCOMMAND) & MF_GRAYED.0 == 0,
                available
            );
            DestroyMenu(menu).unwrap();
        }
        execute(window.0, &state, CONFIRM_SURFACE).unwrap();
        assert!(
            matches!(state.authorization.borrow_mut().request.take(), Some(Request::ConfirmSurface(id)) if id == "event:test")
        );
        assert!(crate::desktop::application::handles(CONFIRM_SURFACE));
        assert!(crate::desktop::group_lock::allows(CONFIRM_SURFACE));
        state.exit_requested.set(true);
        assert!(queue(
            window.0,
            &state,
            Request::ConfirmSurface("event:test".into())
        )
        .is_err());
        assert!(state.authorization.borrow().request.is_none());
    }
}
#[test]
fn restore_confirmation_cancel_and_accept_are_explicit_and_bounded() {
    unsafe {
        let window = window();
        let state = State::new(None, false).unwrap();
        let candidate = recovery_candidate();
        state.authorization.borrow_mut().origin = Some(candidate.origin().into());
        state.authorization.borrow_mut().recovery_candidate = Some(candidate.clone());
        recovery::confirm_with(window.0, &state, |_| false).unwrap();
        assert!(state.authorization.borrow().request.is_none());
        recovery::confirm_with(window.0, &state, |_| true).unwrap();
        assert!(
            recovery::confirm_with(window.0, &state, |_| panic!("second confirmation")).is_err()
        );
        assert!(
            matches!(state.authorization.borrow_mut().request.take(), Some(Request::RestoreSurface(value)) if std::sync::Arc::ptr_eq(&value, &candidate))
        );
        assert!(crate::desktop::application::handles(RESTORE_SURFACE));
        assert!(crate::desktop::group_lock::allows(RESTORE_SURFACE));
    }
}
#[test]
fn restore_modal_rejects_replaced_candidate_and_changed_origin() {
    unsafe {
        let window = window();
        let state = State::new(None, false).unwrap();
        for change_origin in [false, true] {
            let candidate = recovery_candidate();
            state.authorization.borrow_mut().origin = Some(candidate.origin().into());
            state.authorization.borrow_mut().recovery_candidate = Some(candidate);
            assert!(recovery::confirm_with(window.0, &state, |_| {
                if change_origin {
                    state.authorization.borrow_mut().origin =
                        Some("https://changed.example".into());
                } else {
                    state.authorization.borrow_mut().recovery_candidate =
                        Some(recovery_candidate());
                }
                true
            })
            .is_err());
            assert!(state.authorization.borrow().request.is_none());
            assert!(!state.file_dialog.get());
        }
    }
}
#[test]
fn restore_menu_is_disabled_while_mounting_and_without_a_current_candidate() {
    unsafe {
        let state = State::new(None, false).unwrap();
        for busy in [false, true] {
            let mut view = state.authorization.borrow_mut();
            view.origin = Some("https://localhost:1234".into());
            view.recovery_candidate = busy.then(recovery_candidate);
            view.mount_busy = busy;
            drop(view);
            let menu = CreatePopupMenu().unwrap();
            append_menu(menu, &state).unwrap();
            assert_ne!(
                GetMenuState(menu, RESTORE_SURFACE as u32, MF_BYCOMMAND) & MF_GRAYED.0,
                0
            );
            DestroyMenu(menu).unwrap();
        }
    }
}
unsafe fn window() -> Window {
    Window(
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("Authorization test"),
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
fn selection_is_validated_queued_shared_and_never_committed_by_cancel_or_stale_dialog() {
    unsafe {
        let window = window();
        let state = State::new(None, false).unwrap();
        let shared = state.authorization.clone();
        select_with(window.0, &state, |_| None).unwrap();
        assert!(shared.borrow().request.is_none());
        for bad in [
            "http://localhost",
            "https://user:pass@example.com",
            "https://example.com/path",
        ] {
            assert!(select_with(window.0, &state, |_| Some(bad.into())).is_err());
            assert!(shared.borrow().request.is_none());
        }
        select_with(window.0, &state, |_| Some("https://example.com".into())).unwrap();
        assert!(shared.borrow().origin.is_none());
        assert!(
            matches!(shared.borrow_mut().request.take(), Some(Request::Select(origin)) if origin == "https://example.com")
        );
        assert!(select_with(window.0, &state, |_| {
            shared.borrow_mut().origin = Some("https://changed.example".into());
            Some("https://old.example".into())
        })
        .is_err());
        assert!(shared.borrow().request.is_none());
        let hwnd = window.0;
        drop(window);
        assert!(queue(hwnd, &state, Request::Cancel).is_err());
    }
}
#[test]
fn requests_are_bounded_and_shutdown_modal_and_group_lock_rules_are_preserved() {
    unsafe {
        let window = window();
        let state = State::new(None, false).unwrap();
        assert!(queue(window.0, &state, Request::Start(Action::Renew)).is_err());
        assert!(queue(window.0, &state, Request::Discover).is_err());
        state.authorization.borrow_mut().origin = Some("https://example.com".into());
        queue(window.0, &state, Request::Start(Action::Renew)).unwrap();
        assert!(queue(window.0, &state, Request::Cancel).is_err());
        state.authorization.borrow_mut().request.take();
        state.authorization.borrow_mut().busy = true;
        assert!(queue(window.0, &state, Request::Start(Action::Renew)).is_err());
        assert!(queue(window.0, &state, Request::Discover).is_err());
        queue(window.0, &state, Request::Cancel).unwrap();
        state.authorization.borrow_mut().request.take();
        state.exit_requested.set(true);
        assert!(queue(window.0, &state, Request::Cancel).is_err());
        state.exit_requested.set(false);
        state.authorization.borrow_mut().closed = true;
        assert!(queue(window.0, &state, Request::Cancel).is_err());
        for id in [SELECT, PAIR, APPROVAL, RENEW, CANCEL, DISCOVER] {
            assert!(crate::desktop::application::handles(id));
            assert!(crate::desktop::group_lock::allows(id));
        }
        assert!(!handles(0x1090));
        let menu = CreatePopupMenu().unwrap();
        append_menu(menu, &state).unwrap();
        assert_eq!(
            GetMenuState(menu, SELECT as u32, MF_BYCOMMAND) & MF_GRAYED.0,
            MF_GRAYED.0
        );
        DestroyMenu(menu).unwrap();
    }
}

#[test]
fn art_selection_is_explicit_bound_to_window_and_rejects_stale_service() {
    unsafe {
        let window = window();
        let state = State::new(None, false).unwrap();
        state.authorization.borrow_mut().origin = Some("https://example.com".into());
        surface::choose(window.0, &state, || None).unwrap();
        assert!(state.authorization.borrow().request.is_none());
        assert!(surface::choose(window.0, &state, || Some("".into())).is_err());
        surface::choose(window.0, &state, || Some("art:test".into())).unwrap();
        assert!(
            matches!(state.authorization.borrow_mut().request.take(), Some(Request::OpenArt { art, node }) if art == "art:test" && node == state.unit_id.as_str())
        );
        assert!(surface::choose(window.0, &state, || {
            state.authorization.borrow_mut().origin = Some("https://changed.example".into());
            Some("art:stale".into())
        })
        .is_err());
        assert!(state.authorization.borrow().request.is_none());
        assert!(crate::desktop::application::handles(OPEN_ART));
        state.authorization.borrow_mut().mount_busy = true;
        assert!(queue(
            window.0,
            &state,
            Request::OpenArt {
                art: "art:test".into(),
                node: "node:test".into()
            }
        )
        .is_err());
    }
}
#[path = "device_projection_targets_ui_tests.rs"]
mod projection_targets;
