//! Pure-data capture sessions hold shared frames and validate before append commit; no desktop input/thread scheduling.
use super::*;
use std::sync::atomic::Ordering;
#[path = "long_capture_session_edges.rs"]
pub mod edges;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Append {
    Added { origin: i64 },
    Duplicate,
    Rejected(ApproximateMatch),
}
pub struct Session {
    axis: Axis,
    options: MatchOptions,
    frames: Vec<Positioned>,
    edges: Option<(u32, u32)>,
    confirmations: u8,
    total: u64,
    minimum: i64,
    maximum: i64,
    motion: Option<i64>,
    output: OutputTarget,
}
impl Session {
    pub fn new(axis: Axis, first: Frame, options: MatchOptions) -> Result<Self, Error> {
        let length = axis_length(axis, &first);
        if options.minimum_overlap == 0
            || options.minimum_overlap > length
            || !options.maximum_mean_error.is_finite()
            || !(0.0..=12.0).contains(&options.maximum_mean_error)
        {
            return Err(Error::Dimensions);
        }
        let total = u64::from(first.width) * u64::from(first.height);
        Ok(Self {
            axis,
            options,
            frames: vec![Positioned {
                frame: first,
                origin: 0,
            }],
            edges: None,
            confirmations: 0,
            total,
            minimum: 0,
            maximum: 0,
            motion: None,
            output: OutputTarget::Capture,
        })
    }
    pub fn with_output(mut self, output: OutputTarget) -> Result<Self, Error> {
        let frame = &self.frames[0].frame;
        // Every committed append already validated the span before updating these bounds.
        let span = (self.maximum - self.minimum) as u32 + axis_length(self.axis, frame);
        let (width, height) = self.output_size(frame, span);
        output.check(width, height)?;
        self.output = output;
        Ok(self)
    }
    fn output_size(&self, frame: &Frame, span: u32) -> (u32, u32) {
        match self.axis {
            Axis::Vertical => (frame.width, span),
            Axis::Horizontal => (span, frame.height),
        }
    }
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
    pub fn axis(&self) -> Axis {
        self.axis
    }
    /// Latest committed image displacement; duplication/rejection/cancellation/budget failure never changes direction evidence.
    pub fn motion(&self) -> Option<i64> {
        self.motion
    }
    // Check known frame-count/logical-input limits before scrolling, not after altering external apps and discovering the next frame cannot fit.
    pub(super) fn admit_next(&self) -> Result<(), Error> {
        if self.frames.len() >= MAX_FRAMES {
            return Err(Error::FrameLimit);
        }
        let first = &self.frames[0].frame;
        if self.total + u64::from(first.width) * u64::from(first.height) > MAX_INPUT_PIXELS {
            return Err(Error::InputLimit);
        }
        // Next displacement is unknown; before scrolling, check simultaneous final-output/next-frame residency at maximum permitted expansion.
        let length = axis_length(self.axis, first);
        let shift = self
            .options
            .maximum_shift
            .min(length - self.options.minimum_overlap);
        let span = self
            .maximum
            .checked_sub(self.minimum)
            .and_then(|n| n.checked_add(i64::from(length) + i64::from(shift)))
            .and_then(|n| u32::try_from(n).ok())
            .ok_or(Error::OutputLimit)?;
        let (width, height) = self.output_size(first, span);
        self.output.check(width, height)?;
        let bytes = output_bytes(width, height)?
            .checked_add(first.pixels.len())
            .ok_or(Error::Budget)?;
        let _headroom = crate::pixels::reserve(bytes).ok_or(Error::Budget)?;
        Ok(())
    }
    /// Confirm consistent fixed edges after at least two nonzero movements; zero edges need not await a third frame.
    pub fn confirmed_edges(&self) -> Option<(u32, u32)> {
        self.edges
            .filter(|edges| *edges == (0, 0) || self.confirmations >= 2)
    }
    pub fn append(&mut self, frame: Frame, cancel: &AtomicBool) -> Result<Append, Error> {
        self.append_with_reserve(frame, cancel, crate::pixels::reserve)
    }
    fn append_with_reserve(
        &mut self,
        frame: Frame,
        cancel: &AtomicBool,
        reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
    ) -> Result<Append, Error> {
        check(cancel)?;
        let previous = self
            .frames
            .last()
            .expect("session always has a first frame");
        if previous.frame.dimensions() != frame.dimensions() {
            return Err(Error::Dimensions);
        }
        // Exact duplicates consume no frame/input budget and leave last-accepted frames/boundary evidence unchanged.
        let found = if self.edges.is_none_or(|edges| edges == (0, 0)) {
            match match_exact(
                &previous.frame,
                &frame,
                self.axis,
                self.options.minimum_overlap,
                cancel,
            )? {
                Match::Duplicate => FixedMatch::Unchanged,
                Match::Offset(delta)
                    if delta.unsigned_abs() <= u64::from(self.options.maximum_shift) =>
                {
                    FixedMatch::Motion {
                        leading: 0,
                        trailing: 0,
                        delta,
                        mean_error: 0.0,
                    }
                }
                Match::Offset(_) => FixedMatch::Unresolved(ApproximateMatch::NoOverlap),
                Match::Ambiguous => FixedMatch::Unresolved(ApproximateMatch::Ambiguous),
                Match::NoOverlap => {
                    if let Some(edges) = self.edges {
                        edges::match_next(
                            &self.frames,
                            &frame,
                            self.axis,
                            self.options,
                            edges,
                            cancel,
                        )?
                    } else {
                        match_fixed_edges(&previous.frame, &frame, self.axis, self.options, cancel)?
                    }
                }
            }
        } else {
            edges::match_next(
                &self.frames,
                &frame,
                self.axis,
                self.options,
                self.edges.expect("fixed-edge branch"),
                cancel,
            )?
        };
        let (edges, delta) = match found {
            FixedMatch::Unchanged => return Ok(Append::Duplicate),
            FixedMatch::Unresolved(result) => return Ok(Append::Rejected(result)),
            FixedMatch::Motion {
                leading,
                trailing,
                delta,
                ..
            } => ((leading, trailing), delta),
        };
        if self.frames.len() >= MAX_FRAMES {
            return Err(Error::FrameLimit);
        }
        let total = self.total + u64::from(frame.width) * u64::from(frame.height);
        if total > MAX_INPUT_PIXELS {
            return Err(Error::InputLimit);
        }
        let origin = previous
            .origin
            .checked_add(delta)
            .ok_or(Error::OutputLimit)?;
        let minimum = self.minimum.min(origin);
        let maximum = self.maximum.max(origin);
        let length = axis_length(self.axis, &frame);
        let span = maximum
            .checked_sub(minimum)
            .and_then(|n| n.checked_add(i64::from(length)))
            .and_then(|n| u32::try_from(n).ok())
            .ok_or(Error::OutputLimit)?;
        minimum
            .checked_sub(i64::from(edges.0))
            .ok_or(Error::OutputLimit)?;
        let (width, height) = self.output_size(&frame, span);
        self.output.check(width, height)?;
        // Candidate pixels already consume shared budgets; confirm full output can coexist through a short lease, without long-image allocation.
        // Release leases when this admission ends; other documents may later consume budget, so composition rechecks.
        let _output_headroom = reserve(output_bytes(width, height)?).ok_or(Error::Budget)?;
        check(cancel)?;
        self.frames.try_reserve(1).map_err(|_| Error::Budget)?;
        self.frames.push(Positioned { frame, origin });
        self.edges = Some(edges);
        self.confirmations = self.confirmations.saturating_add(1);
        self.total = total;
        self.minimum = minimum;
        self.maximum = maximum;
        self.motion = Some(delta);
        Ok(Append::Added { origin })
    }
    /// Do not consume sessions: output-budget failure/cancellation allows retry; Drop releases all input-frame references.
    pub fn compose(&self, cancel: &AtomicBool) -> Result<Composite, Error> {
        check(cancel)?;
        if self.frames.len() == 1 {
            return stitch(self.axis, &self.frames, cancel);
        }
        let edges = self.confirmed_edges().ok_or(Error::UnconfirmedEdges)?;
        stitch_fixed_edges(self.axis, &self.frames, edges.0, edges.1, cancel)
    }
    pub fn preview(&self, cancel: &AtomicBool) -> Result<Composite, Error> {
        check(cancel)?;
        let edges = if self.frames.len() == 1 {
            (0, 0)
        } else {
            self.confirmed_edges().ok_or(Error::UnconfirmedEdges)?
        };
        compose::preview(self.axis, &self.frames, edges, cancel)
    }
    /// Create independent candidates without consuming current sessions; callers replace old sessions only after subsequent moving frames are accepted.
    pub fn to_spatial(&self, cancel: &AtomicBool) -> Result<SpatialSession, Error> {
        let aggregate = self.compose(cancel)?;
        let last = self
            .frames
            .last()
            .expect("session always has a first frame");
        let leading = self.confirmed_edges().map_or(0, |edges| edges.0);
        let origin = last
            .origin
            .checked_sub(i64::from(leading))
            .ok_or(Error::OutputLimit)?;
        let cursor = match self.axis {
            Axis::Vertical => (0, origin),
            Axis::Horizontal => (origin, 0),
        };
        check(cancel)?;
        let converted = SpatialSession::from_capture(
            self.axis,
            self.options,
            aggregate,
            last.frame.clone(),
            cursor,
            self.frames.len(),
            self.total,
        )?;
        check(cancel)?;
        Ok(converted.with_output(self.output))
    }
}
fn axis_length(axis: Axis, frame: &Frame) -> u32 {
    match axis {
        Axis::Vertical => frame.height,
        Axis::Horizontal => frame.width,
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
#[path = "long_capture_session_tests.rs"]
mod tests;
