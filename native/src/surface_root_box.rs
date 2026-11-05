//! 根盒约束与宿主视口分离；显式尺寸可溢出，但不得改变宿主裁剪范围。
use crate::{surface_length::Context, surface_scene::Node};

pub(crate) fn origin(
    node: &Node,
    viewport: [f32; 2],
    size: [f32; 2],
    mut context: Context,
) -> [f32; 2] {
    let position = node
        .layout()
        .get("position")
        .and_then(|value| value.as_str());
    if !matches!(position, Some("relative" | "absolute")) {
        return [0.; 2];
    }
    let mut result = [0.; 2];
    for (axis, (start, end)) in [("left", "right"), ("top", "bottom")]
        .into_iter()
        .enumerate()
    {
        context.axis = viewport[axis];
        let get = |key| crate::surface_layout_value::length(node, key, context);
        // LTR 起始边优先；relative 的末端偏移为负，absolute 从宿主末端扣最终尺寸。
        result[axis] = get(start).unwrap_or_else(|| {
            get(end).map_or(0., |offset| {
                if position == Some("absolute") {
                    viewport[axis] - offset - size[axis]
                } else {
                    -offset
                }
            })
        });
    }
    result
}

pub(crate) fn resolve(
    node: &Node,
    viewport: [f32; 2],
    intrinsic: [f32; 2],
    mut context: Context,
) -> Option<[f32; 2]> {
    let mut result = viewport;
    for axis in 0..2 {
        let floor = crate::surface_button::minimum(node, viewport[0], context, axis);
        context.axis = viewport[axis];
        let (dimension, min, max) = if axis == 0 {
            ("width", "minWidth", "maxWidth")
        } else {
            ("height", "minHeight", "maxHeight")
        };
        let get = |key| crate::surface_layout_value::length(node, key, context);
        let minimum = get(min).unwrap_or(0.).max(floor);
        let maximum = get(max).unwrap_or(32768.).max(minimum);
        let mut automatic = if axis == 0 { viewport[0] } else { intrinsic[1] };
        // 默认容器满宽由统一取值器返回；仅真正的 auto 使用下面的回退尺寸。
        if node
            .layout()
            .get("position")
            .is_some_and(|value| value == "absolute")
        {
            let (start, end) = if axis == 0 {
                ("left", "right")
            } else {
                ("top", "bottom")
            };
            if let (Some(start), Some(end)) = (get(start), get(end)) {
                automatic = (viewport[axis] - start - end).max(0.);
            }
        }
        result[axis] = get(dimension).unwrap_or(automatic).clamp(minimum, maximum);
    }
    result
        .iter()
        .all(|value| value.is_finite() && (0. ..=32768.).contains(value))
        .then_some(result)
}
