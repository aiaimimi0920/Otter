//! 自动轴会话：首次唯一可靠运动才锁轴；拒绝帧不成为下一次匹配基准，不隐式升级二维。
use super::*;

pub(super) struct Detect {
    reference: Frame,
    session: Option<Session>,
    motion: Option<i64>,
}
impl Detect {
    pub fn new(first: Frame) -> Self {
        Self {
            reference: first,
            session: None,
            motion: None,
        }
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
                    .admit_next()?;
            }
            Ok(())
        }
    }
    pub fn append(&mut self, frame: Frame, cancel: &AtomicBool) -> Result<Append, Error> {
        if frame.dimensions() != self.reference.dimensions() {
            return Err(Error::Dimensions);
        }
        if let Some(session) = &mut self.session {
            if frame.bytes() != self.reference.bytes() {
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
            }
            let result = session.append(frame.clone(), cancel)?;
            if let Append::Added { .. } = result {
                self.motion = session.motion();
                self.reference = frame;
            }
            return Ok(result);
        }
        // 两个候选只共享帧存储；只有一轴通过时才移交其 Session。双轴可解释的纹理拒绝猜测。
        let (vertical, v) = candidate(Axis::Vertical, &self.reference, frame.clone(), cancel)?;
        let (horizontal, h) = candidate(Axis::Horizontal, &self.reference, frame.clone(), cancel)?;
        let accepted = match (v, h) {
            (Append::Duplicate, Append::Duplicate) => return Ok(Append::Duplicate),
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
        self.session
            .as_ref()
            .ok_or(Error::CrossAxis)?
            .to_spatial(cancel)
    }
    fn initial(&self) -> Result<Session, Error> {
        Session::new(
            Axis::Vertical,
            self.reference.clone(),
            options(Axis::Vertical, &self.reference),
        )
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
    let (width, height) = frame.dimensions();
    let length = if axis == Axis::Vertical {
        height
    } else {
        width
    };
    MatchOptions {
        minimum_overlap: (length / 4).clamp(1, 64),
        maximum_shift: length.saturating_sub(1),
        maximum_mean_error: 3.0,
    }
}

#[cfg(test)]
#[path = "long_capture_detect_tests.rs"]
mod tests;
