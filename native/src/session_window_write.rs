//! 单窗口几何合并候选；不写磁盘，不修改图片/参数/标注字段。
use crate::{
    pin_geometry::PinRect,
    session_placement::{Error, Space},
    session_window::{Offset, Rect},
};
use serde_json::{json, Value};

pub struct Geometry {
    pub frame: PinRect,
    pub minified: bool,
    /// 当前缩略窗展开后的实际物理框，调用方须先应用拖动位移，不能传初始旧框。
    /// 无 savedRect 的历史缩略节点须传 None，不能用其运行时同框展开回退伪造 savedRect。
    pub expanded: Option<PinRect>,
    pub normal_opacity: f32,
    pub mini_opacity: f32,
}

pub fn merge(mut original: Value, state: Geometry, space: Space) -> Result<Value, Error> {
    if ![state.normal_opacity, state.mini_opacity]
        .iter()
        .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
        || (!state.minified && state.expanded.is_some())
    {
        return Err(Error::Geometry);
    }
    let object = original.as_object_mut().ok_or(Error::Geometry)?;
    let old = rect_fields(object);
    let old_saved = object.get("savedRect").and_then(rect);
    let old_offset = object.get("cropOffset").and_then(|value| {
        Some(Offset {
            x: value.get("x")?.as_f64()?,
            y: value.get("y")?.as_f64()?,
        })
    });
    let old_expanded = old.and_then(|old| {
        crate::session_window::expanded_rect(
            old,
            object
                .get("minified")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            old_saved,
            old_offset,
        )
    });
    let current = space.logical(state.frame, old)?;
    let expanded = state
        .expanded
        .map(|frame| space.logical(frame, old_expanded))
        .transpose()?;
    for (key, value) in [
        ("x", current.x),
        ("y", current.y),
        ("w", current.w),
        ("h", current.h),
    ] {
        object.insert(key.into(), json!(value));
    }
    object.insert("minified".into(), state.minified.into());
    object.insert("opacityNormal".into(), json!(state.normal_opacity));
    object.insert("opacityMini".into(), json!(state.mini_opacity));
    if let Some(expanded) = expanded {
        let offset = Offset {
            x: current.x - expanded.x,
            y: current.y - expanded.y,
        };
        if crate::session_window::expanded_rect(current, true, Some(expanded), Some(offset))
            .is_none()
        {
            return Err(Error::Geometry);
        }
        object.insert(
            "savedRect".into(),
            json!({"x":expanded.x,"y":expanded.y,"w":expanded.w,"h":expanded.h}),
        );
        object.insert("cropOffset".into(), json!({"x":offset.x,"y":offset.y}));
    } else {
        // 无展开框的历史缩略节点保持无框语义；不能沿用上一次的旧 savedRect。
        object.remove("savedRect");
        // 缺少 savedRect 仍可能有独立取景偏移，退出缩略前不能丢弃。
        if !state.minified {
            object.remove("cropOffset");
        }
    }
    Ok(original)
}
fn rect(value: &Value) -> Option<Rect> {
    rect_fields(value.as_object()?)
}
fn rect_fields(value: &serde_json::Map<String, Value>) -> Option<Rect> {
    let rect = Rect {
        x: value.get("x")?.as_f64()?,
        y: value.get("y")?.as_f64()?,
        w: value.get("w")?.as_f64()?,
        h: value.get("h")?.as_f64()?,
    };
    rect.valid().then_some(rect)
}

#[cfg(test)]
#[path = "session_window_write_tests.rs"]
mod tests;
