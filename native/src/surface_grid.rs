//! 有界二维 Grid owner；输入是已测量场景，不新建渲染树、线程或动作权威。
use crate::{
    surface_container::{Box, Measured},
    surface_cross_axis as cross,
    surface_length::Context,
    surface_scene::{Kind, Node, MAX_NODES},
};
#[path = "surface_grid_axis.rs"]
mod axis;
#[path = "surface_grid_item.rs"]
mod item;
use item::align;
#[path = "surface_grid_placement.rs"]
mod placement;
#[path = "surface_grid_tracks.rs"]
mod tracks;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Error {
    Input,
    Identity,
    Syntax,
    Unsupported,
    Limit,
}
pub(crate) struct Layout<'a> {
    pub boxes: Vec<Box<'a>>,
    pub cells: Vec<[f32; 2]>,
    pub extent: [f32; 2],
}
pub(crate) fn layout<'a>(
    node: &'a Node,
    measured: &[Measured<'_>],
    size: [f32; 2],
    mut context: Context,
    font: &dyn Fn(&Node) -> Option<f32>,
    containing_width: f32,
    definite_height: bool,
) -> Result<Layout<'a>, Error> {
    let children = node.visible_children().collect::<Vec<_>>();
    if node.kind() != Kind::Grid || children.len() != measured.len() || children.len() > MAX_NODES {
        return Err(Error::Identity);
    }
    if size
        .iter()
        .any(|v| !v.is_finite() || !(0. ..=32768.).contains(v))
    {
        return Err(Error::Input);
    }
    if crate::surface_length::Length::Scalar(0., crate::surface_length::Unit::Px)
        .resolve(context)
        .is_none()
        || !containing_width.is_finite()
        || !(0. ..=32768.).contains(&containing_width)
    {
        return Err(Error::Input);
    }
    context.font = font(node).unwrap_or(context.font);
    if !context.font.is_finite() || !(0. ..=8192.).contains(&context.font) {
        return Err(Error::Input);
    }
    let padding = crate::surface_box::inset(node, containing_width, context);
    let inner = crate::surface_box::inner(size, padding);
    for (child, value) in children.iter().zip(measured) {
        let child_font = font(child).unwrap_or(context.font);
        if !child_font.is_finite() || !(0. ..=8192.).contains(&child_font) {
            return Err(Error::Input);
        }
        if child.id() != value.id {
            return Err(Error::Identity);
        }
        if value
            .size
            .iter()
            .any(|v| !v.is_finite() || !(0. ..=32768.).contains(v))
            || value
                .baseline
                .is_some_and(|v| !v.is_finite() || !(0. ..=32768.).contains(&v))
        {
            return Err(Error::Input);
        }
        if child
            .layout()
            .get("position")
            .is_some_and(|v| v == "absolute")
        {
            return Err(Error::Unsupported);
        }
    }
    if node
        .layout()
        .get("gridAutoFlow")
        .is_some_and(|v| v != "row")
    {
        return Err(Error::Unsupported);
    }
    let mut columns = tracks::list(node.layout().get("gridTemplateColumns"), true)?;
    let mut rows = tracks::list(node.layout().get("gridTemplateRows"), true)?;
    let (areas, counts) = placement::place(&children, [columns.len(), rows.len()])?;
    columns.resize(
        counts[0],
        tracks::automatic(node.layout().get("gridAutoColumns"))?,
    );
    rows.resize(
        counts[1],
        tracks::automatic(node.layout().get("gridAutoRows"))?,
    );
    let gaps = tracks::gaps(node, context, inner)?;
    let distributions = [
        axis::distribution(node.layout().get("justify"))?,
        axis::distribution(node.layout().get("alignContent"))?,
    ];
    let mut contributions = [Vec::new(), Vec::new()];
    for (child, value) in children.iter().zip(measured) {
        for (index, key) in ["width", "height"].into_iter().enumerate() {
            let mut local = context;
            local.axis = inner[index];
            local.font = font(child).unwrap_or(context.font);
            let explicit = child.layout_length(key).and_then(|v| v.resolve(local));
            contributions[index].push(explicit.unwrap_or(value.size[index]));
        }
    }
    let x_axis = axis::resolve(
        &columns,
        &areas,
        &contributions[0],
        0,
        axis::Space {
            available: Some(inner[0]),
            gap: gaps[0],
            context,
            distribution: distributions[0],
        },
    )?;
    // 基线组只影响单行项；先分列以使用与最终放置相同的尺寸/合成基线。
    let parent_y = align(node.layout().get("align"), cross::Align::Stretch)?;
    let parent_x = align(node.layout().get("justifyItems"), cross::Align::Stretch)?;
    if parent_x == cross::Align::Baseline {
        return Err(Error::Unsupported);
    }
    let mut baseline = vec![0_f32; rows.len()];
    let mut descent = vec![0_f32; rows.len()];
    for ((child, value), area) in children.iter().zip(measured).zip(&areas) {
        if align(child.layout().get("alignSelf"), parent_y)? == cross::Align::Baseline {
            if area.span[1] != 1 {
                return Err(Error::Unsupported);
            }
            let local = Context {
                font: font(child).unwrap_or(context.font),
                ..context
            };
            let (b, height) = item::baseline(
                child,
                value,
                [x_axis.span(area.start[0], area.span[0]), inner[1]],
                local,
            )?;
            baseline[area.start[1]] = baseline[area.start[1]].max(b);
            descent[area.start[1]] = descent[area.start[1]].max((height - b).max(0.));
        }
    }
    for (index, area) in areas.iter().enumerate() {
        if area.span[1] == 1
            && align(children[index].layout().get("alignSelf"), parent_y)? == cross::Align::Baseline
        {
            contributions[1][index] =
                contributions[1][index].max(baseline[area.start[1]] + descent[area.start[1]]);
        }
    }
    let axes = [
        x_axis,
        axis::resolve(
            &rows,
            &areas,
            &contributions[1],
            1,
            axis::Space {
                available: definite_height.then_some(inner[1]),
                gap: gaps[1],
                context,
                distribution: distributions[1],
            },
        )?,
    ];
    let mut boxes = Vec::with_capacity(children.len());
    let mut cells = Vec::with_capacity(children.len());
    for ((child, value), area) in children.iter().zip(measured).zip(areas) {
        let cell: [f32; 2] = std::array::from_fn(|a| axes[a].span(area.start[a], area.span[a]));
        let mut origin = padding.start();
        let mut allocated = [0.; 2];
        for a in 0..2 {
            let mut local = context;
            local.axis = cell[a];
            local.font = font(child).unwrap_or(context.font);
            if !local.font.is_finite() || !(0. ..=8192.).contains(&local.font) {
                return Err(Error::Input);
            }
            let own = if a == 0 { "justifySelf" } else { "alignSelf" };
            let parent_key = if a == 0 { "justifyItems" } else { "align" };
            let default = if child.kind() == Kind::Image && !node.layout().contains_key(parent_key)
            {
                cross::Align::Start
            } else if a == 0 {
                parent_x
            } else {
                parent_y
            };
            let selected = align(child.layout().get(own), default)?;
            if a == 0 && selected == cross::Align::Baseline {
                return Err(Error::Unsupported);
            }
            let item = item::arrange(child, value, cell, a, selected, local)?;
            let offset = if selected == cross::Align::Baseline {
                baseline[area.start[1]] - value.baseline.unwrap_or(item.size)
            } else {
                item.offset
            };
            origin[a] += axes[a].offsets[area.start[a]] + offset;
            allocated[a] = item.size;
        }
        let mut item = Box {
            node: child,
            origin,
            size: allocated,
        };
        let mut local = context;
        local.font = font(child).unwrap_or(context.font);
        crate::surface_container::positioning::relative(&mut item, cell, local);
        cells.push(cell);
        boxes.push(item);
    }
    Ok(Layout {
        boxes,
        cells,
        extent: std::array::from_fn(|a| axes[a].extent + padding.total()[a]),
    })
}
#[cfg(test)]
#[path = "surface_grid_tests.rs"]
mod tests;
