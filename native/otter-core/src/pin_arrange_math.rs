//! Plan one monitor in physical pixels. The caller groups by monitor before
//! asking for a plan and commits nothing until every monitor has passed.
use crate::pin_geometry::PinRect;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Left,
    CenterX,
    Right,
    Top,
    CenterY,
    Bottom,
    CompactX,
    CompactY,
    DistributeX,
    DistributeY,
    Grid,
}
impl Layout {
    pub fn min_count(self) -> usize {
        if matches!(self, Self::DistributeX | Self::DistributeY) {
            3
        } else {
            2
        }
    }
    pub fn uses_gap(self) -> bool {
        matches!(self, Self::CompactX | Self::CompactY | Self::Grid)
    }
}
pub fn plan(
    layout: Layout,
    frames: &[PinRect],
    anchor: usize,
    work: PinRect,
    dpi: u32,
) -> Option<Vec<PinRect>> {
    plan_with_gap(layout, frames, anchor, work, dpi, 12)
}
pub fn plan_with_gap(
    layout: Layout,
    frames: &[PinRect],
    anchor: usize,
    work: PinRect,
    dpi: u32,
    gap_dip: u16,
) -> Option<Vec<PinRect>> {
    if frames.is_empty()
        || frames.len() > 4096
        || dpi == 0
        || gap_dip > 512
        || frames.iter().any(|r| {
            r.width == 0 || r.height == 0 || r.width > work.width || r.height > work.height
        })
    {
        return None;
    }
    let anchor = *frames.get(anchor)?;
    let mut out = frames.to_vec();
    let mut order: Vec<usize> = (0..out.len()).collect();
    let gap = (i64::from(dpi) * i64::from(gap_dip) + 48) / 96;
    if matches!(layout, Layout::Grid) {
        order.sort_by_key(|i| (frames[*i].y, frames[*i].x, *i));
        let min_x = frames.iter().map(|r| i64::from(r.x)).min()?;
        let min_y = frames.iter().map(|r| i64::from(r.y)).min()?;
        let cols = (out.len() as f64).sqrt().ceil().max(1.0) as usize;
        let mut rel = vec![(0i64, 0i64); out.len()];
        let (mut x, mut y, mut row_h, mut col) = (0i64, 0i64, 0i64, 0usize);
        let (mut grid_w, mut grid_h) = (0i64, 0i64);
        for i in order {
            let w = i64::from(out[i].width);
            let h = i64::from(out[i].height);
            if col > 0 && (col >= cols || x + w > i64::from(work.width)) {
                x = 0;
                y += row_h + gap;
                row_h = 0;
                col = 0;
            }
            rel[i] = (x, y);
            grid_w = grid_w.max(x + w);
            grid_h = grid_h.max(y + h);
            x += w + gap;
            row_h = row_h.max(h);
            col += 1;
        }
        if grid_w > i64::from(work.width) || grid_h > i64::from(work.height) {
            return None;
        }
        let start_x = min_x.clamp(
            i64::from(work.x),
            i64::from(work.x) + i64::from(work.width) - grid_w,
        );
        let start_y = min_y.clamp(
            i64::from(work.y),
            i64::from(work.y) + i64::from(work.height) - grid_h,
        );
        for (i, r) in out.iter_mut().enumerate() {
            r.x = i32::try_from(start_x + rel[i].0).ok()?;
            r.y = i32::try_from(start_y + rel[i].1).ok()?;
        }
    } else {
        let horiz = matches!(
            layout,
            Layout::Left | Layout::CenterX | Layout::Right | Layout::CompactX | Layout::DistributeX
        );
        let pos = |r: PinRect| i64::from(if horiz { r.x } else { r.y });
        let size = |r: PinRect| i64::from(if horiz { r.width } else { r.height });
        order.sort_by_key(|&i| (pos(frames[i]), i));
        let first = pos(frames[order[0]]);
        let last = frames[*order.last()?];
        let total: i64 = out.iter().map(|&r| size(r)).sum();
        let spare = pos(last) + size(last) - first - total;
        let distribute = matches!(layout, Layout::DistributeX | Layout::DistributeY);
        if distribute && spare < 0 {
            return None;
        }
        let count = out.len();
        let compact_size = total + gap * (count - 1) as i64;
        let mut run = if matches!(layout, Layout::CompactX | Layout::CompactY) {
            if compact_size > size(work) {
                return None;
            }
            first.clamp(pos(work), pos(work) + size(work) - compact_size)
        } else {
            first
        };
        for (n, i) in order.into_iter().enumerate() {
            let r = &mut out[i];
            let to = match layout {
                Layout::Left | Layout::Top => pos(anchor),
                Layout::CenterX | Layout::CenterY => pos(anchor) + (size(anchor) - size(*r)) / 2,
                Layout::Right | Layout::Bottom => pos(anchor) + size(anchor) - size(*r),
                Layout::CompactX | Layout::CompactY => {
                    let next = run;
                    run += size(*r) + gap;
                    next
                }
                Layout::DistributeX | Layout::DistributeY => {
                    let next = run + spare * n as i64 / (count.saturating_sub(1).max(1) as i64);
                    run += size(*r);
                    next
                }
                Layout::Grid => unreachable!(),
            };
            let to = i32::try_from(to).ok()?;
            if horiz {
                r.x = to;
            } else {
                r.y = to;
            }
        }
    }
    if matches!(
        layout,
        Layout::Left
            | Layout::Top
            | Layout::CenterX
            | Layout::CenterY
            | Layout::Right
            | Layout::Bottom
    ) {
        for r in &mut out {
            let max_x = i64::from(work.x) + i64::from(work.width) - i64::from(r.width);
            let max_y = i64::from(work.y) + i64::from(work.height) - i64::from(r.height);
            r.x = i32::try_from(i64::from(r.x).clamp(i64::from(work.x), max_x)).ok()?;
            r.y = i32::try_from(i64::from(r.y).clamp(i64::from(work.y), max_y)).ok()?;
        }
    }
    let fits = out.iter().all(|r| {
        r.x >= work.x
            && r.y >= work.y
            && i64::from(r.x) + i64::from(r.width) <= i64::from(work.x) + i64::from(work.width)
            && i64::from(r.y) + i64::from(r.height) <= i64::from(work.y) + i64::from(work.height)
    });
    fits.then_some(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn r(x: i32, w: u32) -> PinRect {
        PinRect {
            x,
            y: 0,
            width: w,
            height: 30,
        }
    }
    #[test]
    fn mixed_widths_fixed_ends_spacing_and_dpi() {
        let work = PinRect {
            height: 100,
            ..r(-1800, 600)
        };
        let input = [r(-1760, 40), r(-1500, 60), r(-1350, 50)];
        let dist = plan(Layout::DistributeX, &input, 2, work, 144).unwrap();
        assert_eq!((dist[0].x, dist[1].x, dist[2].x), (-1760, -1565, -1350));
        let compact = plan(Layout::CompactX, &input, 1, work, 144).unwrap();
        assert_eq!(compact[1].x - compact[0].x, 58);
        assert_eq!(
            plan(Layout::Left, &input, 1, work, 144).unwrap()[0].x,
            -1500
        );
        assert_eq!(
            plan(Layout::CenterX, &input, 1, work, 144).unwrap()[0].x,
            -1490
        );
        assert_eq!(
            plan(Layout::Right, &input, 1, work, 144).unwrap()[0].x,
            -1480
        );
        let spaced = plan_with_gap(Layout::CompactX, &input, 1, work, 96, 24).unwrap();
        assert_eq!(spaced[1].x - spaced[0].x, 40 + 24);
        assert!(plan(Layout::Grid, &input, 0, r(-1800, 55), 96).is_none());
        assert!(plan(
            Layout::DistributeX,
            &[r(0, 50), r(20, 40), r(30, 40)],
            0,
            r(0, 200),
            96
        )
        .is_none());
    }

    #[test]
    fn grid_anchors_at_target_bounding_box_top_left_instead_of_monitor_origin() {
        let work = PinRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let pins = [
            PinRect {
                x: 420,
                y: 310,
                width: 100,
                height: 80,
            },
            PinRect {
                x: 680,
                y: 260,
                width: 100,
                height: 80,
            },
            PinRect {
                x: 350,
                y: 500,
                width: 100,
                height: 80,
            },
            PinRect {
                x: 720,
                y: 540,
                width: 100,
                height: 80,
            },
        ];
        // Bounding-box top-left is (min_x=350, min_y=260); 4 pins form a 2x2 grid with 16px gap.
        let out = plan_with_gap(Layout::Grid, &pins, 0, work, 96, 16).unwrap();
        // Sorted by (y, x): pins[1] (y=260), pins[0] (y=310), pins[2] (y=500), pins[3] (y=540).
        assert_eq!((out[1].x, out[1].y), (350, 260));
        assert_eq!((out[0].x, out[0].y), (350 + 100 + 16, 260));
        assert_eq!((out[2].x, out[2].y), (350, 260 + 80 + 16));
        assert_eq!((out[3].x, out[3].y), (350 + 100 + 16, 260 + 80 + 16));
    }
}
