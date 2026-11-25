//! Background capture for explicit selections; manual capture and bound-target single-step scrolling have separate admission, without focus control.
use crate::{
    long_capture::{
        Axis, CaptureMode, CaptureSource, Frame, MatchOptions, OutputTarget, Session, SessionJob,
    },
    pixels::Pixels,
};
use std::sync::atomic::Ordering;
#[path = "long_capture_source_owner.rs"]
mod owner;
#[path = "long_capture_scroll.rs"]
pub mod scroll;
#[path = "long_capture_stable.rs"]
mod stable;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}
/// first comes from confirmed selection; later capture commands let the same worker capture/append that physical rectangle.
/// The application must complete existing PER_MONITOR_AWARE_V2 initialization first; hosts stop sessions on screen-layout changes.
pub fn start(
    region: Region,
    first: Pixels,
    axis: Axis,
    options: MatchOptions,
) -> Result<SessionJob, String> {
    let session = prepare(region, first, axis, options)?;
    SessionJob::start_with_source(session, owner::Source::new(region, axis, None))
        .map_err(|error| error.to_string())
}
fn prepare(
    region: Region,
    first: Pixels,
    axis: Axis,
    options: MatchOptions,
) -> Result<Session, String> {
    super::capture::validate_region(region.left, region.top, region.width, region.height)
        .map_err(|error| error.to_string())?;
    let frame = Frame::new(region.width as u32, region.height as u32, first)
        .map_err(|error| format!("Invalid long capture frame: {error:?}"))?;
    Session::new(axis, frame, options)
        .and_then(|session| session.with_output(OutputTarget::Document))
        .map_err(|error| format!("Invalid long capture session: {error:?}"))
}
fn capture(region: Region, cancel: &std::sync::atomic::AtomicBool) -> Result<Frame, String> {
    if cancel.load(Ordering::Acquire) {
        return Err("Long capture cancelled".into());
    }
    // GDI calls are uninterruptible; recheck cancellation immediately on return to prevent late-image publication.
    let image = super::capture::region(region.left, region.top, region.width, region.height)
        .map_err(|error| error.to_string())?;
    if cancel.load(Ordering::Acquire) {
        return Err("Long capture cancelled".into());
    }
    Frame::new(image.width, image.height, image.pixels)
        .map_err(|error| format!("Invalid captured frame: {error:?}"))
}
/// Bind before showing controls; on failure UI preserves manual mode rather than pretending automatic scrolling is supported.
pub fn start_scrollable(
    region: Region,
    first: Pixels,
    axis: Axis,
    options: MatchOptions,
) -> Result<SessionJob, String> {
    let session = prepare(region, first, axis, options)?;
    let target = scroll::Target::bind(region)?;
    SessionJob::start_with_source(session, owner::Source::new(region, axis, Some(target)))
        .map_err(|error| error.to_string())
}
/// Freeze targets before controls appear. Observation sources cannot actively send input; old active-capture entry points are unchanged.
pub fn start_recording(region: Region) -> Result<(SessionJob, scroll::Target), String> {
    super::capture::validate_region(region.left, region.top, region.width, region.height)
        .map_err(|error| error.to_string())?;
    // The selection window is destroyed and recording windows not yet created; await compositor transition before binding/capturing the first frame.
    unsafe { windows::Win32::Graphics::Dwm::DwmFlush() }.map_err(|error| error.to_string())?;
    let target = scroll::Target::bind(region)?;
    let mut source = owner::Source::recording(region, target.clone());
    let first = source.capture(
        CaptureMode::Current,
        &std::sync::atomic::AtomicBool::new(false),
    )?;
    let job = SessionJob::start_detecting_for(first, source, OutputTarget::Document)
        .map_err(|error| error.to_string())?;
    Ok((job, target))
}
// Stable sampling retains at most previous BGRA, current DIB, and current BGRA together. Check peak usage before input,
// without holding leases long-term; actual GDI allocations still pass individual budgets to handle races with later allocations by other documents.
fn capture_headroom(region: Region) -> Result<(), String> {
    let bytes = u64::try_from(region.width)
        .ok()
        .and_then(|width| {
            u64::try_from(region.height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(12))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .filter(|bytes| *bytes > 0)
        .ok_or("Invalid capture allocation dimensions")?;
    let _headroom = crate::pixels::reserve(bytes)
        .ok_or("Insufficient pixel budget for capture stabilization")?;
    Ok(())
}
fn settle(cancel: &std::sync::atomic::AtomicBool) -> Result<(), String> {
    wait(cancel, std::time::Duration::from_millis(250))
}
fn wait(
    cancel: &std::sync::atomic::AtomicBool,
    duration: std::time::Duration,
) -> Result<(), String> {
    let deadline = std::time::Instant::now() + duration;
    loop {
        if cancel.load(Ordering::Acquire) {
            return Err("Long capture cancelled while settling".into());
        }
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Ok(());
        }
        std::thread::sleep(remaining.min(std::time::Duration::from_millis(10)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_headroom_checks_dimensions_without_allocating_or_capturing() {
        for (width, height) in [(0, 2), (-1, 2), (2, -1), (i32::MAX, i32::MAX)] {
            assert!(capture_headroom(Region {
                left: 0,
                top: 0,
                width,
                height
            })
            .is_err());
        }
        assert!(capture_headroom(Region {
            left: -10,
            top: -20,
            width: 2,
            height: 2
        })
        .is_ok());
    }
    #[test]
    fn cancelled_settle_does_not_continue_to_capture() {
        assert!(settle(&std::sync::atomic::AtomicBool::new(true)).is_err());
    }
    #[test]
    fn invalid_region_and_frame_fail_without_starting_capture() {
        let options = MatchOptions {
            minimum_overlap: 1,
            maximum_shift: 1,
            maximum_mean_error: 3.0,
        };
        for region in [
            Region {
                left: i32::MAX,
                top: 0,
                width: 2,
                height: 2,
            },
            Region {
                left: 0,
                top: 0,
                width: 0,
                height: 2,
            },
        ] {
            assert!(start(region, vec![0; 16].into(), Axis::Vertical, options).is_err());
        }
        let region = Region {
            left: -10,
            top: -20,
            width: 2,
            height: 2,
        };
        assert!(start(region, vec![0; 4].into(), Axis::Vertical, options).is_err());
        // Even valid construction performs no capture; without explicit capture commands, workers wait only on channels.
        let mut job = start(region, vec![0; 16].into(), Axis::Vertical, options).unwrap();
        job.cancel();
    }
}
