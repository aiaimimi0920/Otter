//! Plan one monitor in physical pixels. The caller groups by monitor before
//! asking for a plan and commits nothing until every monitor has passed.
use crate::pin_geometry::PinRect;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Left,
    Top,
    CenterX,
    CenterY,
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
}
pub fn plan(
    layout: Layout,
    frames: &[PinRect],
    anchor: usize,
    work: PinRect,
    dpi: u32,
) -> Option<Vec<PinRect>> {
    if frames.is_empty()
        || frames.len() > 4096
        || dpi == 0
        || frames.iter().any(|r| {
            r.width == 0 || r.height == 0 || r.width > work.width || r.height > work.height
        })
    {
        return None;
    }
    let anchor = *frames.get(anchor)?;
    let mut out = frames.to_vec();
    let mut order: Vec<usize> = (0..out.len()).collect();
    let gap = (i64::from(dpi) * 12 + 48) / 96;
    if matches!(layout, Layout::Grid) {
        order.sort_by_key(|i| (frames[*i].y, frames[*i].x, *i));
        let (mut x, mut y, mut row) = (0i64, 0i64, 0i64);
        for i in order {
            let r = &mut out[i];
            if x > 0 && x + i64::from(r.width) > i64::from(work.width) {
                x = 0;
                y += row + gap;
                row = 0;
            }
            r.x = i32::try_from(i64::from(work.x) + x).ok()?;
            r.y = i32::try_from(i64::from(work.y) + y).ok()?;
            x += i64::from(r.width) + gap;
            row = row.max(i64::from(r.height));
        }
    } else {
        let horiz = matches!(
            layout,
            Layout::Left | Layout::CenterX | Layout::CompactX | Layout::DistributeX
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
}
