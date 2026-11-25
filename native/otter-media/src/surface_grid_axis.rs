//! Track allocation from measured content contributions; fixed minima do not collapse in narrow hosts, and fr distributes only remaining space.
use super::{
    placement::Area,
    tracks::{Bound, Track},
    Error,
};
use crate::{
    surface_flow::{self as flow, Justify},
    surface_length::{Context, Length, Unit},
};
#[derive(Clone, Copy)]
struct State {
    base: f32,
    limit: f32,
    fraction: f32,
    auto: bool,
    intrinsic: bool,
}
pub(super) struct Axis {
    pub sizes: Vec<f32>,
    pub offsets: Vec<f32>,
    pub extent: f32,
}
pub(super) struct Space {
    pub available: Option<f32>,
    pub gap: f32,
    pub context: Context,
    pub distribution: (Justify, bool),
}
impl Axis {
    pub(super) fn span(&self, start: usize, count: usize) -> f32 {
        let end = start + count - 1;
        self.offsets[end] + self.sizes[end] - self.offsets[start]
    }
}
fn fixed(bound: Bound, definite: Option<f32>, mut context: Context) -> Option<f32> {
    if let Bound::Length(length) = bound {
        if matches!(length, Length::Scalar(_, Unit::Percent)) && definite.is_none() {
            return None;
        }
        context.axis = definite.unwrap_or(context.axis);
        length.resolve(context)
    } else {
        None
    }
}
pub(super) fn distribution(value: Option<&serde_json::Value>) -> Result<(Justify, bool), Error> {
    if value.is_some_and(|v| !v.is_string()) {
        return Err(Error::Syntax);
    }
    Ok(match value.and_then(|v| v.as_str()) {
        None | Some("normal" | "stretch") => (Justify::Start, true),
        Some("start" | "flex-start") => (Justify::Start, false),
        Some("end" | "flex-end") => (Justify::End, false),
        Some("center") => (Justify::Center, false),
        Some("space-between") => (Justify::Between, false),
        Some("space-around") => (Justify::Around, false),
        Some("space-evenly") => (Justify::Evenly, false),
        _ => return Err(Error::Unsupported),
    })
}
fn increase_limited(states: &mut [State], mut active: Vec<usize>, mut amount: f32) {
    // Each round freezes at least one full track; iterations cannot exceed track count and are not per-pixel.
    while amount > 0.001 && !active.is_empty() {
        let share = amount / active.len() as f32;
        let mut used = 0.;
        for index in &active {
            let state = &mut states[*index];
            let add = share.min((state.limit - state.base).max(0.));
            state.base += add;
            used += add;
        }
        amount -= used;
        if used <= 0.001 {
            break;
        }
        active.retain(|index| states[*index].limit - states[*index].base > 0.001);
    }
}
fn increase_evenly(states: &mut [State], amount: f32, eligible: impl Fn(&State) -> bool) {
    if amount > 0.001 {
        let count = states.iter().filter(|s| eligible(s)).count();
        if count != 0 {
            let share = amount / count as f32;
            for state in states.iter_mut().filter(|s| eligible(s)) {
                state.base += share;
            }
        }
    }
}
pub(super) fn resolve(
    tracks: &[Track],
    areas: &[Area],
    contributions: &[f32],
    axis: usize,
    space: Space,
) -> Result<Axis, Error> {
    let Space {
        available,
        gap,
        context,
        distribution,
    } = space;
    if areas.len() != contributions.len()
        || axis > 1
        || !gap.is_finite()
        || gap < 0.
        || available.is_some_and(|v| !v.is_finite() || !(0. ..=32768.).contains(&v))
        || contributions
            .iter()
            .any(|v| !v.is_finite() || !(0. ..=32768.).contains(v))
    {
        return Err(Error::Input);
    }
    let mut work = 0_usize;
    for area in areas {
        let span = area.span[axis];
        if span == 0
            || area.start[axis]
                .checked_add(span)
                .is_none_or(|end| end > tracks.len())
        {
            return Err(Error::Input);
        }
        work += span;
        if work > super::tracks::MAX_CELLS * 16 {
            return Err(Error::Limit);
        }
    }
    if tracks.is_empty() {
        return if areas.is_empty() {
            Ok(Axis {
                sizes: Vec::new(),
                offsets: Vec::new(),
                extent: 0.,
            })
        } else {
            Err(Error::Input)
        };
    }
    // Percentage cycles on indefinite axes are not implemented yet; reject rather than treating them as auto.
    if available.is_none()
        && tracks.iter().any(|t| {
            [t.min, t.max]
                .iter()
                .any(|b| matches!(b, Bound::Length(Length::Scalar(_, Unit::Percent))))
        })
    {
        return Err(Error::Unsupported);
    }
    let mut states = tracks
        .iter()
        .map(|track| {
            let minimum = fixed(track.min, available, context);
            let maximum = fixed(track.max, available, context);
            State {
                base: minimum.unwrap_or(0.),
                limit: maximum.unwrap_or(0.).max(minimum.unwrap_or(0.)),
                fraction: if let Bound::Fraction(n) = track.max {
                    n
                } else {
                    0.
                },
                auto: matches!(track.max, Bound::Auto),
                intrinsic: minimum.is_none(),
            }
        })
        .collect::<Vec<_>>();
    let mut ordered = (0..areas.len()).collect::<Vec<_>>();
    ordered.sort_by_key(|i| areas[*i].span[axis]);
    for index in ordered {
        let area = areas[index];
        let start = area.start[axis];
        let end = start + area.span[axis];
        let span = &mut states[start..end];
        let gutters = gap * (area.span[axis] - 1) as f32;
        let occupied = span.iter().map(|s| s.base).sum::<f32>() + gutters;
        increase_evenly(span, (contributions[index] - occupied).max(0.), |s| {
            s.intrinsic
        });
        // auto maxima use actual content contributions; content cannot arbitrarily raise fixed minmax maxima.
        let auto = span.iter().filter(|s| s.auto).count();
        if auto != 0 {
            let existing = span.iter().map(|s| s.limit.max(s.base)).sum::<f32>() + gutters;
            let extra = (contributions[index] - existing).max(0.) / auto as f32;
            for state in span.iter_mut().filter(|s| s.auto) {
                state.limit = state.limit.max(state.base) + extra;
            }
        }
    }
    for state in &mut states {
        state.limit = state.limit.max(state.base);
    }
    let gutters = gap * tracks.len().saturating_sub(1) as f32;
    if let Some(available) = available {
        let free = (available - gutters - states.iter().map(|s| s.base).sum::<f32>()).max(0.);
        let finite = (0..states.len())
            .filter(|i| states[*i].fraction == 0. && states[*i].limit > states[*i].base)
            .collect::<Vec<_>>();
        increase_limited(&mut states, finite, free);
        let mut flex = states.iter().map(|s| s.fraction > 0.).collect::<Vec<_>>();
        for _ in 0..=states.len() {
            let nonflex = states
                .iter()
                .enumerate()
                .filter(|(i, _)| !flex[*i])
                .map(|(_, s)| s.base)
                .sum::<f32>();
            let weights = states
                .iter()
                .enumerate()
                .filter(|(i, _)| flex[*i])
                .map(|(_, s)| s.fraction)
                .sum::<f32>()
                .max(1.);
            let fraction = ((available - gutters - nonflex) / weights).max(0.);
            let mut frozen = false;
            for (i, state) in states.iter().enumerate() {
                if flex[i] && state.base > fraction * state.fraction + 0.001 {
                    flex[i] = false;
                    frozen = true;
                }
            }
            if !frozen {
                for (i, state) in states.iter_mut().enumerate() {
                    if flex[i] {
                        state.base = state.base.max(fraction * state.fraction);
                    }
                }
                break;
            }
        }
        if distribution.1 {
            let free = (available - gutters - states.iter().map(|s| s.base).sum::<f32>()).max(0.);
            increase_evenly(&mut states, free, |s| s.auto);
        }
    } else {
        for state in &mut states {
            state.base = state.base.max(state.limit);
        }
        let mut fraction = states
            .iter()
            .filter(|s| s.fraction > 0.)
            .map(|s| s.base / s.fraction)
            .fold(0_f32, f32::max);
        for (area, contribution) in areas.iter().zip(contributions) {
            let slice = &states[area.start[axis]..area.start[axis] + area.span[axis]];
            let weights = slice.iter().map(|s| s.fraction).sum::<f32>();
            if weights > 0. {
                let fixed = slice
                    .iter()
                    .filter(|s| s.fraction == 0.)
                    .map(|s| s.base)
                    .sum::<f32>();
                fraction = fraction.max(
                    (contribution - fixed - gap * (area.span[axis] - 1) as f32).max(0.) / weights,
                );
            }
        }
        for state in &mut states {
            state.base = state.base.max(state.fraction * fraction);
        }
    }
    let extent = states.iter().map(|s| s.base).sum::<f32>() + gutters;
    if !extent.is_finite()
        || extent > 16_777_216.
        || states
            .iter()
            .any(|s| !s.base.is_finite() || s.base > 32768.)
    {
        return Err(Error::Input);
    }
    // Reuse single-axis distribution without shrink, rather than duplicating drawing/hit-test geometry.
    let items = states
        .iter()
        .map(|s| flow::Item {
            basis: s.base,
            minimum: s.base,
            maximum: s.base,
            grow: 0.,
            shrink: 0.,
        })
        .collect::<Vec<_>>();
    let layout = flow::arrange(
        &items,
        available.unwrap_or(extent.min(32768.)),
        gap,
        distribution.0,
    )
    .map_err(|_| Error::Input)?;
    // flow's legacy safe alignment clamps negative free space; Grid center/end preserve overflow on both sides.
    let overflow_offset = match distribution.0 {
        Justify::Center => available.map_or(0., |v| (v - extent).min(0.) / 2.),
        Justify::End => available.map_or(0., |v| (v - extent).min(0.)),
        _ => 0.,
    };
    Ok(Axis {
        sizes: states.iter().map(|s| s.base).collect(),
        offsets: layout
            .items
            .iter()
            .map(|p| p.offset + overflow_offset)
            .collect(),
        extent,
    })
}
