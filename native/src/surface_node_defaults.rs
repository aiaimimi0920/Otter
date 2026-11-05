//! 特殊叶子的有效属性视图；原始 wire 字段保留，避免复制或改写整个场景。
use serde_json::{Map, Value};
use std::sync::OnceLock;

pub(crate) fn switch_properties() -> &'static Map<String, Value> {
    // 旧 Switch label/span 不接收节点的 style/layout；不套用普通控件尺寸。
    static VALUE: OnceLock<Map<String, Value>> = OnceLock::new();
    VALUE.get_or_init(Map::new)
}

pub(crate) fn icon_layout() -> &'static Map<String, Value> {
    static VALUE: OnceLock<Map<String, Value>> = OnceLock::new();
    VALUE.get_or_init(Map::new)
}

pub(crate) fn icon_style() -> &'static Map<String, Value> {
    static VALUE: OnceLock<Map<String, Value>> = OnceLock::new();
    VALUE.get_or_init(|| {
        // 前端 icon 不接收节点 style/layout；主题 signal-primary 映射唯一信号黄。
        serde_json::from_value(serde_json::json!({"color":"#d9ff38"})).unwrap()
    })
}

pub(crate) fn progress_layout() -> &'static Map<String, Value> {
    static VALUE: OnceLock<Map<String, Value>> = OnceLock::new();
    VALUE.get_or_init(|| {
        serde_json::from_value(serde_json::json!({"width":"100%","height":"1em"})).unwrap()
    })
}
pub(crate) fn progress_style() -> &'static Map<String, Value> {
    static VALUE: OnceLock<Map<String, Value>> = OnceLock::new();
    VALUE.get_or_init(Map::new)
}

pub(crate) fn divider_layout() -> &'static Map<String, Value> {
    static VALUE: OnceLock<Map<String, Value>> = OnceLock::new();
    VALUE.get_or_init(|| {
        // 对齐前端 hr: width:100%, margin:0, border-top:1px；不接收节点样式覆盖。
        serde_json::from_value(serde_json::json!({"width":"100%","height":1,"minHeight":1}))
            .unwrap()
    })
}
pub(crate) fn divider_style() -> &'static Map<String, Value> {
    static VALUE: OnceLock<Map<String, Value>> = OnceLock::new();
    VALUE.get_or_init(|| {
        // 当前暗色主题 --nt-border-subtle 回退色；未来主题切换需统一接主题 owner。
        serde_json::from_value(serde_json::json!({"background":"#242b33"})).unwrap()
    })
}
