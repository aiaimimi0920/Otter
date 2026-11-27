//! Physical screen layout; DIP margins scale with the monitor under the pointer.
pub type Bounds = [i32; 4];
pub fn scale(dip: i32, dpi: u32) -> i32 {
    (dip * dpi.clamp(48, 768) as i32 + 48) / 96
}
fn overlaps([l, t, r, b]: Bounds, o: Bounds) -> bool {
    l < o[2] && r > o[0] && t < o[3] && b > o[1]
}
pub fn place(screen: Bounds, [px, py]: [i32; 2], selection: Option<Bounds>, dpi: u32) -> Bounds {
    let [l, t, r, b] = screen;
    let w = scale(168, dpi).min(r.saturating_sub(l)).max(0);
    let h = scale(204, dpi).min(b.saturating_sub(t)).max(0);
    let gap = scale(24, dpi);
    let safe = scale(36, dpi);
    let rect = |x: i32, y: i32| {
        let x = x.clamp(l, r.saturating_sub(w).max(l));
        let y = y.clamp(t, b.saturating_sub(h).max(t));
        [x, y, x.saturating_add(w), y.saturating_add(h)]
    };
    let left = px.saturating_sub(gap).saturating_sub(w);
    let top = py.saturating_sub(gap).saturating_sub(h);
    let right = px.saturating_add(gap);
    let bottom = py.saturating_add(gap);
    let Some(s) = selection else {
        return rect(
            if right.saturating_add(w) <= r {
                right
            } else {
                left
            },
            if bottom.saturating_add(h) <= b {
                bottom
            } else {
                top
            },
        );
    };
    let pos_x = i64::from(px) * 2 >= i64::from(s[0]) + i64::from(s[2]);
    let pos_y = i64::from(py) * 2 >= i64::from(s[1]) + i64::from(s[3]);
    let badge = [
        s[0],
        s[1].saturating_sub(scale(32, dpi)),
        s[0].saturating_add(scale(220, dpi)),
        s[1].saturating_add(scale(28, dpi)),
    ];
    let positions = [
        rect(
            if pos_x { right } else { left },
            if pos_y { bottom } else { top },
        ),
        rect(s[2].saturating_add(gap), top),
        rect(s[0].saturating_sub(gap).saturating_sub(w), bottom),
        rect(left, s[1].saturating_sub(gap).saturating_sub(h)),
        rect(right, s[3].saturating_add(gap)),
    ];
    let clear = |v: Bounds, margin: i32| {
        i64::from(px) + i64::from(margin) <= i64::from(v[0])
            || i64::from(px) - i64::from(margin) >= i64::from(v[2])
            || i64::from(py) + i64::from(margin) <= i64::from(v[1])
            || i64::from(py) - i64::from(margin) >= i64::from(v[3])
    };
    if let Some(v) = positions
        .into_iter()
        .find(|v| clear(*v, gap) && !overlaps(*v, s) && !overlaps(*v, badge))
    {
        return v;
    }
    // There is no exterior space. Prefer the opposite quadrant, then another
    // distant corner if the size label occupies it. Never hide at a screen edge.
    let xs = if pos_x { [l, r - w] } else { [r - w, l] };
    let ys = if pos_y { [t, b - h] } else { [b - h, t] };
    let corners = [
        rect(xs[0], ys[0]),
        rect(xs[0], ys[1]),
        rect(xs[1], ys[0]),
        rect(xs[1], ys[1]),
    ];
    corners
        .into_iter()
        .find(|v| clear(*v, safe) && !overlaps(*v, badge))
        .unwrap_or(corners[0])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_four_edges_retain_clearance_and_size_on_mixed_dpi_negative_monitors() {
        let s = [-1920, -1080, 0, 0];
        for dpi in [96, 144, 192] {
            for [x, y] in [[-1919, -1079], [-1, -1], [-1919, -1], [-1, -1079]] {
                let [l, t, r, b] = place(s, [x, y], Some(s), dpi);
                assert!(l >= s[0] && r <= s[2] && t >= s[1] && b <= s[3]);
                assert_eq!([r - l, b - t], [scale(168, dpi), scale(204, dpi)]);
                assert!((l - x).max(x - r).max(t - y).max(y - b) >= scale(36, dpi));
            }
        }
    }
}
