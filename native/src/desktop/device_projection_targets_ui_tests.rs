//! 隐藏窗口真实菜单/队列；暂停仅约束收件，无发送/停止资格时各命令保持禁用。
use super::*;
use crate::{
    desktop::device_authorization_ui::{sender, targets},
    projection_delivery::targets::Directory,
};
const ORIGIN: &str = "https://loom.example";
#[test]
fn projection_targets_menu_queues_while_receive_paused_and_escapes_bounded_readonly_entries() {
    unsafe {
        let window = window();
        let state = State::new(None, false).unwrap();
        let mut value = crate::projection_delivery::targets::tests::body();
        value["targets"][0]["name"] = serde_json::json!("A&B".repeat(100));
        let mut view = state.authorization.borrow_mut();
        view.origin = Some(ORIGIN.into());
        view.consent.install_paused(true);
        view.targets.directory = Some(
            Directory::parse(&serde_json::to_vec(&value).unwrap(), ORIGIN)
                .unwrap()
                .into(),
        );
        drop(view);
        let menu = CreatePopupMenu().unwrap();
        append_menu(menu, &state).unwrap();
        assert_eq!(
            GetMenuState(menu, targets::REFRESH as u32, MF_BYCOMMAND) & MF_GRAYED.0,
            0
        );
        assert!(crate::desktop::application::handles(targets::REFRESH));
        assert!(crate::desktop::group_lock::allows(targets::REFRESH));
        let child = GetSubMenu(menu, GetMenuItemCount(Some(menu)) - 2);
        assert!(!child.is_invalid());
        let commands = [
            targets::REFRESH,
            sender::BASE,
            sender::groups::BASE,
            sender::groups::STOP_BASE,
        ];
        assert_eq!(GetMenuItemCount(Some(child)), commands.len() as i32);
        for (index, command) in commands.into_iter().enumerate() {
            assert_eq!(GetMenuItemID(child, index as i32), command as u32);
            if index != 0 {
                assert_ne!(
                    GetMenuState(child, command as u32, MF_BYCOMMAND) & MF_GRAYED.0,
                    0
                );
            }
        }
        let mut text = [0u16; 512];
        let count = GetMenuStringW(child, 1, Some(&mut text), MF_BYPOSITION);
        let text = String::from_utf16(&text[..count as usize]).unwrap();
        assert!(text.contains("&&") && text.contains('…') && text.len() < 256);
        assert_ne!(GetMenuState(child, 1, MF_BYPOSITION) & MF_GRAYED.0, 0);
        DestroyMenu(menu).unwrap();
        execute(window.0, &state, targets::REFRESH).unwrap();
        assert!(matches!(
            state.authorization.borrow_mut().request.take(),
            Some(Request::ProjectionTargets)
        ));
    }
}
#[test]
fn projection_targets_menu_rejects_busy_closed_missing_origin_and_competing_requests() {
    for mode in 0..6 {
        let mut view = View {
            origin: Some(ORIGIN.into()),
            closed: mode == 0,
            busy: mode == 1,
            ..Default::default()
        };
        view.projection.busy = mode == 2;
        view.targets.busy = mode == 3;
        if mode == 4 {
            view.origin = None;
        }
        if mode == 5 {
            view.request = Some(Request::Cancel);
        }
        assert!(targets::request(&view).is_err());
    }
    let mut view = View::default();
    view.targets.busy = true;
    view.request = Some(Request::ProjectionTargets);
    targets::reset(&mut view);
    assert!(view.request.is_none() && !view.targets.busy);
    view.request = Some(Request::Cancel);
    targets::reset(&mut view);
    assert!(matches!(view.request, Some(Request::Cancel)));
}
