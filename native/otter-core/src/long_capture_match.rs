//! Exact-overlap fast path: line signatures/KMP keep scans linear; final comparison uses original pixels, never trusting hash collisions.
use super::{Axis, Error, Frame};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Match {
    Duplicate,
    /// Current-frame axial-origin offset relative to previous frame; negative means upward/leftward extension.
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
fn length(frame: &Frame, axis: Axis) -> usize {
    match axis {
        Axis::Vertical => frame.height as usize,
        Axis::Horizontal => frame.width as usize,
    }
}
fn signatures<'a>(frame: &'a Frame, axis: Axis, cancel: &AtomicBool) -> Result<&'a [u64], Error> {
    if let Some(cached) = frame.signature_slot(axis).get() {
        check(cancel)?;
        return Ok(cached);
    }
    // Performance audit optimization:
    // Replaced byte-by-byte FNV-1a (which had a 3-4 cycle loop-carried dependency on every single byte and
    // could not auto-vectorize) with 64-bit word mixing on Vertical rows (8x fewer multiplies) and independent
    // 32-bit pixel lane mixing on Horizontal columns (4x fewer multiplies, auto-vectorized across columns).
    // Correctness is unchanged because `equal_region` performs an authoritative byte-exact slice comparison
    // on every candidate offset returned by `overlaps`.
    const PRIME: u64 = 0x9e3779b97f4a7c15;
    let mut result = vec![0xcbf29ce484222325u64; length(frame, axis)];
    let stride = frame.width as usize * 4;
    for (y, row) in frame.pixels.chunks_exact(stride).enumerate() {
        check(cancel)?;
        match axis {
            Axis::Vertical => {
                let mut hash = result[y];
                let chunks = row.chunks_exact(8);
                let tail = chunks.remainder();
                for word in chunks {
                    let v = u64::from_le_bytes(word.try_into().unwrap());
                    hash = (hash ^ v).wrapping_mul(PRIME);
                    hash ^= hash >> 32;
                }
                if !tail.is_empty() {
                    let v = u32::from_le_bytes(tail.try_into().unwrap()) as u64;
                    hash = (hash ^ v).wrapping_mul(PRIME);
                    hash ^= hash >> 32;
                }
                result[y] = hash;
            }
            Axis::Horizontal => {
                for (hash, pixel) in result.iter_mut().zip(row.chunks_exact(4)) {
                    let px = u32::from_le_bytes([pixel[0], pixel[1], pixel[2], pixel[3]]) as u64;
                    let mixed = (*hash ^ px).wrapping_mul(PRIME);
                    *hash = mixed ^ (mixed >> 29);
                }
            }
        }
    }
    let _ = frame.signature_slot(axis).set(result.into_boxed_slice());
    Ok(frame.signature_slot(axis).get().unwrap())
}
// Return suffix/prefix overlaps meeting minimums; two suffice to prove ambiguity, avoiding quadratic candidate growth.
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
    let stride = a.width as usize * 4;
    let (offset_a, offset_b, rows, bytes) = match axis {
        Axis::Vertical => (start_a * stride, start_b * stride, length, stride),
        Axis::Horizontal => (start_a * 4, start_b * 4, a.height as usize, length * 4),
    };
    // BGRA is row-contiguous; whole-span slice comparisons process bulk bytes without per-pixel indexing/cross-row reads.
    for row in 0..rows {
        check(cancel)?;
        let a_start = offset_a + row * stride;
        let b_start = offset_b + row * stride;
        if a.pixels[a_start..a_start + bytes] != b.pixels[b_start..b_start + bytes] {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Exact translation detection for equal-sized captures. Never guess low-texture/periodic content; approximate matches/fixed bars need separate analysis.
/// No persistent signature cache; frame-axis limits bound each allocation below 1 MiB, without pixel copies.
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
    let length = length(previous, axis);
    if minimum_overlap == 0 || minimum_overlap as usize > length {
        return Err(Error::Dimensions);
    }
    if equal_region(previous, current, axis, 0, 0, length, cancel)? {
        return Ok(Match::Duplicate);
    }
    let a = signatures(previous, axis, cancel)?;
    let b = signatures(current, axis, cancel)?;
    let forward = overlaps(a, b, minimum_overlap as usize);
    let backward = overlaps(b, a, minimum_overlap as usize);
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
