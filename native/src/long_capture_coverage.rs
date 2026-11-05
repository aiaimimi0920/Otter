//! 最大完整覆盖矩形：逐行直方图，O(width * height) 时间、O(width) 辅助空间。
use super::*;
pub(super) fn largest(
    pixels: &[u8],
    width: u32,
    height: u32,
    cancel: &AtomicBool,
) -> Result<(u32, u32, u32, u32), Error> {
    let mut heights = Vec::new();
    heights
        .try_reserve_exact(width as usize)
        .map_err(|_| Error::Budget)?;
    heights.resize(width as usize, 0u32);
    let mut stack: Vec<(usize, u32)> = Vec::new();
    stack
        .try_reserve_exact(width as usize)
        .map_err(|_| Error::Budget)?;
    let mut best = (0, 0, 0, 0);
    let mut area = 0u64;
    for y in 0..height {
        check(cancel)?;
        for (x, column) in heights.iter_mut().enumerate() {
            *column = if pixels[(y as usize * width as usize + x) * 4 + 3] == 255 {
                *column + 1
            } else {
                0
            };
        }
        stack.clear();
        for x in 0..=width as usize {
            let h = heights.get(x).copied().unwrap_or(0);
            let mut start = x;
            while let Some(&(left, previous)) = stack.last() {
                if previous <= h {
                    break;
                }
                stack.pop();
                let candidate = (left as u32, y + 1 - previous, (x - left) as u32, previous);
                let candidate_area = u64::from(candidate.2) * u64::from(candidate.3);
                // 与主干一致：同面积优先更靠上，然后更靠左。
                if candidate_area > area
                    || (candidate_area == area && (candidate.1, candidate.0) < (best.1, best.0))
                {
                    best = candidate;
                    area = candidate_area;
                }
                start = left;
            }
            if h > 0 && stack.last().is_none_or(|&(_, previous)| previous != h) {
                stack.push((start, h));
            }
        }
    }
    check(cancel)?;
    if area == 0 {
        Err(Error::Empty)
    } else {
        Ok(best)
    }
}
