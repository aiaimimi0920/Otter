//! Clipboard placement uses unit identity, physical geometry and commit-only cascade anchors.
use crate::{pin_geometry::PinRect, workspace_id::UnitId};
use std::sync::Arc;
use windows::Win32::{
    Foundation::*,
    UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
};

#[derive(Clone)]
pub(super) struct Origin {
    pub unit: UnitId,
    pub frame: PinRect,
    pub offset: (i64, i64),
    pub step: i32,
}
impl Origin {
    pub unsafe fn capture(hwnd: HWND, state: &super::State) -> Option<Self> {
        if !state.canvas.borrow().pinned {
            return None;
        }
        let mut rect = RECT::default();
        let mut cursor = POINT::default();
        GetWindowRect(hwnd, &mut rect).ok()?;
        GetCursorPos(&mut cursor).ok()?;
        let frame = rectangle(rect)?;
        Some(Self {
            unit: state.unit_id.clone(),
            frame,
            offset: (
                i64::from(cursor.x) - i64::from(frame.x),
                i64::from(cursor.y) - i64::from(frame.y),
            ),
            step: ((20 * u64::from(GetDpiForWindow(hwnd).max(96))) / 96) as i32,
        })
    }
}
pub(super) fn rectangle(rect: RECT) -> Option<PinRect> {
    let frame = PinRect {
        x: rect.left,
        y: rect.top,
        width: u32::try_from(i64::from(rect.right) - i64::from(rect.left)).ok()?,
        height: u32::try_from(i64::from(rect.bottom) - i64::from(rect.top)).ok()?,
    };
    valid(frame).then_some(frame)
}
fn valid(frame: PinRect) -> bool {
    frame.width > 0
        && frame.height > 0
        && frame.width <= 8192
        && frame.height <= 8192
        && u64::from(frame.width) * u64::from(frame.height) <= 16_777_216
        && i64::from(frame.x) + i64::from(frame.width) <= i64::from(i32::MAX)
        && i64::from(frame.y) + i64::from(frame.height) <= i64::from(i32::MAX)
}
#[derive(Clone)]
pub(super) struct Request {
    identity: Arc<()>,
    revision: u64,
    origin: Origin,
    cursor: (i32, i32),
}
pub(super) struct Session {
    identity: Arc<()>,
    revision: u64,
    origin: Origin,
}
impl Session {
    pub fn new(origin: Origin) -> Self {
        Self {
            identity: Arc::default(),
            revision: 0,
            origin,
        }
    }
    pub fn request(&self, cursor: (i32, i32)) -> Request {
        Request {
            identity: self.identity.clone(),
            revision: self.revision,
            origin: self.origin.clone(),
            cursor,
        }
    }
    pub fn refresh(&self, request: &Request) -> Request {
        if Arc::ptr_eq(&self.identity, &request.identity) {
            self.request(request.cursor)
        } else {
            request.clone()
        }
    }
    pub fn commit(&mut self, request: &Request, unit: UnitId, frame: PinRect) {
        if Arc::ptr_eq(&self.identity, &request.identity)
            && self.revision == request.revision
            && valid(frame)
        {
            self.origin.unit = unit;
            self.origin.frame = frame;
            self.revision = self.revision.wrapping_add(1);
        }
    }
}
impl Request {
    pub fn unit(&self) -> &UnitId {
        &self.origin.unit
    }
    pub fn frame(&self, current: Option<PinRect>) -> PinRect {
        let anchor = current
            .filter(|frame| valid(*frame))
            .unwrap_or(self.origin.frame);
        let (x, y) = (i64::from(self.cursor.0), i64::from(self.cursor.1));
        let inside = x >= i64::from(anchor.x)
            && y >= i64::from(anchor.y)
            && x <= i64::from(anchor.x) + i64::from(anchor.width)
            && y <= i64::from(anchor.y) + i64::from(anchor.height);
        let (x, y) = if inside {
            (
                i64::from(self.origin.frame.x) + i64::from(self.origin.step),
                i64::from(self.origin.frame.y) + i64::from(self.origin.step),
            )
        } else {
            (x - self.origin.offset.0, y - self.origin.offset.1)
        };
        PinRect {
            x: x.clamp(
                i64::from(i32::MIN),
                i64::from(i32::MAX) - i64::from(self.origin.frame.width),
            ) as i32,
            y: y.clamp(
                i64::from(i32::MIN),
                i64::from(i32::MAX) - i64::from(self.origin.frame.height),
            ) as i32,
            ..self.origin.frame
        }
    }
}

#[cfg(test)]
#[path = "clipboard_placement_tests.rs"]
mod tests;
