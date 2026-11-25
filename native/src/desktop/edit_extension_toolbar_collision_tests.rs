//! 合法的同名与分隔符不构成槽身份；每个按钮/组项均路由自己的命令。
use super::*;

fn catalog(rows: &[(&str, &str, Option<&str>)]) -> serde_json::Value {
    let mut data = crate::loom_extension_snapshot::toolbar_fixture();
    let plugin = data["plugins"][0].clone();
    let command = data["contributions"]["commands"][0].clone();
    let mut plugins = Vec::new();
    let mut commands = Vec::new();
    let mut menus = Vec::new();
    for (index, (owner, id, group)) in rows.iter().enumerate() {
        if !plugins
            .iter()
            .any(|p: &serde_json::Value| p["id"] == *owner)
        {
            let mut next = plugin.clone();
            next["id"] = json!(owner);
            next["scopeId"] = json!(owner);
            plugins.push(next);
        }
        let command_id = format!("collision.run.{index}");
        let mut next = command.clone();
        next["id"] = json!(format!("fixture-command-{index}"));
        next["commandId"] = json!(command_id);
        next["pluginId"] = json!(owner);
        next["scopeId"] = json!(owner);
        commands.push(next);
        let mut menu = json!({
            "id":id, "pluginId":owner, "scopeId":owner,
            "commandId":command_id, "placement":"hook.unit.toolbar",
            "title":format!("item-{index}"), "order":index,
        });
        if let Some(group) = group {
            menu["payload"] = json!({"groupId":group,"groupTitle":group});
        }
        menus.push(menu);
    }
    data["plugins"] = json!(plugins);
    data["contributions"]["commands"] = json!(commands);
    data["contributions"]["menus"] = json!(menus);
    data
}
unsafe fn assert_routes(rows: &[(&str, &str, Option<&str>)], expected: &[Vec<usize>]) {
    for (slot_index, members) in expected.iter().enumerate() {
        for (selection, member) in members.iter().enumerate() {
            // 真正提交会置 busy，每次路由用独立 HWND/状态，不绕过单槽门禁。
            let f = Fixture::new();
            setup(&f, catalog(rows));
            let visible = (ext::FIRST..ext::FIRST + 8)
                .filter(|id| {
                    let child = GetDlgItem(Some(f.panel.0), *id as i32).unwrap();
                    GetWindowLongW(child, GWL_STYLE) as u32 & WS_VISIBLE.0 != 0
                })
                .count();
            assert_eq!(visible, expected.len());
            let message = click(&f, ext::FIRST + slot_index as u16);
            let mut offered = false;
            ext::execute_with(f.owner.0, &f.state, message.wParam.0, |_, _, options| {
                offered = true;
                assert!(
                    rows[*member].2.is_some(),
                    "direct button must not open group"
                );
                assert_eq!(
                    options.iter().map(|i| i.label.as_str()).collect::<Vec<_>>(),
                    members
                        .iter()
                        .map(|i| format!("item-{i}"))
                        .collect::<Vec<_>>(),
                );
                assert!(options.iter().all(|i| i.enabled));
                Ok(Some(selection))
            })
            .unwrap();
            assert_eq!(offered, rows[*member].2.is_some());
            assert_eq!(
                extension_commands::requested_command(&f.state),
                Some(format!("collision.run.{member}")),
            );
        }
    }
}
#[test]
fn direct_id_and_group_id_remain_distinct_in_both_orderings() {
    unsafe {
        assert_routes(
            &[
                ("example", "shared", None),
                ("example", "first", Some("shared")),
                ("example", "second", Some("shared")),
            ],
            &[vec![0], vec![1, 2]],
        );
        assert_routes(
            &[
                ("example", "first", Some("shared")),
                ("example", "shared", None),
                ("example", "second", Some("shared")),
            ],
            &[vec![0, 2], vec![1]],
        );
    }
}
#[test]
fn delimiter_collisions_do_not_merge_other_plugins_groups_or_direct_buttons() {
    unsafe {
        assert_routes(
            &[
                ("example:tools", "one", Some("palette")),
                ("example", "two", Some("tools:palette")),
                ("example:tools", "three", Some("palette")),
                ("example", "four", Some("tools:palette")),
            ],
            &[vec![0, 2], vec![1, 3]],
        );
        assert_routes(
            &[
                ("example:tools", "direct", None),
                ("example", "tools:direct", None),
            ],
            &[vec![0], vec![1]],
        );
    }
}
