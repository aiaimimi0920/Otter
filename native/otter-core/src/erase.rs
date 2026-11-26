//! Premultiplied-BGRA content-eraser kernel with round caps/joins; original images are read-only, and cancellation returns no partial result.
use crate::{
    pixels::{self, Pixels},
    viewport::{checked_image_bytes, Point},
};
#[path = "erase_frame.rs"]
pub mod frame;
#[path = "erase_mask.rs"]
mod mask;
#[path = "erase_span.rs"]
pub mod span;
pub use frame::{apply_history_in_frame, apply_in_frame, apply_in_projection};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EraseError {
    InvalidInput,
    BudgetExceeded,
    Cancelled,
}

fn reserve_buffer<T>(count: usize) -> Result<(Vec<T>, crate::byte_budget::Lease), EraseError> {
    let bytes = count
        .checked_mul(std::mem::size_of::<T>())
        .ok_or(EraseError::BudgetExceeded)?;
    let mut lease = pixels::reserve(bytes).ok_or(EraseError::BudgetExceeded)?;
    let mut data = Vec::new();
    data.try_reserve_exact(count)
        .map_err(|_| EraseError::BudgetExceeded)?;
    if !lease.grow_to(data.capacity() * std::mem::size_of::<T>()) {
        return Err(EraseError::BudgetExceeded);
    }
    Ok((data, lease))
}

fn valid(point: Point) -> bool {
    point.x.is_finite()
        && point.y.is_finite()
        && point.x.abs() <= 1_000_000.0
        && point.y.abs() <= 1_000_000.0
}

/// Union 4x4 subpixel coverage; repeated points/self-intersections/repeated segments never accumulate soft-edge erasure.
/// UI dimensions match mainline, 4-96; callers convert logical coordinates to pixels.
pub fn apply(
    source: &Pixels,
    size: (u32, u32),
    points: &[Point],
    width: f32,
    cancelled: impl FnMut() -> bool,
) -> Result<Pixels, EraseError> {
    if !width.is_finite() || !(4.0..=96.0).contains(&width) {
        return Err(EraseError::InvalidInput);
    }
    apply_scaled(source, size, points, width, (1.0, 1.0), cancelled)
}

fn apply_scaled(
    source: &Pixels,
    size: (u32, u32),
    points: &[Point],
    width: f32,
    scale: (f64, f64),
    mut cancelled: impl FnMut() -> bool,
) -> Result<Pixels, EraseError> {
    if checked_image_bytes(size.0, size.1) != Some(source.len())
        || points.is_empty()
        || points.len() > 65_536
        || !points.iter().copied().all(valid)
        || !width.is_finite()
        || width <= 0.0
        || width > 1_000_000.0
    {
        return Err(EraseError::InvalidInput);
    }
    if cancelled() {
        return Err(EraseError::Cancelled);
    }
    let logical_radius = width as f64 / 2.0;
    // Circular bounds of elliptical capsules are only conservative scan bounds; actual coverage uses logical coordinates.
    let radius = logical_radius * scale.0.max(scale.1);
    let region = span::region(points, radius, size);
    if cancelled() {
        return Err(EraseError::Cancelled);
    }
    let Some([region_left, region_top, region_right, region_bottom]) = region else {
        return Ok(source.clone());
    };
    // If the entire affected region is transparent, allocate no mask/whole-image candidate; cancellation remains per-row.
    let mut has_content = false;
    for y in region_top..region_bottom {
        if cancelled() {
            return Err(EraseError::Cancelled);
        }
        let start = (y as usize * size.0 as usize + region_left as usize) * 4;
        let end = (y as usize * size.0 as usize + region_right as usize) * 4;
        if source[start..end].iter().any(|byte| *byte != 0) {
            has_content = true;
            break;
        }
    }
    if !has_content {
        return Ok(source.clone());
    }
    // Small regions retain compact dense storage; large regions avoid allocating untouched tiles.
    let mut coverage = mask::Mask::new(
        [region_left, region_top, region_right, region_bottom],
        source.len(),
    )?;
    // 4x4 subsamples offset from pixel centers by at most (+/-0.375/sx, +/-0.375/sy); center-distance thresholds can skip/fill whole pixels.
    let margin = 0.38 * (1.0 / scale.0).hypot(1.0 / scale.1);
    let inner_sq = (logical_radius > margin).then(|| (logical_radius - margin).powi(2));
    let outer_sq = (logical_radius + margin).powi(2);
    let radius_sq = logical_radius * logical_radius;
    for i in 0..points.len() {
        if cancelled() {
            return Err(EraseError::Cancelled);
        }
        let a = points[i.saturating_sub(1)];
        let b = points[i];
        let top = ((a.y.min(b.y) as f64 - radius).floor().max(0.0) as u32).min(size.1);
        let bottom = ((a.y.max(b.y) as f64 + radius).ceil().max(0.0) as u32).min(size.1);
        let dx = (b.x as f64 - a.x as f64) / scale.0;
        let dy = (b.y as f64 - a.y as f64) / scale.1;
        let length = dx * dx + dy * dy;
        for y in top..bottom {
            if cancelled() {
                return Err(EraseError::Cancelled);
            }
            let Some((left, right)) = span::row(a, b, radius, y, size.0) else {
                continue;
            };
            let cy = (y as f64 + 0.5 - a.y as f64) / scale.1;
            for x in left.max(region_left)..right.min(region_right) {
                let existing = coverage.get(x, y);
                if existing == u16::MAX {
                    continue;
                }
                let cx = (x as f64 + 0.5 - a.x as f64) / scale.0;
                let ct = if length == 0.0 {
                    0.0
                } else {
                    ((cx * dx + cy * dy) / length).clamp(0.0, 1.0)
                };
                let center_sq = (cx - ct * dx).powi(2) + (cy - ct * dy).powi(2);
                if center_sq > outer_sq {
                    continue;
                }
                let mut bitmask = if inner_sq.is_some_and(|limit| center_sq <= limit) {
                    u16::MAX
                } else {
                    existing
                };
                if bitmask != u16::MAX {
                    for sy in 0..4 {
                        let py = (y as f64 + (sy as f64 + 0.5) / 4.0 - a.y as f64) / scale.1;
                        for sx in 0..4 {
                            let bit = 1u16 << (sy * 4 + sx);
                            if bitmask & bit != 0 {
                                continue;
                            }
                            let px = (x as f64 + (sx as f64 + 0.5) / 4.0 - a.x as f64) / scale.0;
                            let t = if length == 0.0 {
                                0.0
                            } else {
                                ((px * dx + py * dy) / length).clamp(0.0, 1.0)
                            };
                            if (px - t * dx).powi(2) + (py - t * dy).powi(2) <= radius_sq {
                                bitmask |= bit;
                            }
                        }
                    }
                }
                if bitmask != existing {
                    coverage.set(x, y, bitmask)?;
                }
            }
        }
    }
    if !coverage.touched() {
        return if cancelled() {
            Err(EraseError::Cancelled)
        } else {
            Ok(source.clone())
        };
    }
    // Merge all stroke-segment coverage before one pixel blend; copy base images only when the first pixel actually changes.
    let mut output: Option<(Vec<u8>, crate::byte_budget::Lease)> = None;
    for y in region_top..region_bottom {
        if cancelled() {
            return Err(EraseError::Cancelled);
        }
        let Some((row_start_x, row_end_x)) = coverage.row_span(y) else {
            continue;
        };
        let row_base = y as usize * size.0 as usize;
        for x in row_start_x..row_end_x {
            let cell = coverage.get(x, y);
            if cell == 0 {
                continue;
            }
            let start = (row_base + x as usize) * 4;
            let src = &source[start..start + 4];
            let pixel = if cell == u16::MAX {
                [0u8; 4]
            } else {
                let keep = 16 - cell.count_ones();
                [
                    ((src[0] as u32 * keep + 8) / 16) as u8,
                    ((src[1] as u32 * keep + 8) / 16) as u8,
                    ((src[2] as u32 * keep + 8) / 16) as u8,
                    ((src[3] as u32 * keep + 8) / 16) as u8,
                ]
            };
            if pixel == src {
                continue;
            }
            if output.is_none() {
                if cancelled() {
                    return Err(EraseError::Cancelled);
                }
                let (mut bytes, lease) = reserve_buffer::<u8>(source.len())?;
                bytes.extend_from_slice(source);
                output = Some((bytes, lease));
            }
            output.as_mut().unwrap().0[start..start + 4].copy_from_slice(&pixel);
        }
    }
    if cancelled() {
        return Err(EraseError::Cancelled);
    }
    Ok(match output {
        Some((bytes, lease)) => Pixels::from_reserved(bytes, lease),
        None => source.clone(),
    })
}

#[cfg(test)]
#[path = "erase_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "erase_reference_tests.rs"]
mod reference_tests;
