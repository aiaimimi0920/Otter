//! Single-owner 2D capture-session state; validate before commit so errors preserve last-accepted frames/accumulated coordinates.
use super::*;
use std::sync::atomic::Ordering;
#[path = "long_capture_spatial_detect.rs"]
pub mod detecting;
#[path = "long_capture_spatial_preview.rs"]
pub mod preview;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpatialAppend {
    Added { x: i64, y: i64 },
    Duplicate,
    Rejected(ApproximateMatch),
}
pub struct SpatialSession {
    frames: Vec<SpatialFrame>,
    reference: Frame,
    cursor: (i64, i64),
    accepted_count: usize,
    input_total: u64,
    axis: Axis,
    detect_axis: bool,
    options: MatchOptions,
    output: OutputTarget,
}
impl SpatialSession {
    pub fn new(axis: Axis, first: Frame, options: MatchOptions) -> Result<Self, Error> {
        // Share parameter admission with 1D sessions; cloning adds only shared pixel references.
        Session::new(axis, first.clone(), options)?;
        Ok(Self {
            reference: first.clone(),
            cursor: (0, 0),
            accepted_count: 1,
            input_total: u64::from(first.width) * u64::from(first.height),
            frames: vec![SpatialFrame {
                frame: first,
                x: 0,
                y: 0,
            }],
            axis,
            detect_axis: false,
            options,
            output: OutputTarget::Capture,
        })
    }
    // Called only on conversion from a session (or first frame) already admitted to this target.
    pub(super) fn with_output(mut self, output: OutputTarget) -> Self {
        self.output = output;
        self
    }
    pub fn frame_count(&self) -> usize {
        self.accepted_count
    }
    pub fn axis(&self) -> Axis {
        self.axis
    }
    pub(super) fn detected_axis(&self) -> Option<Axis> {
        (!self.detect_axis).then_some(self.axis)
    }
    pub(super) fn detecting(first: Frame) -> Result<Self, Error> {
        let options = MatchOptions::for_detection(Axis::Vertical, &first);
        let mut session = Self::new(Axis::Vertical, first, options)?;
        session.detect_axis = true;
        Ok(session)
    }
    pub fn position(&self) -> (i64, i64) {
        self.cursor
    }
    pub(super) fn admit_next(&self) -> Result<(), Error> {
        if self.accepted_count >= MAX_FRAMES {
            return Err(Error::FrameLimit);
        }
        if self.input_total + u64::from(self.reference.width) * u64::from(self.reference.height)
            > MAX_INPUT_PIXELS
        {
            return Err(Error::InputLimit);
        }
        let (_, _, mut width, mut height) = spatial::bounds(&self.frames, &AtomicBool::new(false))?;
        let length = match self.axis {
            Axis::Vertical => self.reference.height,
            Axis::Horizontal => self.reference.width,
        };
        let shift = self
            .options
            .maximum_shift
            .min(length - self.options.minimum_overlap);
        match self.axis {
            Axis::Vertical => height = height.checked_add(shift).ok_or(Error::OutputLimit)?,
            Axis::Horizontal => width = width.checked_add(shift).ok_or(Error::OutputLimit)?,
        }
        if u64::from(width) * u64::from(height) > 64 * 1024 * 1024 {
            return Err(Error::OutputLimit);
        }
        let bytes = output_bytes(width, height)?
            .checked_add(self.reference.pixels.len())
            .ok_or(Error::Budget)?;
        let _headroom = crate::pixels::reserve(bytes).ok_or(Error::Budget)?;
        Ok(())
    }
    pub(super) fn from_capture(
        axis: Axis,
        options: MatchOptions,
        aggregate: Composite,
        reference: Frame,
        cursor: (i64, i64),
        accepted_count: usize,
        input_total: u64,
    ) -> Result<Self, Error> {
        // Aggregate images are not new captures: allow axes beyond single-frame limits, while retaining output/2D 64 MP limits.
        let expected = output_bytes(aggregate.width, aggregate.height)?;
        if aggregate.pixels.len() != expected {
            return Err(Error::Pixels);
        }
        let (x, y) = match axis {
            Axis::Vertical => (0, aggregate.origin),
            Axis::Horizontal => (aggregate.origin, 0),
        };
        let frames = vec![SpatialFrame {
            x,
            y,
            frame: Frame {
                width: aggregate.width,
                height: aggregate.height,
                pixels: aggregate.pixels,
                signatures: std::sync::Arc::new([
                    std::sync::OnceLock::new(),
                    std::sync::OnceLock::new(),
                ]),
            },
        }];
        spatial::bounds(&frames, &AtomicBool::new(false))?;
        Ok(Self {
            frames,
            reference,
            cursor,
            accepted_count,
            input_total,
            axis,
            detect_axis: false,
            options,
            output: OutputTarget::Capture,
        })
    }
    /// movement is physical viewport displacement relative to the last accepted frame; requests after failure still use that reference.
    pub fn append(
        &mut self,
        frame: Frame,
        movement: (i64, i64),
        cancel: &AtomicBool,
    ) -> Result<SpatialAppend, Error> {
        self.append_with_reserve(frame, movement, cancel, crate::pixels::reserve)
    }
    fn append_with_reserve(
        &mut self,
        frame: Frame,
        movement: (i64, i64),
        cancel: &AtomicBool,
        reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
    ) -> Result<SpatialAppend, Error> {
        check(cancel)?;
        let (matched, detected) = if self.detect_axis {
            detecting::match_frame(&self.reference, &frame, movement, cancel)?
        } else {
            (
                match_spatial(
                    &self.reference,
                    &frame,
                    movement,
                    self.axis,
                    self.options,
                    cancel,
                )?,
                None,
            )
        };
        let (dx, dy) = match matched {
            SpatialMatch::Rejected(reason) => return Ok(SpatialAppend::Rejected(reason)),
            SpatialMatch::Offset { x, y, .. } => (x, y),
        };
        if dx == 0 && dy == 0 && frame.bytes() == self.reference.bytes() {
            check(cancel)?;
            return Ok(SpatialAppend::Duplicate);
        }
        let x = self.cursor.0.checked_add(dx).ok_or(Error::OutputLimit)?;
        let y = self.cursor.1.checked_add(dy).ok_or(Error::OutputLimit)?;
        if self.accepted_count >= MAX_FRAMES {
            return Err(Error::FrameLimit);
        }
        let total = self.input_total + u64::from(frame.width) * u64::from(frame.height);
        if total > MAX_INPUT_PIXELS {
            return Err(Error::InputLimit);
        }
        self.frames.try_reserve(1).map_err(|_| Error::Budget)?;
        let reference = frame.clone();
        self.frames.push(SpatialFrame { frame, x, y });
        // Temporary candidates are visible only under exclusive &mut and popped on every failure; acceptance occurs afterward.
        let admitted = (|| {
            let bounds @ (_, _, width, height) = spatial::bounds(&self.frames, cancel)?;
            spatial::check_output(self.output, &self.frames, bounds, cancel)?;
            let _headroom = reserve(output_bytes(width, height)?).ok_or(Error::Budget)?;
            check(cancel)
        })();
        if let Err(error) = admitted {
            self.frames.pop();
            return Err(error);
        }
        self.reference = reference;
        if let Some(axis) = detected {
            self.axis = axis;
            self.options = MatchOptions::for_detection(axis, &self.reference);
            self.detect_axis = false;
        }
        self.cursor = (x, y);
        self.accepted_count += 1;
        self.input_total = total;
        Ok(SpatialAppend::Added { x, y })
    }
    pub fn compose(&self, cancel: &AtomicBool) -> Result<SpatialComposite, Error> {
        stitch_spatial(&self.frames, cancel)
    }
    /// Previews include transparent holes/full coverage, without cropping to the final export's largest rectangle.
    pub fn preview(&self, cancel: &AtomicBool) -> Result<SpatialComposite, Error> {
        preview::render(&self.frames, cancel)
    }
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
#[cfg(test)]
#[path = "long_capture_spatial_session_tests.rs"]
mod tests;
