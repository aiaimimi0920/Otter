//! Row/column cross-axis geometry; baselines come from the measurement owner, with bottom-edge synthesis for boxes without text baselines.
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
fn valid_item(item: &Item) -> bool {
    valid(item.intrinsic)
        && valid(item.minimum)
        && valid(item.maximum)
        && item.minimum <= item.maximum
        && item.explicit.is_none_or(valid)
        && item.baseline.is_none_or(valid)
}
fn size(item: &Item, available: f32, align: Align) -> f32 {
    item.explicit
        .unwrap_or(if align == Align::Stretch {
            available
        } else {
            item.intrinsic
        })
        .clamp(item.minimum, item.maximum)
}
fn place(item: &Item, size: f32, available: f32, align: Align, baseline: Option<f32>) -> Placement {
    let offset = match align {
        Align::Stretch | Align::Start => 0.,
        Align::Center => (available - size) / 2.,
        Align::End => available - size,
        Align::Baseline => baseline.unwrap_or(0.) - item.baseline.unwrap_or(size),
    };
    Placement { offset, size }
}
/// Single item shares the same geometry without constructing group buffers.
pub(crate) fn arrange_one(item: Item, available: f32, align: Align) -> Result<Placement, Error> {
    if !valid(available) || !valid_item(&item) {
        return Err(Error::Input);
    }
    let size = size(&item, available, align);
    let baseline = (align == Align::Baseline).then(|| 0_f32.max(item.baseline.unwrap_or(size)));
    Ok(place(&item, size, available, align, baseline))
}
pub fn arrange(items: &[Item], available: f32, align: Align) -> Result<CrossAxis, Error> {
    if items.len() > crate::surface_scene::MAX_NODES {
        return Err(Error::Limit);
    }
    if !valid(available) || !items.iter().all(valid_item) {
        return Err(Error::Input);
    }
    let sizes: Vec<f32> = items
        .iter()
        .map(|item| size(item, available, align))
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
            let p = place(item, size, available, align, baseline);
            before = before.max(-p.offset);
            after = after.max(p.offset + p.size - available);
            p
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
