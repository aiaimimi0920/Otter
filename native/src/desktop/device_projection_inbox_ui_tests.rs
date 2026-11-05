//! 真实菜单与隐藏发起窗口；数据来自原合同 fixture，不联网、不显示对话框。
use super::*;
use crate::{desktop::device_authorization_ui::projection as inbox, projection_delivery::Inbox};
use std::sync::Arc;
const ORIGIN: &str = "https://loom.example";
#[path = "device_projection_recovery_ui_tests.rs"]
mod recovery;
fn entry(accepted: bool) -> Arc<crate::projection_delivery::Invitation> {
    let mut value = crate::projection_delivery::tests::invitation(ORIGIN, accepted);
    value["sourceName"] = serde_json::json!("A&B".repeat(100));
    Arc::new(
        Inbox::parse(
            &serde_json::to_vec(&serde_json::json!({"invitations":[value]})).unwrap(),
            ORIGIN,
        )
        .unwrap()
        .into_entries()
        .pop()
        .unwrap(),
    )
}
#[test]
fn real_projection_menu_and_queue_keep_shared_state_identity_and_bounded_labels() {
    unsafe {
        let window = window();
        let state = State::new(None, false).unwrap();
        let pending = entry(false);
        let mut view = state.authorization.borrow_mut();
        view.origin = Some(ORIGIN.into());
        view.projection.entries = vec![pending.clone(), entry(true)];
        drop(view);
        let menu = CreatePopupMenu().unwrap();
        append_menu(menu, &state).unwrap();
        for command in [inbox::REFRESH, inbox::REJECT_BASE, inbox::ACCEPT_BASE] {
            assert_eq!(
                GetMenuState(menu, command as u32, MF_BYCOMMAND) & MF_GRAYED.0,
                0
            );
            assert!(crate::desktop::application::handles(command));
            assert!(crate::desktop::group_lock::allows(command));
        }
        assert_ne!(
            GetMenuState(menu, (inbox::REJECT_BASE + 1) as u32, MF_BYCOMMAND) & MF_GRAYED.0,
            0
        );
        let mut label = [0u16; 512];
        let length = GetMenuStringW(
            menu,
            inbox::REJECT_BASE as u32,
            Some(&mut label),
            MF_BYCOMMAND,
        );
        let label = String::from_utf16(&label[..length as usize]).unwrap();
        assert!(label.contains("&&") && label.contains('…') && label.len() < 256);
        DestroyMenu(menu).unwrap();
        execute(window.0, &state, inbox::REFRESH).unwrap();
        assert!(matches!(
            state.authorization.borrow_mut().request.take(),
            Some(Request::Projection(inbox::Request::Refresh))
        ));
        execute(window.0, &state, inbox::REJECT_BASE).unwrap();
        assert!(matches!(state.authorization.borrow_mut().request.take(),
            Some(Request::Projection(inbox::Request::Reject(selected))) if Arc::ptr_eq(&selected, &pending)));
    }
}
#[test]
fn projection_menu_and_requests_reject_busy_closed_cross_origin_and_accepted_candidates() {
    unsafe {
        let state = State::new(None, false).unwrap();
        for mode in 0..7 {
            let mut view = state.authorization.borrow_mut();
            *view = View::default();
            view.origin = Some(
                if mode == 4 {
                    "https://other.example"
                } else {
                    ORIGIN
                }
                .into(),
            );
            view.projection.entries = vec![entry(mode == 5)];
            view.closed = mode == 0;
            view.busy = mode == 1;
            view.projection.busy = mode == 2;
            if mode == 3 {
                view.request = Some(Request::Cancel);
            }
            if mode == 6 {
                view.origin = None;
            }
            assert!(inbox::request(&view, inbox::REJECT_BASE).is_err());
            assert!(inbox::request(&view, inbox::ACCEPT_BASE).is_err());
            drop(view);
            let menu = CreatePopupMenu().unwrap();
            append_menu(menu, &state).unwrap();
            assert_ne!(
                GetMenuState(menu, inbox::REJECT_BASE as u32, MF_BYCOMMAND) & MF_GRAYED.0,
                0
            );
            DestroyMenu(menu).unwrap();
        }
    }
}
#[test]
fn workspace_reset_discards_projection_request_but_preserves_unrelated_authorization_intent() {
    let mut view = View::default();
    view.projection.entries.push(entry(false));
    view.request = Some(Request::Projection(inbox::Request::Refresh));
    inbox::reset(&mut view);
    assert!(view.request.is_none() && view.projection.entries.is_empty());
    view.request = Some(Request::Cancel);
    inbox::reset(&mut view);
    assert!(matches!(view.request, Some(Request::Cancel)));
}
