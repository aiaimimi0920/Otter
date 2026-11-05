//! 真实未结投送菜单/模态确认；同记录替代、忙碌、撤权切换与取消不排队、不触盘。
use super::*;
use crate::projection_delivery::{
    accept::tests::invitation,
    journal::{tests::Directory, Store, Ticket},
};
fn candidate(directory: &Directory) -> Arc<Ticket> {
    let store = Store::open(&directory.0).unwrap();
    Arc::new(store.begin(&invitation(ORIGIN), "device-old").unwrap())
}
#[test]
fn recovery_menu_requires_explicit_confirmation_and_preserves_exact_record_identity() {
    unsafe {
        let directory = Directory::new();
        let entry = candidate(&directory);
        let bytes = std::fs::read(directory.path()).unwrap();
        let window = window();
        let state = State::new(None, false).unwrap();
        state.modals.register(&state.file_dialog);
        state.authorization.borrow_mut().origin = Some(ORIGIN.into());
        state
            .authorization
            .borrow_mut()
            .projection
            .recovery
            .entries
            .push(entry.clone());
        let menu = CreatePopupMenu().unwrap();
        append_menu(menu, &state).unwrap();
        assert_eq!(
            GetMenuState(menu, inbox::recovery::BASE as u32, MF_BYCOMMAND) & MF_GRAYED.0,
            0
        );
        assert!(crate::desktop::application::handles(inbox::recovery::BASE));
        assert!(crate::desktop::group_lock::allows(inbox::recovery::BASE));
        DestroyMenu(menu).unwrap();
        inbox::recovery::confirm_with(window.0, &state, inbox::recovery::BASE, |_| {
            assert!(state.modals.blocked());
            assert!(state.authorization.borrow().request.is_none());
            false
        })
        .unwrap();
        assert!(state.authorization.borrow().request.is_none());
        inbox::recovery::confirm_with(window.0, &state, inbox::recovery::BASE, |_| true).unwrap();
        assert!(matches!(state.authorization.borrow_mut().request.take(),
            Some(Request::Projection(inbox::Request::Compensate(current))) if Arc::ptr_eq(&entry, &current)));
        assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
    }
}
#[test]
fn recovery_confirmation_revalidates_busy_origin_closed_and_replaced_candidates_without_refcell_borrow(
) {
    unsafe {
        let directory = Directory::new();
        let entry = candidate(&directory);
        let replacement = Arc::new(entry.retry());
        let window = window();
        let state = State::new(None, false).unwrap();
        state.modals.register(&state.file_dialog);
        for mode in 0..6 {
            *state.authorization.borrow_mut() = View::default();
            state.authorization.borrow_mut().origin = Some(ORIGIN.into());
            state.authorization.borrow_mut().projection.recovery.entries = vec![entry.clone()];
            let result =
                inbox::recovery::confirm_with(window.0, &state, inbox::recovery::BASE, |_| {
                    let mut view = state.authorization.borrow_mut();
                    match mode {
                        0 => view.closed = true,
                        1 => view.busy = true,
                        2 => view.projection.busy = true,
                        3 => view.origin = Some("https://other.example".into()),
                        4 => view.projection.recovery.entries = vec![replacement.clone()],
                        5 => state.exit_requested.set(true),
                        _ => unreachable!(),
                    }
                    true
                });
            assert!(result.is_err());
            assert!(state.authorization.borrow().request.is_none());
            assert!(!state.file_dialog.get());
            state.exit_requested.set(false);
        }
    }
}
#[test]
fn unavailable_recovery_never_opens_a_confirmation_or_queues_a_request() {
    unsafe {
        let directory = Directory::new();
        let entry = candidate(&directory);
        let window = window();
        let state = State::new(None, false).unwrap();
        state.modals.register(&state.file_dialog);
        for mode in 0..6 {
            *state.authorization.borrow_mut() = View::default();
            state.authorization.borrow_mut().origin = Some(ORIGIN.into());
            state.authorization.borrow_mut().projection.recovery.entries = vec![entry.clone()];
            match mode {
                0 => state.authorization.borrow_mut().busy = true,
                1 => state.authorization.borrow_mut().projection.busy = true,
                2 => state.authorization.borrow_mut().closed = true,
                3 => state.authorization.borrow_mut().origin = None,
                4 => state.file_dialog.set(true),
                5 => state.exit_requested.set(true),
                _ => unreachable!(),
            }
            let _ = inbox::recovery::confirm_with(window.0, &state, inbox::recovery::BASE, |_| {
                panic!("unavailable recovery opened a dialog")
            });
            assert!(state.authorization.borrow().request.is_none());
            state.file_dialog.set(false);
            state.exit_requested.set(false);
        }
    }
}
#[test]
fn recovery_saved_label_requires_the_exact_candidate_not_an_equal_record_snapshot() {
    unsafe {
        let directory = Directory::new();
        let entry = candidate(&directory);
        let replacement = Arc::new(entry.retry());
        let state = State::new(None, false).unwrap();
        state.authorization.borrow_mut().origin = Some(ORIGIN.into());
        state.authorization.borrow_mut().projection.recovery.entries = vec![entry.clone()];
        for (saved, prefix) in [
            (replacement, "解除未结投送绑定："),
            (entry, "解除已保存投送绑定："),
        ] {
            state.authorization.borrow_mut().projection.recovery.saved = vec![saved];
            let menu = CreatePopupMenu().unwrap();
            append_menu(menu, &state).unwrap();
            let mut text = [0u16; 128];
            let count = GetMenuStringW(
                menu,
                inbox::recovery::BASE as u32,
                Some(&mut text),
                MF_BYCOMMAND,
            );
            assert!(String::from_utf16(&text[..count as usize])
                .unwrap()
                .starts_with(prefix));
            DestroyMenu(menu).unwrap();
        }
        assert!(state.authorization.borrow().request.is_none());
    }
}
#[test]
fn projection_display_recovery_menu_only_queues_the_exact_saved_noncleanup_candidate() {
    let directory = Directory::new();
    let entry = candidate(&directory);
    let mut view = View {
        origin: Some(ORIGIN.into()),
        ..View::default()
    };
    view.projection.recovery.entries = vec![entry.clone()];
    let command = inbox::recovery::DISPLAY_BASE;
    assert!(inbox::request(&view, command).is_err());
    view.projection.recovery.saved = vec![Arc::new(entry.retry())];
    assert!(inbox::request(&view, command).is_err());
    view.projection.recovery.saved = vec![entry.clone()];
    assert!(
        matches!(inbox::request(&view, command), Ok(inbox::Request::Display(current)) if Arc::ptr_eq(&current, &entry))
    );
    view.projection.busy = true;
    assert!(inbox::request(&view, command).is_err());
    view.projection.busy = false;
    let marked = Arc::new(entry.retry().queue_cleanup().unwrap());
    view.projection.recovery.entries = vec![marked.clone()];
    view.projection.recovery.saved = vec![marked];
    assert!(inbox::request(&view, command).is_err());
}
