//! 已测量场景子节点的行列布局接线；保留节点身份和显式定位边界。
#[path = "surface_position.rs"]
pub(crate) mod positioning;
#[path = "surface_stack.rs"]
mod stack;
use crate::{
    surface_cross_axis as cross, surface_flow as flow,
    surface_length::Context,
    surface_scene::{Kind, Node},
};
pub struct Measured<'a> {
    pub id: &'a str,
    pub size: [f32; 2],
    pub baseline: Option<f32>,
}
#[derive(Debug)]
pub struct Box<'a> {
    pub node: &'a Node,
    pub origin: [f32; 2],
    pub size: [f32; 2],
}
#[derive(Debug)]
pub enum Error {
    Input,
    Identity,
    Unsupported,
    Flow(flow::Error),
    Cross(cross::Error),
}
pub fn arrange<'a>(
    node: &'a Node,
    measured: &[Measured<'_>],
    size: [f32; 2],
    context: Context,
) -> Result<Vec<Box<'a>>, Error> {
    arrange_with_fonts(node, measured, size, context, &|_| None)
}
/// 字号已经按场景继承解析；布局层只使用计算值，不重复解析 fontSize。
pub fn arrange_with_fonts<'a>(
    node: &'a Node,
    measured: &[Measured<'_>],
    size: [f32; 2],
    context: Context,
    font: &dyn Fn(&Node) -> Option<f32>,
) -> Result<Vec<Box<'a>>, Error> {
    arrange_in(node, measured, size, context, font, size[0])
}
pub(crate) fn arrange_in<'a>(
    node: &'a Node,
    measured: &[Measured<'_>],
    size: [f32; 2],
    mut context: Context,
    font: &dyn Fn(&Node) -> Option<f32>,
    containing_width: f32,
) -> Result<Vec<Box<'a>>, Error> {
    context.font = font(node).unwrap_or(context.font);
    if node.kind() == Kind::Grid {
        return crate::surface_grid::layout(
            node,
            measured,
            size,
            context,
            font,
            containing_width,
            true,
        )
        .map(|value| value.boxes)
        .map_err(|_| Error::Unsupported);
    }
    let row = match node.kind() {
        Kind::Row => true,
        Kind::View | Kind::Column | Kind::Scroll | Kind::Stack => false,
        _ => return Err(Error::Unsupported),
    };
    if measured.len() != node.visible_children().count()
        || measured.len() > crate::surface_scene::MAX_NODES
    {
        return Err(Error::Identity);
    }
    if size
        .iter()
        .any(|v| !v.is_finite() || !(0. ..=32768.).contains(v))
    {
        return Err(Error::Input);
    }
    for (child, measure) in node.visible_children().zip(measured) {
        if font(child).is_some_and(|v| !v.is_finite() || !(0. ..=8192.).contains(&v)) {
            return Err(Error::Input);
        }
        if measure
            .size
            .iter()
            .any(|v| !v.is_finite() || !(0. ..=32768.).contains(v))
            || measure
                .baseline
                .is_some_and(|v| !v.is_finite() || !(0. ..=32768.).contains(&v))
        {
            return Err(Error::Input);
        }
        if child.id() != measure.id {
            return Err(Error::Identity);
        }
        if positioning::absolute(child)
            && !node
                .layout()
                .get("position")
                .is_some_and(|v| v == "relative" || v == "absolute")
        {
            return Err(Error::Unsupported);
        }
    }
    context.axis = size[0];
    // cqw/cqh 使用宿主查询容器，不能被每层普通 row/column 的尺寸替换。
    if crate::surface_length::Length::Scalar(0., crate::surface_length::Unit::Px)
        .resolve(context)
        .is_none()
    {
        return Err(Error::Input);
    }
    let padding = crate::surface_box::inset(node, containing_width, context);
    let inner = crate::surface_box::inner(size, padding);
    if node.kind() == Kind::Stack {
        return stack::arrange(node, measured, size, inner, padding, context, font);
    }
    let main = usize::from(!row);
    let other = 1 - main;
    context.axis = inner[main];
    let gap = node
        .layout_length("gap")
        .and_then(|v| v.resolve(context))
        .unwrap_or(0.);
    let justify = match node.layout().get("justify").and_then(|v| v.as_str()) {
        Some("center") => flow::Justify::Center,
        Some("flex-end") => flow::Justify::End,
        Some("space-between") => flow::Justify::Between,
        Some("space-around") => flow::Justify::Around,
        Some("space-evenly") => flow::Justify::Evenly,
        _ => flow::Justify::Start,
    };
    let align = match node.layout().get("align").and_then(|v| v.as_str()) {
        Some("flex-start") => cross::Align::Start,
        Some("center") => cross::Align::Center,
        Some("flex-end") => cross::Align::End,
        Some("baseline") if row => cross::Align::Baseline,
        Some("baseline") => return Err(Error::Unsupported),
        _ => cross::Align::Stretch,
    };
    let mut main_items = Vec::with_capacity(measured.len());
    let mut cross_items = Vec::with_capacity(measured.len());
    for (child, measure) in node.visible_children().zip(measured) {
        if positioning::absolute(child) {
            continue;
        }
        let axis = |index: usize| {
            let mut c = context;
            c.font = font(child).unwrap_or(c.font);
            c.axis = inner[index];
            let (key, min, max) = if index == 0 {
                ("width", "minWidth", "maxWidth")
            } else {
                ("height", "minHeight", "maxHeight")
            };
            let get = |key| crate::surface_layout_value::length(child, key, c);
            let minimum = get(min)
                .unwrap_or(0.)
                .max(crate::surface_button::minimum(child, inner[0], c, index));
            (get(key), minimum, get(max).unwrap_or(32768.).max(minimum))
        };
        let (explicit, mut minimum, maximum) = axis(main);
        // 替换元素的 auto 主轴最小值保留内容/比例基准；不能把两张图任意压扁。
        let min_key = if main == 0 { "minWidth" } else { "minHeight" };
        if child.kind() == Kind::Image
            && child
                .layout_length(min_key)
                .is_none_or(|v| v == crate::surface_length::Length::Auto)
            && !crate::surface_overflow::computed(child)[main].clips()
        {
            minimum = minimum.max(
                measure.size[main]
                    .min(maximum)
                    .min(explicit.unwrap_or(32768.)),
            );
        }
        let grow = child
            .layout()
            .get("grow")
            .and_then(|v| v.as_f64())
            .filter(|v| v.is_finite())
            .unwrap_or(if child.kind() == Kind::Spacer { 1. } else { 0. })
            .clamp(0., 100.) as f32;
        main_items.push(flow::Item {
            basis: explicit.unwrap_or(measure.size[main]),
            minimum,
            maximum,
            grow,
            shrink: 1.,
        });
        let (explicit, minimum, maximum) = axis(other);
        cross_items.push(cross::Item {
            intrinsic: measure.size[other],
            explicit,
            minimum,
            maximum,
            baseline: if row { measure.baseline } else { None },
        });
    }
    let main_layout = flow::arrange(&main_items, inner[main], gap, justify).map_err(Error::Flow)?;
    let cross_layout = cross::arrange(&cross_items, inner[other], align).map_err(Error::Cross)?;
    let mut normal = 0;
    let border = crate::surface_box::borders(node, context);
    node.visible_children()
        .zip(measured)
        .map(|(node, measure)| {
            let mut child_context = context;
            child_context.font = font(node).unwrap_or(context.font);
            if positioning::absolute(node) {
                return positioning::positioned(node, measure, size, border, child_context);
            }
            let a = main_layout.items[normal];
            let b = cross_layout.items[normal];
            normal += 1;
            let mut origin = padding.start();
            let mut size = [0.; 2];
            origin[main] += a.offset;
            origin[other] += b.offset;
            size[main] = a.size;
            size[other] = b.size;
            let mut result = Box { node, origin, size };
            positioning::relative(&mut result, inner, child_context);
            Ok(result)
        })
        .collect()
}
#[cfg(test)]
#[path = "surface_container_tests.rs"]
mod tests;
