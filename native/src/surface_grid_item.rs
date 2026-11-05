//! Grid item 的对齐/尺寸约束；基线预量与最终放置共享同一尺寸规则。
use super::{cross, Error};
use crate::{
    surface_container::Measured,
    surface_length::{Context, Length, Unit},
    surface_scene::{Kind, Node},
};
pub(super) fn align(
    value: Option<&serde_json::Value>,
    default: cross::Align,
) -> Result<cross::Align, Error> {
    if value.is_some_and(|v| !v.is_string()) {
        return Err(Error::Syntax);
    }
    Ok(match value.and_then(|v| v.as_str()) {
        None | Some("auto" | "normal") => default,
        Some("stretch") => cross::Align::Stretch,
        Some("start" | "flex-start") => cross::Align::Start,
        Some("end" | "flex-end") => cross::Align::End,
        Some("center") => cross::Align::Center,
        Some("baseline") => cross::Align::Baseline,
        _ => return Err(Error::Unsupported),
    })
}
pub(super) fn arrange(
    child: &Node,
    value: &Measured<'_>,
    cell: [f32; 2],
    axis: usize,
    selected: cross::Align,
    mut context: Context,
) -> Result<cross::Placement, Error> {
    context.axis = cell[axis];
    let (dimension, min, max) = if axis == 0 {
        ("width", "minWidth", "maxWidth")
    } else {
        ("height", "minHeight", "maxHeight")
    };
    let get = |key| child.layout_length(key).and_then(|v| v.resolve(context));
    let minimum = get(min).unwrap_or(0.).max(crate::surface_button::minimum(
        child, cell[0], context, axis,
    ));
    let maximum = get(max)
        .unwrap_or(if child.kind() == Kind::Image && axis == 0 {
            cell[axis]
        } else {
            32768.
        })
        .max(minimum);
    let item = cross::Item {
        intrinsic: value.size[axis],
        explicit: get(dimension),
        minimum,
        maximum,
        baseline: value.baseline,
    };
    Ok(cross::arrange(
        &[item],
        cell[axis],
        if selected == cross::Align::Baseline {
            cross::Align::Start
        } else {
            selected
        },
    )
    .map_err(|_| Error::Input)?
    .items[0])
}
pub(super) fn baseline(
    child: &Node,
    value: &Measured<'_>,
    cell: [f32; 2],
    context: Context,
) -> Result<(f32, f32), Error> {
    // baseline 组先决定 auto 行高；百分比高度循环依赖尚未实现，不能拿父整高假算。
    for key in ["height", "minHeight", "maxHeight"] {
        if matches!(
            child.layout_length(key),
            Some(Length::Scalar(_, Unit::Percent) | Length::BoundedPosition { .. })
        ) {
            return Err(Error::Unsupported);
        }
    }
    let size = arrange(child, value, cell, 1, cross::Align::Baseline, context)?.size;
    Ok((value.baseline.unwrap_or(size), size))
}
