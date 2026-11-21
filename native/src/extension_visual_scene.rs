//! 扩展声明式场景的独立白名单；不接受图片/脚本/资源节点，事件只保留同一声明命令。
use crate::loom_extension_snapshot::text;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
pub(super) fn sanitize(
    value: &Value,
    command: Option<&str>,
    attached: bool,
    visible: Option<bool>,
) -> Option<Value> {
    struct Budget {
        nodes: usize,
        clicks: usize,
        bytes: usize,
        ids: BTreeSet<String>,
    }
    fn visit(
        value: &Value,
        command: Option<&str>,
        depth: usize,
        budget: &mut Budget,
    ) -> Option<Value> {
        if depth > 32 || budget.nodes == 0 {
            return None;
        }
        let source = value.as_object()?;
        let id = source.get("id")?.as_str().filter(|s| text(s, 128))?;
        let kind = source.get("type")?.as_str()?;
        if ![
            "row", "column", "stack", "text", "icon", "button", "textarea", "progress", "divider",
            "spacer",
        ]
        .contains(&kind)
            || !budget.ids.insert(id.into())
        {
            return None;
        }
        budget.nodes -= 1;
        let mut result = Map::new();
        result.insert("id".into(), id.into());
        result.insert("type".into(), kind.into());
        for key in ["props", "layout", "style", "accessibility"] {
            if let Some(value) = source.get(key) {
                if !value.is_null() && !value.is_object() {
                    return None;
                }
                budget.bytes = budget
                    .bytes
                    .checked_sub(serde_json::to_vec(value).ok()?.len())?;
                result.insert(key.into(), value.clone());
            }
        }
        let mut children = Vec::new();
        if let Some(value) = source.get("children") {
            for child in value.as_array()? {
                children.push(visit(child, command, depth + 1, budget)?);
            }
        }
        result.insert("children".into(), children.into());
        if let Some(click) = source
            .get("events")
            .and_then(|e| e.get("click"))
            .and_then(Value::as_str)
            .filter(|s| Some(*s) == command && budget.clicks > 0)
        {
            budget.clicks -= 1;
            result.insert("events".into(), serde_json::json!({"click":click}));
        }
        Some(Value::Object(result))
    }
    let mut budget = Budget {
        nodes: if attached { 512 } else { 64 },
        clicks: if attached { 256 } else { 16 },
        bytes: if attached { 256 * 1024 } else { 16 * 1024 },
        ids: BTreeSet::new(),
    };
    let mut scene = visit(value, command, 0, &mut budget)?;
    if let Some(visible) = visible {
        if !scene.get("props").is_some_and(Value::is_object) {
            scene["props"] = serde_json::json!({});
        }
        scene["props"]["visible"] = visible.into();
    }
    Some(scene)
}
