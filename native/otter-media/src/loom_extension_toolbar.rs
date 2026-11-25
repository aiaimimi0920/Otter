//! Mainline toolbar-menu declaration projection; exposes only trusted same-scope metadata, without executing commands or issuing resource permissions.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolbarItem {
    pub id: String,
    pub plugin: String,
    pub command: String,
    pub title: String,
    pub order: i32,
    pub group: Option<String>,
    pub group_title: Option<String>,
    pub disabled_title: Option<String>,
    pub available: bool,
}
fn payload_text(item: &Contribution, key: &str) -> Option<String> {
    // Menu metadata matches mainline nestedPayload; its wrapper does not require the command payload's schema marker.
    let outer = item.payload.as_ref()?.as_object()?;
    let payload = outer
        .get("payload")
        .and_then(Value::as_object)
        .unwrap_or(outer);
    let text = payload.get(key)?.as_str()?;
    (text.encode_utf16().count() <= 128 && !text.trim().is_empty()).then(|| text.trim().into())
}
impl Snapshot {
    /// when controls visibility, and command when controls availability; execution still rechecks through command/invoke.
    pub fn toolbar(&self, context: &Context<'_>) -> Vec<ToolbarItem> {
        self.0.contributions["menus"]
            .iter()
            .filter_map(|menu| {
                if menu.placement.as_deref() != Some("hook.unit.toolbar")
                    || self.owner(&menu.scope_id).is_none()
                    || !menu.predicate.as_ref().unwrap().evaluate(context)
                {
                    return None;
                }
                let command_id = menu.command_id.as_deref()?;
                let command = self.0.contributions["commands"]
                    .iter()
                    .rev()
                    .find(|command| {
                        command.command_id.as_deref().unwrap_or(&command.id) == command_id
                    })?;
                if command.plugin_id != menu.plugin_id || command.scope_id != menu.scope_id {
                    return None;
                }
                Some(ToolbarItem {
                    id: menu.id.clone(),
                    plugin: menu.plugin_id.clone(),
                    command: command_id.into(),
                    title: menu
                        .title
                        .as_ref()
                        .or(command.title.as_ref())
                        .unwrap_or(&command.id)
                        .clone(),
                    order: menu.order.unwrap_or(0),
                    group: payload_text(menu, "groupId"),
                    group_title: payload_text(menu, "groupTitle"),
                    disabled_title: payload_text(menu, "disabledTitle"),
                    available: self
                        .command(command_id, context)
                        .is_some_and(|c| c.permissions().is_some()),
                })
            })
            .collect()
    }
}
#[cfg(any(test, feature = "test-support"))]
#[path = "loom_extension_toolbar_tests.rs"]
pub mod tests;
