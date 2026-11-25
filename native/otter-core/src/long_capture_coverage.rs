//! 最大完整覆盖矩形：用帧边界压缩坐标，在不读取像素的情况下确定裁切尺寸。
use super::*;

type Rect = (u32, u32, u32, u32);

pub(super) fn largest(
    frames: &[SpatialFrame],
    left: i64,
    top: i64,
    cancel: &AtomicBool,
) -> Result<Rect, Error> {
    check(cancel)?;
    // bounds 已验证坐标跨度。只有穿过帧边缘才会改变覆盖；最多 2 * MAX_FRAMES 个横/纵坐标。
    let mut columns = Vec::with_capacity(frames.len() * 2);
    let mut events = Vec::with_capacity(frames.len() * 2);
    for item in frames {
        let x = (item.x - left) as u32;
        let y = (item.y - top) as u32;
        let right = x + item.frame.width;
        columns.extend([x, right]);
        events.push((y, x, right, 1));
        events.push((y + item.frame.height, x, right, -1));
    }
    columns.sort_unstable();
    columns.dedup();
    events.sort_unstable_by_key(|event| event.0);
    let mut changes = vec![0i32; columns.len()];
    let mut heights = vec![0u32; columns.len().saturating_sub(1)];
    let mut stack = Vec::with_capacity(heights.len());
    let mut best = (0, 0, 0, 0);
    let mut i = 0;
    while let Some(&(y, ..)) = events.get(i) {
        check(cancel)?;
        while let Some(&(_, x, right, delta)) = events.get(i).filter(|event| event.0 == y) {
            let start = columns
                .binary_search(&x)
                .expect("frame starts at a column boundary");
            let end = columns
                .binary_search(&right)
                .expect("frame ends at a column boundary");
            changes[start] += delta;
            changes[end] -= delta;
            i += 1;
        }
        let Some(&(bottom, ..)) = events.get(i) else {
            break;
        };
        let mut active = 0;
        for (height, change) in heights.iter_mut().zip(&changes) {
            active += change;
            *height = if active > 0 { *height + bottom - y } else { 0 };
        }
        // The histogram uses real strip heights and widths, not counts of compressed cells.
        scan_row(&columns, &heights, bottom, &mut stack, &mut best);
    }
    check(cancel)?;
    if best.2 == 0 {
        Err(Error::Empty)
    } else {
        Ok(best)
    }
}

fn scan_row(
    columns: &[u32],
    heights: &[u32],
    bottom: u32,
    stack: &mut Vec<(usize, u32)>,
    best: &mut Rect,
) {
    stack.clear();
    let mut area = u64::from(best.2) * u64::from(best.3);
    for (x, h) in heights.iter().copied().chain([0]).enumerate() {
        let mut start = x;
        while let Some(&(left, previous)) = stack.last() {
            if previous <= h {
                break;
            }
            stack.pop();
            let candidate = (
                columns[left],
                bottom - previous,
                columns[x] - columns[left],
                previous,
            );
            let candidate_area = u64::from(candidate.2) * u64::from(candidate.3);
            // 同面积优先更靠上，其次更靠左；同起点时保留先到达的最小底边。
            if candidate_area > area
                || (candidate_area == area && (candidate.1, candidate.0) < (best.1, best.0))
            {
                *best = candidate;
                area = candidate_area;
            }
            start = left;
        }
        if h > 0 && stack.last().is_none_or(|&(_, previous)| previous != h) {
            stack.push((start, h));
        }
    }
}
