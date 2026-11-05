//! 行/列交叉轴几何；基线来自测量 owner，无文字基线的盒子按底边合成。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Stretch,
    Start,
    Center,
    End,
    Baseline,
}
#[derive(Clone, Copy, Debug)]
pub struct Item {
    pub intrinsic: f32,
    pub explicit: Option<f32>,
    pub minimum: f32,
    pub maximum: f32,
    pub baseline: Option<f32>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub offset: f32,
    pub size: f32,
}
#[derive(Debug)]
pub struct CrossAxis {
    pub items: Vec<Placement>,
    pub baseline: Option<f32>,
    pub overflow_before: f32,
    pub overflow_after: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Input,
    Limit,
}
fn valid(value: f32) -> bool {
    value.is_finite() && (0. ..=32768.).contains(&value)
}
pub fn arrange(items: &[Item], available: f32, align: Align) -> Result<CrossAxis, Error> {
    if items.len() > crate::surface_scene::MAX_NODES {
        return Err(Error::Limit);
    }
    if !valid(available)
        || items.iter().any(|item| {
            !valid(item.intrinsic)
                || !valid(item.minimum)
                || !valid(item.maximum)
                || item.minimum > item.maximum
                || item.explicit.is_some_and(|v| !valid(v))
                || item.baseline.is_some_and(|v| !valid(v))
        })
    {
        return Err(Error::Input);
    }
    let sizes: Vec<f32> = items
        .iter()
        .map(|item| {
            item.explicit
                .unwrap_or(if align == Align::Stretch {
                    available
                } else {
                    item.intrinsic
                })
                .clamp(item.minimum, item.maximum)
        })
        .collect();
    let baseline = if align == Align::Baseline && !items.is_empty() {
        Some(
            items
                .iter()
                .zip(&sizes)
                .map(|(item, size)| item.baseline.unwrap_or(*size))
                .fold(0., f32::max),
        )
    } else {
        None
    };
    let mut before = 0_f32;
    let mut after = 0_f32;
    let placements = items
        .iter()
        .zip(sizes)
        .map(|(item, size)| {
            let offset = match align {
                Align::Stretch | Align::Start => 0.,
                Align::Center => (available - size) / 2.,
                Align::End => available - size,
                Align::Baseline => baseline.unwrap_or(0.) - item.baseline.unwrap_or(size),
            };
            before = before.max(-offset);
            after = after.max(offset + size - available);
            Placement { offset, size }
        })
        .collect();
    Ok(CrossAxis {
        items: placements,
        baseline,
        overflow_before: before,
        overflow_after: after,
    })
}
#[cfg(test)]
#[path = "surface_cross_axis_tests.rs"]
mod tests;
