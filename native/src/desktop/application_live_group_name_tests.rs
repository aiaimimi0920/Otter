//! 真实系统菜单请求配合注入名称选择，验证模态事务；不冒充可见名称框人工输入验收。
use super::*;
use crate::{
    desktop::composition::live::window::groups::{Request, CREATE, RENAME},
    workspace::{Group, Workspace},
    workspace_id::UnitId,
};
unsafe fn request(app: &Application, hwnd: HWND, command: usize) -> Request {
    app.sync_groups();
    SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(command)), Some(LPARAM(0)));
    app.live_sessions[0].take_group_request().unwrap()
}
#[test]
fn live_group_name_create_and_rename_preserve_identity_and_are_saved() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (hwnd, unit) = live(&mut app, true);
        let identity = app.workspace.identity_stamp();
        let create = request(&app, hwnd, CREATE);
        assert!(app
            .edit_live_group_with(&unit, &create, |app, owner, old| {
                assert_eq!(owner, hwnd);
                assert!(app.modals.blocked());
                assert!(old.is_empty());
                Some("截图组".into())
            })
            .unwrap());
        assert!(!app.modals.blocked());
        assert!(identity.matches(&app.workspace));
        let group = app.workspace.group_of(&unit).unwrap().id.clone();
        let rename = request(&app, hwnd, RENAME);
        assert!(app
            .edit_live_group_with(&unit, &rename, |app, _, old| {
                assert!(app.modals.blocked());
                assert_eq!(old, "截图组");
                Some("Live 编辑组".into())
            })
            .unwrap());
        assert_eq!(app.workspace.group_of(&unit).unwrap().id, group);
        app.request_save_candidate().unwrap();
        let saved = super::save::prepared(&mut app).unwrap();
        let decoded = crate::session_document::decode(&saved.candidate.bytes).unwrap();
        assert_eq!(
            decoded.workspace.group_of(&unit).unwrap().name,
            "Live 编辑组"
        );
        assert!(IsWindow(Some(hwnd)).as_bool());
        app.shutdown_live().unwrap();
    }
}
#[test]
fn live_group_name_cancel_invalid_and_full_catalog_publish_no_partial_group() {
    unsafe {
        let environment = Environment::new();
        for mode in 0..4 {
            let mut app = environment.app();
            let (hwnd, unit) = live(&mut app, true);
            if mode == 3 {
                for index in 0..crate::workspace::MAX_UNITS {
                    app.workspace
                        .upsert_group(Group {
                            id: UnitId::parse(&format!("g{index}")).unwrap(),
                            name: format!("group {index}"),
                            hidden: false,
                            locked: false,
                        })
                        .unwrap();
                }
            }
            let before = app.workspace.groups().to_vec();
            let create = request(&app, hwnd, CREATE);
            let result = app.edit_live_group_with(&unit, &create, |_, _, _| match mode {
                0 => None,
                1 => Some(String::new()),
                2 => Some("x".repeat(257)),
                _ => Some("valid".into()),
            });
            if mode == 0 {
                assert!(!result.unwrap());
            } else {
                assert!(result.is_err());
            }
            assert_eq!(app.workspace.groups(), before);
            assert!(app.workspace.group_of(&unit).is_none());
            assert!(!app.modals.blocked());
            assert!(app.live_sessions[0].is_alive());
            app.shutdown_live().unwrap();
        }
    }
}
#[test]
fn live_group_name_rejects_closed_owner_replacement_and_group_change_during_dialog() {
    unsafe {
        let environment = Environment::new();
        for mode in 0..3 {
            let mut app = environment.app();
            let (hwnd, unit) = live(&mut app, true);
            let create = request(&app, hwnd, CREATE);
            assert!(app
                .edit_live_group_with(&unit, &create, |app, _, _| {
                    match mode {
                        0 => {
                            DestroyWindow(hwnd).unwrap();
                        }
                        1 => {
                            app.workspace =
                                Workspace::with_initial(app.documents[0].state.unit_id.clone());
                        }
                        _ => {
                            app.workspace
                                .upsert_group(Group {
                                    id: UnitId::parse("other").unwrap(),
                                    name: "other".into(),
                                    hidden: false,
                                    locked: false,
                                })
                                .unwrap();
                        }
                    }
                    Some("stale".into())
                })
                .is_err());
            assert!(app
                .workspace
                .groups()
                .iter()
                .all(|group| group.name != "stale"));
            assert!(!app.modals.blocked());
            app.shutdown_live().unwrap();
        }
    }
}
