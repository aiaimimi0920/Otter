//! Partition solid borders by the minimum distance-to-edge/border-width ratio; partitions cover the outer box, then one rounded ring mask removes the inner hole.
use crate::surface_box::Insets;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
pub(super) fn polygon(rect: D2D_RECT_F, width: Insets, side: usize) -> Vec<[f32; 2]> {
    let widths = [width.top, width.right, width.bottom, width.left];
    if widths[side] == 0. {
        return Vec::new();
    }
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;
    // Compute in local coordinates so multiplying large screen origins by border widths does not lose corner precision.
    let distances = [[0., 1., 0.], [-1., 0., w], [0., -1., h], [1., 0., 0.]];
    let mut points = vec![[0., 0.], [w, 0.], [w, h], [0., h]];
    for other in 0..4 {
        if other == side || widths[other] == 0. || points.is_empty() {
            continue;
        }
        let coefficient: [f32; 3] = std::array::from_fn(|a| {
            distances[side][a] * widths[other] - distances[other][a] * widths[side]
        });
        let distance = |p: [f32; 2]| coefficient[0] * p[0] + coefficient[1] * p[1] + coefficient[2];
        let mut output = Vec::with_capacity(8);
        let emit = |output: &mut Vec<[f32; 2]>, point: [f32; 2]| {
            if output.last() != Some(&point) {
                output.push(point);
            }
        };
        let mut previous = *points.last().unwrap();
        let mut before = distance(previous);
        for &current in &points {
            let after = distance(current);
            if (before <= 0.) != (after <= 0.) {
                let point = if before == 0. {
                    previous
                } else if after == 0. {
                    current
                } else {
                    let ratio = (before / (before - after)).clamp(0., 1.);
                    std::array::from_fn(|a| previous[a] + (current[a] - previous[a]) * ratio)
                };
                emit(&mut output, point);
            }
            if after <= 0. {
                emit(&mut output, current);
            }
            previous = current;
            before = after;
        }
        if output.len() > 1 && output.first() == output.last() {
            output.pop();
        }
        // A convex quadrilateral intersected with three half-planes has at most seven vertices; text/node counts do not expand one border's working set.
        debug_assert!(output.len() <= 7);
        points = output;
    }
    for point in &mut points {
        point[0] += rect.left;
        point[1] += rect.top;
    }
    points
}
