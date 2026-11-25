//! Positioned-frame stitching primitives for scrolling capture; matchers supply positions, with no guessed scrolling or screen capture.
use crate::pixels::Pixels;
use std::sync::atomic::AtomicBool;
#[path = "long_capture_compose.rs"]
pub mod compose;
#[path = "long_capture_match.rs"]
pub mod matching;
#[path = "long_capture_view.rs"]
pub mod view;
pub use matching::{match_exact, Match};
#[path = "long_capture_approx.rs"]
pub mod approximate;
pub use approximate::{match_approximate, ApproximateMatch, MatchOptions};
#[path = "long_capture_fixed.rs"]
pub mod fixed;
pub use fixed::{match_fixed_edges, stitch_fixed_edges, FixedMatch};
#[path = "long_capture_session.rs"]
pub mod session;
pub use session::{Append, Session};
#[path = "long_capture_job.rs"]
pub mod job;
pub use job::{CaptureMode, CaptureSource, Completion, DispatchError, Reply, SessionJob, Snapshot};
#[path = "long_capture_spatial.rs"]
pub mod spatial;
pub use spatial::{stitch_spatial, SpatialComposite, SpatialFrame};
#[path = "long_capture_spatial_match.rs"]
pub mod spatial_matching;
pub use spatial_matching::{match_spatial, SpatialMatch};
#[path = "long_capture_spatial_session.rs"]
pub mod spatial_session;
pub use spatial_session::{SpatialAppend, SpatialSession};
pub const MAX_FRAMES: usize = 512;
const MAX_FRAME_AXIS: u32 = 32_768;
const MAX_FRAME_PIXELS: u64 = 64 * 1024 * 1024;
const MAX_INPUT_PIXELS: u64 = 128 * 1024 * 1024;
const MAX_OUTPUT_AXIS: u32 = 131_072;
const MAX_OUTPUT_PIXELS: u64 = 128 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Vertical,
    Horizontal,
}
/// Accepted-image movement direction in stitching coordinates, not wheel signs or direct input-event assignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}
impl Direction {
    pub fn from_offset(axis: Axis, delta: i64) -> Option<Self> {
        match (axis, delta.signum()) {
            (Axis::Vertical, -1) => Some(Self::Up),
            (Axis::Vertical, 1) => Some(Self::Down),
            (Axis::Horizontal, -1) => Some(Self::Left),
            (Axis::Horizontal, 1) => Some(Self::Right),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Empty,
    FrameLimit,
    Dimensions,
    Pixels,
    InputLimit,
    CrossAxis,
    Gap,
    OutputLimit,
    DocumentLimit,
    Budget,
    Cancelled,
    ChangedEdges,
    UnconfirmedEdges,
}
/// The stitcher can assemble images larger than a desktop document accepts.
#[derive(Clone, Copy)]
pub enum OutputTarget {
    Capture,
    Document,
}
impl OutputTarget {
    fn check(self, width: u32, height: u32) -> Result<(), Error> {
        match self {
            Self::Document if crate::viewport::checked_image_bytes(width, height).is_none() => {
                Err(Error::DocumentLimit)
            }
            _ => Ok(()),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Frame {
    width: u32,
    height: u32,
    pixels: Pixels,
}
impl Frame {
    /// Tightly packed BGRA pixels; Frame clones share only pixels/budget leases.
    pub fn new(width: u32, height: u32, pixels: Pixels) -> Result<Self, Error> {
        let count = frame_pixels(width, height)?;
        if count * 4 != pixels.len() as u64 {
            return Err(Error::Pixels);
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    pub fn bytes(&self) -> &[u8] {
        &self.pixels
    }
}
/// Absolute position on the scrolling axis; negative coordinates are allowed and first frames need not start at zero.
pub struct Positioned {
    pub frame: Frame,
    pub origin: i64,
}
pub struct Composite {
    pub width: u32,
    pub height: u32,
    pub origin: i64,
    pub pixels: Pixels,
}

/// Preserve earlier-frame coverage and add only new regions, without alpha blending/implicit color conversion.
/// Reject holes/cross-axis dimension mismatches/overflow before output allocation; shared pixel budgets may reject before per-image limits.
pub fn stitch(axis: Axis, frames: &[Positioned], cancel: &AtomicBool) -> Result<Composite, Error> {
    compose::stitch(axis, frames, cancel, crate::pixels::reserve)
}
fn frame_pixels(width: u32, height: u32) -> Result<u64, Error> {
    if width == 0 || height == 0 || width > MAX_FRAME_AXIS || height > MAX_FRAME_AXIS {
        return Err(Error::Dimensions);
    }
    let count = u64::from(width) * u64::from(height);
    if count > MAX_FRAME_PIXELS {
        return Err(Error::Dimensions);
    }
    Ok(count)
}
fn output_bytes(width: u32, height: u32) -> Result<usize, Error> {
    let count = u64::from(width) * u64::from(height);
    if width == 0
        || height == 0
        || width > MAX_OUTPUT_AXIS
        || height > MAX_OUTPUT_AXIS
        || count > MAX_OUTPUT_PIXELS
    {
        return Err(Error::OutputLimit);
    }
    usize::try_from(count * 4).map_err(|_| Error::OutputLimit)
}

#[cfg(test)]
#[path = "long_capture_tests.rs"]
mod tests;
