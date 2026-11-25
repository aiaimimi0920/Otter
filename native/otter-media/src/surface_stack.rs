//! stack 的共享区域重叠布局；保留场景顺序，不引入行列 gap 或 grow。
use super::*;
pub(super) fn arrange<'a>(
    node: &'a Node,
    measured: &[Measured<'_>],
    parent: [f32; 2],
    inner: [f32; 2],
    padding: crate::surface_box::Insets,
    context: Context,
    font: &dyn Fn(&Node) -> Option<f32>,
) -> Result<Vec<Box<'a>>, Error> {
    // 多轨道和内容分布需独立 grid 算法，不能借用 flex justify 冒充。
    if node
        .layout()
        .get("justify")
        .and_then(|v| v.as_str())
        .is_some_and(|v| v != "flex-start")
    {
        return Err(Error::Unsupported);
    }
    let align = match node.layout().get("align").and_then(|v| v.as_str()) {
        Some("flex-start") => cross::Align::Start,
        Some("center") => cross::Align::Center,
        Some("flex-end") => cross::Align::End,
        Some("baseline") => cross::Align::Baseline,
        _ => cross::Align::Stretch,
    };
    let mut horizontal = Vec::with_capacity(measured.len());
    let mut vertical = Vec::with_capacity(measured.len());
    for (child, measure) in node.visible_children().zip(measured) {
        if positioning::absolute(child) {
            continue;
        }
        for (axis, available) in inner.iter().copied().enumerate() {
            let (dimension, min, max) = if axis == 0 {
                ("width", "minWidth", "maxWidth")
            } else {
                ("height", "minHeight", "maxHeight")
            };
            let mut c = context;
            c.font = font(child).unwrap_or(c.font);
            c.axis = available;
            let get = |key| crate::surface_layout_value::length(child, key, c);
            let minimum = get(min)
                .unwrap_or(0.)
                .max(crate::surface_button::minimum(child, inner[0], c, axis));
            let item = cross::Item {
                intrinsic: measure.size[axis],
                explicit: get(dimension),
                minimum,
                maximum: get(max).unwrap_or(32768.).max(minimum),
                baseline: if axis == 1 { measure.baseline } else { None },
            };
            if axis == 0 {
                horizontal.push(item);
            } else {
                vertical.push(item);
            }
        }
    }
    let x = cross::arrange(&horizontal, inner[0], cross::Align::Stretch).map_err(Error::Cross)?;
    let y = cross::arrange(&vertical, inner[1], align).map_err(Error::Cross)?;
    let mut normal = 0;
    let border = crate::surface_box::borders(node, context);
    node.visible_children()
        .zip(measured)
        .map(|(node, measure)| {
            let mut child_context = context;
            child_context.font = font(node).unwrap_or(context.font);
            if positioning::absolute(node) {
                return positioning::positioned(node, measure, parent, border, child_context);
            }
            let a = x.items[normal];
            let b = y.items[normal];
            normal += 1;
            let mut result = Box {
                node,
                origin: [padding.left + a.offset, padding.top + b.offset],
                size: [a.size, b.size],
            };
            positioning::relative(&mut result, inner, child_context);
            Ok(result)
        })
        .collect()
}
