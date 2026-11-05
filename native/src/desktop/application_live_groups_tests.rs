//! Live 系统菜单到应用分组事务；隐藏/离屏真实 HWND，不读取用户桌面或注册用户热键。
use super::*;
use crate::desktop::composition::live::window::groups::{JOIN, LEAVE};
use crate::{
    desktop::group_menu::{self, Request},
    workspace_id::UnitId,
};
unsafe fn command(hwnd: HWND, command: usize) {
    SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(command)), Some(LPARAM(0)));
}
unsafe fn host_request(app: &mut Application, request: Request) {
    let host = &app.documents[0];
    group_menu::queue(host.window.0, &host.state, request).unwrap();
    app.pending_groups(false).unwrap();
}
unsafe fn group(app: &mut Application) -> UnitId {
    host_request(app, Request::Create("Live 分组".into()));
    app.workspace.groups()[0].id.clone()
}
#[test]
fn live_group_menu_joins_leaves_and_rejects_stale_catalog_and_old_workspace() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (hwnd, unit) = live(&mut app, true);
        let group = group(&mut app);
        command(hwnd, JOIN);
        app.pending_groups(false).unwrap();
        assert_eq!(app.workspace.group_of(&unit).unwrap().id, group);
        command(hwnd, LEAVE);
        app.pending_groups(false).unwrap();
        assert!(app.workspace.group_of(&unit).is_none());
        for index in 1..crate::workspace::MAX_UNITS {
            app.workspace
                .upsert_group(crate::workspace::Group {
                    id: UnitId::parse(&format!("group-{index}")).unwrap(),
                    name: format!("Group & {index}"),
                    hidden: false,
                    locked: false,
                })
                .unwrap();
        }
        app.sync_groups();
        command(hwnd, JOIN + (crate::workspace::MAX_UNITS - 1) * 16);
        app.pending_groups(false).unwrap();
        assert_eq!(
            app.workspace.group_of(&unit).unwrap().id.as_str(),
            "group-127"
        );
        command(hwnd, LEAVE);
        app.pending_groups(false).unwrap();
        command(hwnd, JOIN);
        let mut changed = app.workspace.groups()[0].clone();
        changed.name = "changed".into();
        app.workspace.upsert_group(changed).unwrap();
        app.pending_groups(false).unwrap();
        assert!(app.workspace.group_of(&unit).is_none());
        command(hwnd, JOIN);
        app.workspace =
            crate::workspace::Workspace::with_initial(app.documents[0].state.unit_id.clone());
        app.pending_groups(false).unwrap();
        assert!(!app.workspace.contains(&unit));
        assert!(app.live_sessions[0].is_alive());
        app.shutdown_live().unwrap();
    }
}
#[test]
fn live_group_visibility_lock_and_delete_follow_shared_group_state() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (hwnd, unit) = live(&mut app, true);
        let group = group(&mut app);
        command(hwnd, JOIN);
        app.pending_groups(false).unwrap();
        // 离屏但真实可见标志；恢复不抢焦点，不在用户桌面展示测试内容。
        SetWindowPos(
            hwnd,
            None,
            -30000,
            -30000,
            64,
            64,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        assert!(IsWindowVisible(hwnd).as_bool());
        app.set_group_hidden_with(&group, true, |_, _| Ok(()), |_, _| {})
            .unwrap();
        app.sync_groups();
        assert!(!IsWindowVisible(hwnd).as_bool());
        assert!(app.live_sessions[0]
            .enable_input()
            .unwrap_err()
            .to_string()
            .contains("hidden"));
        app.set_group_hidden_with(&group, false, |_, _| Ok(()), |_, _| {})
            .unwrap();
        app.sync_groups();
        assert!(IsWindowVisible(hwnd).as_bool());
        host_request(&mut app, Request::SetLocked(group.clone(), true));
        assert!(app.live_sessions[0]
            .enable_input()
            .unwrap_err()
            .to_string()
            .contains("locked"));
        host_request(&mut app, Request::SetLocked(group.clone(), false));
        assert!(!app.live_sessions[0]
            .enable_input()
            .unwrap_err()
            .to_string()
            .contains("locked"));
        host_request(&mut app, Request::Delete(group));
        assert!(app.workspace.group_of(&unit).is_none());
        assert!(app.live_sessions[0].is_alive());
        app.shutdown_live().unwrap();
    }
}
#[test]
fn live_group_explicit_bulk_close_retires_static_and_live_but_preserves_other_host() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (hwnd, unit) = live(&mut app, true);
        let group = group(&mut app);
        command(hwnd, JOIN);
        app.pending_groups(false).unwrap();
        app.add_state(State::new(None, false).unwrap(), false)
            .unwrap();
        let other = app.documents[1].window.0;
        let old = app.documents[0].window.0;
        let original = app.restored_session.as_ref().unwrap().original.clone();
        host_request(&mut app, Request::Close(group.clone()));
        assert!(!IsWindow(Some(hwnd)).as_bool());
        assert!(!IsWindow(Some(old)).as_bool());
        assert!(IsWindow(Some(other)).as_bool());
        assert!(!app.workspace.contains(&unit));
        assert!(app.workspace.groups().iter().all(|item| item.id != group));
        assert!(app
            .restored_session
            .as_ref()
            .unwrap()
            .original
            .same_storage(&original));
        app.shutdown_live().unwrap();
    }
}
