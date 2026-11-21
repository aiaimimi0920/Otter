//! 实际目录 schema 的菜单可见性、权限、同 scope 归属与两种 payload 包装。
use super::*;
use serde_json::json;
pub(crate) fn fixture() -> Value {
    let mut data = super::super::tests::fixture();
    let commands = data["contributions"]["commands"].as_array_mut().unwrap();
    commands[0]["id"] = json!("run-command");
    commands[0]["commandId"] = json!("custom.run");
    commands[0]["title"] = json!("执行");
    commands[0]["when"] = json!("unit.hasImage");
    commands[0]["pluginId"] = json!("example/tools");
    data["plugins"][0]["id"] = json!("example/tools");
    data["contributions"]["menus"] = json!([
        {"id":"direct", "pluginId":"example/tools", "scopeId":"scope:ocr", "commandId":"custom.run", "placement":"hook.unit.toolbar", "order":1},
        {"id":"group-one", "pluginId":"example/tools", "scopeId":"scope:ocr", "commandId":"custom.run", "placement":"hook.unit.toolbar", "title":"工具一", "order":2,
          "payload":{"payload":{"groupId":" group ","groupTitle":"工具组"}}},
        {"id":"group-two", "pluginId":"example/tools", "scopeId":"scope:ocr", "commandId":"custom.run", "placement":"hook.unit.toolbar", "title":"工具二", "order":3,
          "payload":{"groupId":"group","groupTitle":"忽略后项标题","disabledTitle":"当前不可用"}}
    ]);
    data
}
fn context(image: bool) -> Context<'static> {
    Context {
        kind: Some(crate::extension_when::UnitKind::Sticker),
        has_image: image,
        attachment_types: &[],
    }
}
fn parse(data: Value) -> Snapshot {
    Snapshot::parse(&serde_json::to_vec(&data).unwrap()).unwrap()
}
#[test]
fn same_scope_menus_keep_disabled_commands_visible_and_unwrap_both_payloads() {
    let snapshot = parse(fixture());
    let items = snapshot.toolbar(&context(true));
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].title, "执行");
    assert_eq!(items[1].group.as_deref(), Some("group"));
    assert_eq!(items[2].disabled_title.as_deref(), Some("当前不可用"));
    assert!(items.iter().all(|i| i.available));
    assert!(snapshot
        .toolbar(&context(false))
        .iter()
        .all(|i| !i.available));
}
#[test]
fn menu_when_placement_owner_and_command_ownership_are_independent_gates() {
    for case in 0..7 {
        let mut data = fixture();
        match case {
            0 => data["plugins"][0]["trustStatus"] = json!("unsigned_developer"),
            1 => data["plugins"][0]["trustStatus"] = json!("revoked"),
            2 => data["plugins"][0]["trustStatus"] = json!("untrusted"),
            3 => {
                for menu in data["contributions"]["menus"].as_array_mut().unwrap() {
                    menu["when"] = json!("unit.kind == 'art'");
                }
            }
            4 => {
                for menu in data["contributions"]["menus"].as_array_mut().unwrap() {
                    menu["placement"] = json!("hook.commandPalette");
                }
            }
            5 => {
                for menu in data["contributions"]["menus"].as_array_mut().unwrap() {
                    menu["commandId"] = json!("absent");
                }
            }
            _ => {
                let mut other = data["plugins"][0].clone();
                other["id"] = json!("other");
                other["scopeId"] = json!("other-scope");
                data["plugins"].as_array_mut().unwrap().push(other);
                data["contributions"]["commands"][0]["pluginId"] = json!("other");
                data["contributions"]["commands"][0]["scopeId"] = json!("other-scope");
            }
        }
        assert!(
            parse(data).toolbar(&context(true)).is_empty(),
            "case {case}"
        );
    }
}
#[test]
fn invalid_group_metadata_does_not_create_a_group_or_authorize_a_command() {
    let mut data = fixture();
    data["contributions"]["menus"][0]["payload"] =
        json!({"groupId":"x".repeat(129),"groupTitle":42});
    data["plugins"][0]["effectivePermissions"] = json!([]);
    let items = parse(data).toolbar(&context(true));
    assert!(items[0].group.is_none() && items.iter().all(|i| !i.available));
}
#[test]
fn duplicate_command_ids_follow_the_baseline_last_definition_without_using_earlier_permissions() {
    let mut data = fixture();
    let mut command = data["contributions"]["commands"][0].clone();
    command["id"] = json!("latest-command");
    command["title"] = json!("最新声明");
    command["payload"]["payload"]["permissions"] = json!(["not.granted"]);
    data["contributions"]["commands"]
        .as_array_mut()
        .unwrap()
        .push(command);
    let snapshot = parse(data);
    let items = snapshot.toolbar(&context(true));
    assert_eq!(items[0].title, "最新声明");
    assert!(items.iter().all(|i| !i.available));
    assert!(snapshot
        .command("custom.run", &context(true))
        .unwrap()
        .permissions()
        .is_none());
}
