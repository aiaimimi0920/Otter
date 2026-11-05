//! 精确重叠快速路径：线签名与 KMP 保持线性扫描；最终比较原像素，不信任哈希碰撞。
use super::{Axis, Error, Frame};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Match {
    Duplicate,
    /// 当前帧相对前帧的轴向起点偏移，负值表示向上/左扩展。
    Offset(i64),
    Ambiguous,
    NoOverlap,
}

fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
fn size(frame: &Frame, axis: Axis) -> (usize, usize) {
    match axis {
        Axis::Vertical => (frame.height as usize, frame.width as usize),
        Axis::Horizontal => (frame.width as usize, frame.height as usize),
    }
}
fn pixel(frame: &Frame, axis: Axis, line: usize, cross: usize) -> &[u8] {
    let index = match axis {
        Axis::Vertical => line * frame.width as usize + cross,
        Axis::Horizontal => cross * frame.width as usize + line,
    } * 4;
    &frame.pixels[index..index + 4]
}
fn signatures(frame: &Frame, axis: Axis, cancel: &AtomicBool) -> Result<Vec<u64>, Error> {
    let (lines, cross) = size(frame, axis);
    let mut result = Vec::with_capacity(lines);
    for line in 0..lines {
        check(cancel)?;
        let mut hash = 0xcbf29ce484222325u64;
        for x in 0..cross {
            for byte in pixel(frame, axis, line, x) {
                hash = (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
            }
        }
        result.push(hash);
    }
    Ok(result)
}
// 返回所有满足下限的后缀/前缀重叠；只保留两个即可证明歧义，避免候选平方增长。
fn overlaps(text: &[u64], pattern: &[u64], minimum: usize) -> Vec<usize> {
    let mut prefix = vec![0; pattern.len()];
    for index in 1..pattern.len() {
        let mut matched = prefix[index - 1];
        while matched > 0 && pattern[index] != pattern[matched] {
            matched = prefix[matched - 1];
        }
        if pattern[index] == pattern[matched] {
            matched += 1;
        }
        prefix[index] = matched;
    }
    let mut matched = 0;
    for value in text {
        while matched > 0 && (matched == pattern.len() || *value != pattern[matched]) {
            matched = prefix[matched - 1];
        }
        if *value == pattern[matched] {
            matched += 1;
        }
    }
    let mut result = Vec::with_capacity(2);
    while matched >= minimum && result.len() < 2 {
        result.push(matched);
        matched = prefix[matched - 1];
    }
    result
}
fn equal_region(
    a: &Frame,
    b: &Frame,
    axis: Axis,
    start_a: usize,
    start_b: usize,
    length: usize,
    cancel: &AtomicBool,
) -> Result<bool, Error> {
    for line in 0..length {
        check(cancel)?;
        for x in 0..size(a, axis).1 {
            if pixel(a, axis, start_a + line, x) != pixel(b, axis, start_b + line, x) {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// 等尺寸截图的精确平移识别。低纹理/周期内容不猜测；近似匹配与固定栏需独立分析。
/// 签名不持久缓存，每次内存上界由帧轴上限约束（小于 1 MiB），没有像素副本。
pub fn match_exact(
    previous: &Frame,
    current: &Frame,
    axis: Axis,
    minimum_overlap: u32,
    cancel: &AtomicBool,
) -> Result<Match, Error> {
    check(cancel)?;
    if previous.dimensions() != current.dimensions() {
        return Err(Error::Dimensions);
    }
    let length = size(previous, axis).0;
    if minimum_overlap == 0 || minimum_overlap as usize > length {
        return Err(Error::Dimensions);
    }
    if equal_region(previous, current, axis, 0, 0, length, cancel)? {
        return Ok(Match::Duplicate);
    }
    let a = signatures(previous, axis, cancel)?;
    let b = signatures(current, axis, cancel)?;
    let forward = overlaps(&a, &b, minimum_overlap as usize);
    let backward = overlaps(&b, &a, minimum_overlap as usize);
    check(cancel)?;
    if forward.len() + backward.len() > 1 {
        return Ok(Match::Ambiguous);
    }
    let (first, second, overlap, sign) = match (forward.first(), backward.first()) {
        (Some(&overlap), None) => (previous, current, overlap, 1),
        (None, Some(&overlap)) => (current, previous, overlap, -1),
        _ => return Ok(Match::NoOverlap),
    };
    let offset = length - overlap;
    if equal_region(first, second, axis, offset, 0, overlap, cancel)? {
        Ok(Match::Offset(offset as i64 * sign))
    } else {
        Ok(Match::NoOverlap)
    }
}

#[cfg(test)]
#[path = "long_capture_match_tests.rs"]
mod tests;
