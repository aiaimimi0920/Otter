//! 节点 chrome 与普通贴图通知设置；只读写已有会话字段，不拥有图像或 Art 执行策略。
use serde_json::{Map, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub hide_collapsed_endpoints: bool,
    pub notify_downstream: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            hide_collapsed_endpoints: false,
            notify_downstream: true,
        }
    }
}
impl Settings {
    pub fn read(window: &crate::session_window::Window<'_>) -> Result<Self, &'static str> {
        let execution: Option<Value> = window
            .execution_config
            .map(|raw| serde_json::from_str(raw.get()))
            .transpose()
            .map_err(|_| "Invalid node execution config")?;
        Ok(Self {
            hide_collapsed_endpoints: window.hide_collapsed_endpoints.unwrap_or(false),
            // Art 的通知权威仍为 Selection.execution；此值只供普通贴图使用。
            notify_downstream: execution
                .as_ref()
                .and_then(|value| value.get("propagation"))
                .and_then(|value| value.get("notifyDownstream"))
                .and_then(Value::as_bool)
                .unwrap_or(true),
        })
    }
    pub fn write(self, fields: &mut Map<String, Value>, art: bool) -> Result<(), &'static str> {
        if self.hide_collapsed_endpoints || fields.contains_key("hideCollapsedEndpoints") {
            fields.insert(
                "hideCollapsedEndpoints".into(),
                self.hide_collapsed_endpoints.into(),
            );
        }
        let declared = fields
            .get("executionConfig")
            .and_then(|config| config.get("propagation"))
            .and_then(|propagation| propagation.get("notifyDownstream"))
            .and_then(Value::as_bool)
            .is_some();
        // 未理解的既有配置原样保留；缺省 true 不应把 null 或扩展字段改写成新配置。
        if !art && (!self.notify_downstream || declared) {
            let config = fields
                .entry("executionConfig")
                .or_insert_with(|| Value::Object(Map::new()));
            if config.is_null() {
                *config = Value::Object(Map::new());
            }
            let config = config
                .as_object_mut()
                .ok_or("Invalid node execution config")?;
            let propagation = config
                .entry("propagation")
                .or_insert_with(|| Value::Object(Map::new()));
            if propagation.is_null() {
                *propagation = Value::Object(Map::new());
            }
            propagation
                .as_object_mut()
                .ok_or("Invalid node propagation config")?
                .insert("notifyDownstream".into(), self.notify_downstream.into());
        }
        Ok(())
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "unit_node_settings_tests.rs"]
mod tests;
