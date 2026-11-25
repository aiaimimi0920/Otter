//! Effective-property views for special leaves; preserve original wire fields without copying or rewriting the entire scene.
use serde_json::{Map, Value};
use std::sync::OnceLock;

pub fn switch_properties() -> &'static Map<String, Value> {
    // Legacy Switch label/span do not receive node style/layout; do not apply ordinary-control sizing.
    static VALUE: OnceLock<Map<String, Value>> = OnceLock::new();
    VALUE.get_or_init(Map::new)
}

pub fn icon_layout() -> &'static Map<String, Value> {
    static VALUE: OnceLock<Map<String, Value>> = OnceLock::new();
    VALUE.get_or_init(Map::new)
}

pub fn icon_style() -> &'static Map<String, Value> {
    static VALUE: OnceLock<Map<String, Value>> = OnceLock::new();
    VALUE.get_or_init(|| {
        // Frontend icons do not receive node style/layout; theme signal-primary maps to the sole signal yellow.
        serde_json::from_value(serde_json::json!({"color":"#d9ff38"})).unwrap()
    })
}

pub fn progress_layout() -> &'static Map<String, Value> {
    static VALUE: OnceLock<Map<String, Value>> = OnceLock::new();
    VALUE.get_or_init(|| {
        serde_json::from_value(serde_json::json!({"width":"100%","height":"1em"})).unwrap()
    })
}
pub fn progress_style() -> &'static Map<String, Value> {
    static VALUE: OnceLock<Map<String, Value>> = OnceLock::new();
    VALUE.get_or_init(Map::new)
}

pub fn divider_layout() -> &'static Map<String, Value> {
    static VALUE: OnceLock<Map<String, Value>> = OnceLock::new();
    VALUE.get_or_init(|| {
        // Match frontend hr: width:100%, margin:0, border-top:1px; do not accept node-style overrides.
        serde_json::from_value(serde_json::json!({"width":"100%","height":1,"minHeight":1}))
            .unwrap()
    })
}
pub fn divider_style() -> &'static Map<String, Value> {
    static VALUE: OnceLock<Map<String, Value>> = OnceLock::new();
    VALUE.get_or_init(|| {
        // Current dark-theme fallback for --nt-border-subtle; future theme switching must consistently use the theme owner.
        serde_json::from_value(serde_json::json!({"background":"#242b33"})).unwrap()
    })
}
