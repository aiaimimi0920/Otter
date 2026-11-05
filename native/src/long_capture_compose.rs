//! 先建立最多 1025 个共享帧片段（含固定首尾），再一次分配输出。
use super::*;
use std::{collections::VecDeque, sync::atomic::Ordering};
#[path = "long_capture_preview.rs"]
mod preview;
pub(super) use preview::render as preview;
struct Segment {
    frame: usize,
    offset: u32,
    length: u32,
}
struct Plan {
    segments: VecDeque<Segment>,
    origin: i64,
    end: i64,
    cross: u32,
    width: u32,
    height: u32,
    bytes: usize,
}
fn cancelled(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
fn axis_size(axis: Axis, frame: &Frame) -> (u32, u32) {
    match axis {
        Axis::Vertical => (frame.height, frame.width),
        Axis::Horizontal => (frame.width, frame.height),
    }
}
fn plan(
    axis: Axis,
    frames: &[Positioned],
    cancel: &AtomicBool,
    edges: (u32, u32),
) -> Result<Plan, Error> {
    cancelled(cancel)?;
    if frames.is_empty() {
        return Err(Error::Empty);
    }
    if frames.len() > MAX_FRAMES {
        return Err(Error::FrameLimit);
    }
    let (length, cross) = axis_size(axis, &frames[0].frame);
    let removed = edges.0.checked_add(edges.1).ok_or(Error::Dimensions)?;
    let length = length
        .checked_sub(removed)
        .filter(|n| *n > 0)
        .ok_or(Error::Dimensions)?;
    let mut origin = frames[0].origin;
    let mut end = origin
        .checked_add(length as i64)
        .ok_or(Error::OutputLimit)?;
    let mut segments = VecDeque::from([Segment {
        frame: 0,
        offset: edges.0,
        length,
    }]);
    let mut total = u64::from(frames[0].frame.width) * u64::from(frames[0].frame.height);
    for (index, input) in frames.iter().enumerate().skip(1) {
        cancelled(cancel)?;
        let (length, current_cross) = axis_size(axis, &input.frame);
        let length = length
            .checked_sub(removed)
            .filter(|n| *n > 0)
            .ok_or(Error::Dimensions)?;
        if current_cross != cross {
            return Err(Error::CrossAxis);
        }
        total = total
            .checked_add(u64::from(input.frame.width) * u64::from(input.frame.height))
            .ok_or(Error::InputLimit)?;
        if total > MAX_INPUT_PIXELS {
            return Err(Error::InputLimit);
        }
        let current_end = input
            .origin
            .checked_add(length as i64)
            .ok_or(Error::OutputLimit)?;
        if input.origin > end || current_end < origin {
            return Err(Error::Gap);
        }
        if input.origin < origin {
            segments.push_front(Segment {
                frame: index,
                offset: edges.0,
                length: u32::try_from(origin - input.origin).map_err(|_| Error::OutputLimit)?,
            });
            origin = input.origin;
        }
        if current_end > end {
            segments.push_back(Segment {
                frame: index,
                offset: edges.0
                    + u32::try_from(end - input.origin).map_err(|_| Error::OutputLimit)?,
                length: u32::try_from(current_end - end).map_err(|_| Error::OutputLimit)?,
            });
            end = current_end;
        }
        let axis_length = u32::try_from(end.checked_sub(origin).ok_or(Error::OutputLimit)?)
            .map_err(|_| Error::OutputLimit)?;
        let (width, height) = match axis {
            Axis::Vertical => (cross, axis_length),
            Axis::Horizontal => (axis_length, cross),
        };
        output_bytes(width, height)?;
    }
    if edges.0 > 0 {
        segments.push_front(Segment {
            frame: 0,
            offset: 0,
            length: edges.0,
        });
        origin = origin
            .checked_sub(i64::from(edges.0))
            .ok_or(Error::OutputLimit)?;
    }
    if edges.1 > 0 {
        segments.push_back(Segment {
            frame: 0,
            offset: axis_size(axis, &frames[0].frame).0 - edges.1,
            length: edges.1,
        });
        end = end
            .checked_add(i64::from(edges.1))
            .ok_or(Error::OutputLimit)?;
    }
    let axis_length = u32::try_from(end.checked_sub(origin).ok_or(Error::OutputLimit)?)
        .map_err(|_| Error::OutputLimit)?;
    let (width, height) = match axis {
        Axis::Vertical => (cross, axis_length),
        Axis::Horizontal => (axis_length, cross),
    };
    Ok(Plan {
        segments,
        origin,
        end,
        cross,
        width,
        height,
        bytes: output_bytes(width, height)?,
    })
}
pub(super) fn stitch(
    axis: Axis,
    frames: &[Positioned],
    cancel: &AtomicBool,
    reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
) -> Result<Composite, Error> {
    stitch_edges(axis, frames, cancel, reserve, (0, 0))
}
pub(super) fn stitch_edges(
    axis: Axis,
    frames: &[Positioned],
    cancel: &AtomicBool,
    reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
    edges: (u32, u32),
) -> Result<Composite, Error> {
    let plan = plan(axis, frames, cancel, edges)?;
    cancelled(cancel)?;
    if plan.segments.len() == 1 {
        // 单帧或完全重复覆盖无需分配输出，继续共享原像素租约。
        let frame = &frames[0].frame;
        return Ok(Composite {
            width: plan.width,
            height: plan.height,
            origin: plan.origin,
            pixels: frame.pixels.clone(),
        });
    }
    let mut lease = reserve(plan.bytes).ok_or(Error::Budget)?;
    cancelled(cancel)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(plan.bytes)
        .map_err(|_| Error::Budget)?;
    if !lease.grow_to(bytes.capacity()) {
        return Err(Error::Budget);
    }
    bytes.resize(plan.bytes, 0);
    let mut written = 0usize;
    for segment in plan.segments {
        cancelled(cancel)?;
        let frame = &frames[segment.frame].frame;
        match axis {
            Axis::Vertical => {
                let stride = plan.cross as usize * 4;
                for row in 0..segment.length as usize {
                    cancelled(cancel)?;
                    let source = (segment.offset as usize + row) * stride;
                    let target = (written + row) * stride;
                    bytes[target..target + stride]
                        .copy_from_slice(&frame.pixels[source..source + stride]);
                }
            }
            Axis::Horizontal => {
                let source_stride = frame.width as usize * 4;
                let target_stride = plan.width as usize * 4;
                let count = segment.length as usize * 4;
                for row in 0..plan.cross as usize {
                    cancelled(cancel)?;
                    let source = row * source_stride + segment.offset as usize * 4;
                    let target = row * target_stride + written * 4;
                    bytes[target..target + count]
                        .copy_from_slice(&frame.pixels[source..source + count]);
                }
            }
        }
        written += segment.length as usize;
    }
    debug_assert_eq!(written as i64, plan.end - plan.origin);
    cancelled(cancel)?;
    Ok(Composite {
        width: plan.width,
        height: plan.height,
        origin: plan.origin,
        pixels: Pixels::from_reserved(bytes, lease),
    })
}
