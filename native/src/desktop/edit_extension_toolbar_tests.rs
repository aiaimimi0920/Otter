//! Real BUTTON notifications, catalog/source evidence, and group selection; inject choices only at modal-return boundaries, without Loom/user input.
use super::*;
use crate::desktop::{edit_extension_toolbar as ext, extension_commands, extension_document};
use serde_json::json;
use std::sync::Arc;
#[path = "edit_extension_toolbar_collision_tests.rs"]
mod collisions;
unsafe fn setup(f: &Fixture, data: serde_json::Value) {
    f.state.extensions.borrow_mut().snapshot = Some(Arc::new(
        crate::loom_extension_snapshot::Snapshot::parse(&serde_json::to_vec(&data).unwrap())
            .unwrap(),
    ));
    extension_document::observe(&f.state);
    refresh(&f.state);
}
unsafe fn click(f: &Fixture, id: u16) -> MSG {
    SendMessageW(
        GetDlgItem(Some(f.panel.0), id as i32).unwrap(),
        BM_CLICK,
        None,
        None,
    );
    let mut message = MSG::default();
    assert!(PeekMessageW(
        &mut message,
        Some(f.owner.0),
        super::super::super::EXECUTE,
        super::super::super::EXECUTE,
        PM_REMOVE
    )
    .as_bool());
    message
}
#[test]
fn slots_use_baseline_grouping_english_order_and_eight_slot_limit_without_losing_group_items() {
    unsafe {
        let f = Fixture::new();
        let mut data = crate::loom_extension_snapshot::toolbar_fixture();
        let template = data["contributions"]["menus"][0].clone();
        let menus = data["contributions"]["menus"].as_array_mut().unwrap();
        menus.clear();
        for i in 0..20 {
            let mut menu = template.clone();
            menu["id"] = json!(format!("menu-{i}"));
            menu["order"] = json!(-1);
            menu["payload"] = json!({"groupId":"group","groupTitle":"所有工具"});
            menus.push(menu);
        }
        for id in ["z", "a-1", "A-2", "10", "2", "B-2", "b-1", "x", "c"] {
            let mut menu = template.clone();
            menu["id"] = json!(id);
            menu["title"] = json!(id);
            menu["order"] = json!(0);
            menus.push(menu);
        }
        setup(&f, data);
        let mut count = 0;
        for id in ext::FIRST..ext::FIRST + 8 {
            let child = GetDlgItem(Some(f.panel.0), id as i32).unwrap();
            if GetWindowLongW(child, GWL_STYLE) as u32 & WS_VISIBLE.0 != 0 {
                count += 1;
            }
        }
        assert_eq!(count, 8);
        for (index, expected) in ["10", "2", "a-1", "A-2", "b-1", "B-2", "c"]
            .iter()
            .enumerate()
        {
            let mut text = [0u16; 128];
            let len = GetWindowTextW(
                GetDlgItem(Some(f.panel.0), (ext::FIRST + 1 + index as u16) as i32).unwrap(),
                &mut text,
            );
            assert_eq!(String::from_utf16_lossy(&text[..len as usize]), *expected);
        }
        let message = click(&f, ext::FIRST);
        ext::execute_with(f.owner.0, &f.state, message.wParam.0, |_, _, options| {
            assert_eq!(options.len(), 20);
            Ok(None)
        })
        .unwrap();
        assert!(!extension_commands::busy(&f.state));
    }
}
#[test]
fn direct_contribution_button_routes_exact_command_once_and_does_not_change_tool() {
    unsafe {
        let f = Fixture::new();
        setup(&f, crate::loom_extension_snapshot::toolbar_fixture());
        let fixed = GetDlgItem(Some(f.panel.0), catalog::SHAPE as i32).unwrap();
        let tool = f.state.canvas.borrow().tool;
        let message = click(&f, ext::FIRST);
        assert_ne!(message.wParam.0 >> 32, 0);
        DispatchMessageW(&message);
        assert_eq!(
            extension_commands::requested_command(&f.state).as_deref(),
            Some("custom.run")
        );
        DispatchMessageW(&message);
        assert_eq!(
            extension_commands::requested_command(&f.state).as_deref(),
            Some("custom.run")
        );
        assert_eq!(f.state.canvas.borrow().tool, tool);
        assert_eq!(
            GetDlgItem(Some(f.panel.0), catalog::SHAPE as i32).unwrap(),
            fixed
        );
    }
}
#[test]
fn group_body_and_triangle_offer_same_snapshot_and_keep_all_fixed_labels_enabled() {
    unsafe {
        for triangle in [false, true] {
            let f = Fixture::new();
            setup(&f, crate::loom_extension_snapshot::toolbar_fixture());
            let id = ext::FIRST + 1;
            let message = click(&f, if triangle { id | 0x8000 } else { id });
            assert_eq!(message.wParam.0 as u16, id);
            let fixed: Vec<_> = control_ids()
                .into_iter()
                .map(|id| {
                    (
                        id,
                        IsWindowEnabled(GetDlgItem(Some(f.panel.0), id).unwrap()).as_bool(),
                    )
                })
                .collect();
            ext::execute_with(f.owner.0, &f.state, message.wParam.0, |_, _, options| {
                assert_eq!(
                    options.iter().map(|i| i.label.as_str()).collect::<Vec<_>>(),
                    ["工具一", "工具二"]
                );
                assert!(options.iter().all(|i| i.enabled));
                SendMessageW(f.owner.0, WM_NULL, None, None);
                for (id, enabled) in &fixed {
                    assert_eq!(
                        IsWindowEnabled(GetDlgItem(Some(f.panel.0), *id).unwrap()).as_bool(),
                        *enabled
                    );
                }
                Ok(Some(1))
            })
            .unwrap();
            assert_eq!(
                extension_commands::requested_command(&f.state).as_deref(),
                Some("custom.run")
            );
        }
    }
}
#[test]
fn stale_queued_buttons_cannot_rebind_to_a_new_catalog_or_changed_image() {
    unsafe {
        for case in 0..3 {
            let f = Fixture::new();
            setup(&f, crate::loom_extension_snapshot::toolbar_fixture());
            let message = click(&f, ext::FIRST);
            match case {
                0 => {
                    let mut data = crate::loom_extension_snapshot::toolbar_fixture();
                    data["generation"] = json!(8);
                    setup(&f, data);
                }
                1 => {
                    f.state.extensions.borrow_mut().snapshot = None;
                    refresh(&f.state);
                }
                _ => {
                    let mut c = f.state.canvas.borrow_mut();
                    c.editor
                        .begin(crate::editor::Tool::Line, crate::viewport::Point::default())
                        .unwrap();
                    c.editor
                        .extend(crate::viewport::Point { x: 20.0, y: 20.0 })
                        .unwrap();
                    c.editor.commit();
                }
            }
            DispatchMessageW(&message);
            assert!(!extension_commands::busy(&f.state));
        }
    }
}
#[test]
fn revoked_during_group_menu_or_disabled_command_never_enqueues_work() {
    unsafe {
        let f = Fixture::new();
        setup(&f, crate::loom_extension_snapshot::toolbar_fixture());
        let message = click(&f, ext::FIRST + 1);
        ext::execute_with(f.owner.0, &f.state, message.wParam.0, |_, _, _| {
            f.state.extensions.borrow_mut().snapshot = None;
            Ok(Some(0))
        })
        .unwrap();
        assert!(!extension_commands::busy(&f.state));
        let mut data = crate::loom_extension_snapshot::toolbar_fixture();
        data["plugins"][0]["effectivePermissions"] = json!([]);
        setup(&f, data);
        assert!(
            !IsWindowEnabled(GetDlgItem(Some(f.panel.0), ext::FIRST as i32).unwrap()).as_bool()
        );
        let message = click(&f, ext::FIRST + 1);
        ext::execute_with(f.owner.0, &f.state, message.wParam.0, |_, _, options| {
            assert!(options.iter().all(|i| !i.enabled));
            Ok(Some(0))
        })
        .unwrap();
        assert!(!extension_commands::busy(&f.state));
    }
}
