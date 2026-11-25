//! Automatic-axis sessions lock only on first uniquely reliable motion; rejected frames never become matching references, and no implicit 2D upgrade.
use super::*;

pub(super) struct Detect {
    reference: Frame,
    session: Option<Session>,
    motion: Option<i64>,
    output: OutputTarget,
}
impl Detect {
    pub fn new(first: Frame) -> Self {
        Self {
            reference: first,
            session: None,
            motion: None,
            output: OutputTarget::Capture,
        }
    }
    pub fn with_output(mut self, output: OutputTarget) -> Result<Self, Error> {
        output.check(self.reference.width, self.reference.height)?;
        self.output = output;
        Ok(self)
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            axis: self.session.as_ref().map(Session::axis),
            direction: self.session.as_ref().and_then(|session| {
                self.motion
                    .and_then(|delta| Direction::from_offset(session.axis(), delta))
            }),
            frame_count: self.session.as_ref().map_or(1, Session::frame_count),
            confirmed_edges: self.session.as_ref().and_then(Session::confirmed_edges),
            spatial_position: None,
        }
    }
    pub fn admit_next(&self) -> Result<(), Error> {
        if let Some(session) = &self.session {
            session.admit_next()
        } else {
            for axis in [Axis::Vertical, Axis::Horizontal] {
                Session::new(axis, self.reference.clone(), options(axis, &self.reference))?
                    .with_output(self.output)?
                    .admit_next()?;
            }
            Ok(())
        }
    }
    pub fn append(&mut self, frame: Frame, cancel: &AtomicBool) -> Result<Append, Error> {
        if cancel.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        if frame.dimensions() != self.reference.dimensions() {
            return Err(Error::Dimensions);
        }
        // Stationary samples create no dual-axis candidates/reentry into locked-axis sessions; preserve direction/fixed-edge evidence.
        if frame.bytes() == self.reference.bytes() {
            return if cancel.load(Ordering::Acquire) {
                Err(Error::Cancelled)
            } else {
                Ok(Append::Duplicate)
            };
        }
        if let Some(session) = &mut self.session {
            let other = match session.axis() {
                Axis::Vertical => Axis::Horizontal,
                Axis::Horizontal => Axis::Vertical,
            };
            if matches!(
                candidate(other, &self.reference, frame.clone(), cancel)?.1,
                Append::Added { .. }
            ) {
                return Err(Error::CrossAxis);
            }
            let result = session.append(frame.clone(), cancel)?;
            if let Append::Added { .. } = result {
                self.motion = session.motion();
                self.reference = frame;
            }
            return Ok(result);
        }
        // Two candidates share only frame storage; transfer Session only if one axis passes, rejecting textures explainable on both axes.
        let (vertical, v) = candidate(Axis::Vertical, &self.reference, frame.clone(), cancel)?;
        let (horizontal, h) = candidate(Axis::Horizontal, &self.reference, frame.clone(), cancel)?;
        let accepted = match (v, h) {
            (
                Append::Added { .. },
                Append::Rejected(ApproximateMatch::NoOverlap | ApproximateMatch::LowTexture),
            ) => Some((vertical, v)),
            (
                Append::Rejected(ApproximateMatch::NoOverlap | ApproximateMatch::LowTexture),
                Append::Added { .. },
            ) => Some((horizontal, h)),
            _ => None,
        };
        if let Some((session, result)) = accepted {
            // Apply the limit only to the unique winning axis, not the probe of the other axis.
            let session = session.with_output(self.output)?;
            if cancel.load(Ordering::Acquire) {
                return Err(Error::Cancelled);
            }
            self.motion = session.motion();
            self.reference = frame;
            self.session = Some(session);
            Ok(result)
        } else {
            Ok(Append::Rejected(ApproximateMatch::Ambiguous))
        }
    }
    pub fn compose(&self, cancel: &AtomicBool) -> Result<Composite, Error> {
        match &self.session {
            Some(session) => session.compose(cancel),
            None => self.initial()?.compose(cancel),
        }
    }
    pub fn preview(&self, cancel: &AtomicBool) -> Result<Composite, Error> {
        match &self.session {
            Some(session) => session.preview(cancel),
            None => self.initial()?.preview(cancel),
        }
    }
    pub fn to_spatial(&self, cancel: &AtomicBool) -> Result<SpatialSession, Error> {
        if cancel.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        match &self.session {
            Some(session) => session.to_spatial(cancel),
            None => Ok(SpatialSession::detecting(self.reference.clone())?.with_output(self.output)),
        }
    }
    fn initial(&self) -> Result<Session, Error> {
        Session::new(
            Axis::Vertical,
            self.reference.clone(),
            options(Axis::Vertical, &self.reference),
        )?
        .with_output(self.output)
    }
}
fn candidate(
    axis: Axis,
    previous: &Frame,
    current: Frame,
    cancel: &AtomicBool,
) -> Result<(Session, Append), Error> {
    let mut session = Session::new(axis, previous.clone(), options(axis, previous))?;
    let result = session.append(current, cancel)?;
    Ok((session, result))
}
fn options(axis: Axis, frame: &Frame) -> MatchOptions {
    MatchOptions::for_detection(axis, frame)
}

#[cfg(test)]
#[path = "long_capture_detect_tests.rs"]
mod tests;
