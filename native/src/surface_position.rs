//! 定位几何：相对偏移不改变占位；绝对定位需要明确的包含块和双轴锚点。
use super::*;
pub(super) fn absolute(node: &Node) -> bool {
    node.layout()
        .get("position")
        .is_some_and(|v| v == "absolute")
}
fn length(node: &Node, key: &str, axis: f32, mut context: Context) -> Option<f32> {
    context.axis = axis;
    crate::surface_layout_value::length(node, key, context)
}
pub(super) fn positioned<'a>(
    node: &'a Node,
    measured: &Measured<'_>,
    parent: [f32; 2],
    border: crate::surface_box::Insets,
    context: Context,
) -> Result<Box<'a>, Error> {
    // 绝对定位包含块是父 padding box，不包含边框；不能使用内容盒扣掉 padding。
    let parent = crate::surface_box::inner(parent, border);
    let mut origin = [0.; 2];
    let mut size = [0.; 2];
    for axis in 0..2 {
        let (start, end, dimension, min, max) = if axis == 0 {
            ("left", "right", "width", "minWidth", "maxWidth")
        } else {
            ("top", "bottom", "height", "minHeight", "maxHeight")
        };
        let get = |key| length(node, key, parent[axis], context);
        let before = get(start);
        let after = get(end);
        // CSS static-position 依赖祖先布局；尚无完整树 owner 时不得猜成左上角。
        if before.is_none() && after.is_none() {
            return Err(Error::Unsupported);
        }
        let preferred = get(dimension).unwrap_or_else(|| match (before, after) {
            (Some(a), Some(b)) => (parent[axis] - a - b).max(0.),
            _ => measured.size[axis],
        });
        let minimum = get(min).unwrap_or(0.).max(crate::surface_button::minimum(
            node, parent[0], context, axis,
        ));
        let maximum = get(max).unwrap_or(32768.).max(minimum);
        size[axis] = preferred.clamp(minimum, maximum);
        origin[axis] = before.unwrap_or_else(|| parent[axis] - after.unwrap_or(0.) - size[axis]);
        if !size[axis].is_finite()
            || !(0. ..=32768.).contains(&size[axis])
            || !origin[axis].is_finite()
        {
            return Err(Error::Input);
        }
    }
    Ok(Box {
        node,
        origin: std::array::from_fn(|a| origin[a] + border.start()[a]),
        size,
    })
}
pub(crate) fn relative(item: &mut Box<'_>, parent: [f32; 2], context: Context) {
    if !item
        .node
        .layout()
        .get("position")
        .is_some_and(|v| v == "relative")
    {
        return;
    }
    for (axis, (start, end)) in [("left", "right"), ("top", "bottom")]
        .into_iter()
        .enumerate()
    {
        let offset = length(item.node, start, parent[axis], context)
            .unwrap_or_else(|| -length(item.node, end, parent[axis], context).unwrap_or(0.));
        item.origin[axis] += offset;
    }
}
