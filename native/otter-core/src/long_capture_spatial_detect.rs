//! Unlocked-axis viewport movement commits physical coordinates only; the first reliable residual page motion determines scrolling axis.
use super::*;

pub(super) fn match_frame(
    previous: &Frame,
    current: &Frame,
    movement: (i64, i64),
    cancel: &AtomicBool,
) -> Result<(SpatialMatch, Option<Axis>), Error> {
    let candidate = |axis| {
        match_spatial(
            previous,
            current,
            movement,
            axis,
            MatchOptions::for_detection(axis, previous),
            cancel,
        )
    };
    let vertical = candidate(Axis::Vertical)?;
    let horizontal = candidate(Axis::Horizontal)?;
    let chosen = match (vertical, horizontal) {
        (SpatialMatch::Offset { x, y, .. }, SpatialMatch::Offset { x: hx, y: hy, .. })
            if (x, y) == (hx, hy) && (x, y) == movement =>
        {
            return Ok((vertical, None))
        }
        (SpatialMatch::Offset { .. }, SpatialMatch::Rejected(_)) => (vertical, Axis::Vertical),
        (SpatialMatch::Rejected(_), SpatialMatch::Offset { .. }) => (horizontal, Axis::Horizontal),
        _ => return Ok((SpatialMatch::Rejected(ApproximateMatch::Ambiguous), None)),
    };
    let detected = match chosen.0 {
        SpatialMatch::Offset { x, y, .. } if (x, y) != movement => Some(chosen.1),
        _ => None,
    };
    Ok((chosen.0, detected))
}
