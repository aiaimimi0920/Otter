//! Worker-exclusive active sessions; old 1D sessions remain authoritative until first spatial append succeeds.
use super::*;
pub(super) enum Active {
    Linear(Session),
    Detecting(detect::Detect),
    Spatial(SpatialSession, Option<Direction>),
}
impl Active {
    pub fn snapshot(&self) -> Snapshot {
        match self {
            Self::Linear(session) => snapshot(session),
            Self::Detecting(session) => session.snapshot(),
            Self::Spatial(session, motion) => Snapshot {
                axis: session.detected_axis(),
                direction: *motion,
                frame_count: session.frame_count(),
                confirmed_edges: None,
                spatial_position: Some(session.position()),
            },
        }
    }
    pub fn append_at(
        &mut self,
        frame: Frame,
        movement: (i64, i64),
        cancel: &AtomicBool,
    ) -> Completion {
        match self {
            Self::Spatial(session, motion) => {
                Completion::SpatialAppend(append_spatial(session, motion, frame, movement, cancel))
            }
            Self::Detecting(session) if movement == (0, 0) => {
                Completion::Append(session.append(frame, cancel))
            }
            Self::Detecting(session) => {
                let mut motion = session.snapshot().direction;
                let mut candidate = match session.to_spatial(cancel) {
                    Ok(candidate) => candidate,
                    Err(error) => return Completion::SpatialAppend(Err(error)),
                };
                let result = append_spatial(&mut candidate, &mut motion, frame, movement, cancel);
                if matches!(result, Ok(SpatialAppend::Added { .. })) {
                    *self = Self::Spatial(candidate, motion);
                }
                Completion::SpatialAppend(result)
            }
            Self::Linear(session) if movement == (0, 0) => {
                Completion::Append(session.append(frame, cancel))
            }
            Self::Linear(session) => {
                let mut motion = session
                    .motion()
                    .and_then(|delta| Direction::from_offset(session.axis(), delta));
                let mut candidate = match session.to_spatial(cancel) {
                    Ok(candidate) => candidate,
                    Err(error) => return Completion::SpatialAppend(Err(error)),
                };
                let result = append_spatial(&mut candidate, &mut motion, frame, movement, cancel);
                if matches!(result, Ok(SpatialAppend::Added { .. })) {
                    // Single-owner replacement also releases old frames; failure/duplication/ambiguity never switches or changes reference coordinates.
                    *self = Self::Spatial(candidate, motion);
                }
                Completion::SpatialAppend(result)
            }
        }
    }
    pub fn append(&mut self, frame: Frame, cancel: &AtomicBool) -> Completion {
        self.append_at(frame, (0, 0), cancel)
    }
    pub fn compose(&self, cancel: &AtomicBool) -> Completion {
        match self {
            Self::Linear(session) => Completion::Compose(session.compose(cancel)),
            Self::Detecting(session) => Completion::Compose(session.compose(cancel)),
            Self::Spatial(session, _) => Completion::SpatialCompose(session.compose(cancel)),
        }
    }
    pub fn preview(&self, cancel: &AtomicBool) -> Completion {
        match self {
            Self::Linear(session) => Completion::Preview(session.preview(cancel)),
            Self::Detecting(session) => Completion::Preview(session.preview(cancel)),
            Self::Spatial(session, _) => Completion::SpatialPreview(session.preview(cancel)),
        }
    }
    pub fn admit_next(&self) -> Result<(), Error> {
        match self {
            Self::Linear(session) => session.admit_next(),
            Self::Detecting(session) => session.admit_next(),
            Self::Spatial(session, _) => session.admit_next(),
        }
    }
}

// Initial conversion/subsequent appends share direction-commit rules; physical viewport movement is not page scrolling.
fn append_spatial(
    session: &mut SpatialSession,
    motion: &mut Option<Direction>,
    frame: Frame,
    movement: (i64, i64),
    cancel: &AtomicBool,
) -> Result<SpatialAppend, Error> {
    let before = session.position();
    let result = session.append(frame, movement, cancel);
    if let (Ok(SpatialAppend::Added { x, y }), Some(axis)) = (&result, session.detected_axis()) {
        let delta = match axis {
            Axis::Vertical => y - before.1 - movement.1,
            Axis::Horizontal => x - before.0 - movement.0,
        };
        if delta != 0 {
            *motion = Direction::from_offset(axis, delta);
        }
    }
    result
}

#[cfg(test)]
#[path = "long_capture_active_tests.rs"]
mod tests;
