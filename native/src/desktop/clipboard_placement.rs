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
        let mut cursor = POINT::default();
        GetCursorPos(&mut cursor).ok()?;
        Self::capture_at(hwnd, state, cursor)
    }
    pub unsafe fn capture_at(hwnd: HWND, state: &super::State, cursor: POINT) -> Option<Self> {
        if !state.canvas.borrow().pinned {
            return None;
        }
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect).ok()?;
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
struct Source {
    unit: UnitId,
    frame: PinRect,
}
#[derive(Clone)]
pub(super) struct Request {
    identity: Arc<()>,
    revision: u64,
    origin: Origin,
    source: Source,
    cursor: (i32, i32),
    continuing: bool,
}
pub(super) struct Session {
    identity: Arc<()>,
    revision: u64,
    origin: Origin,
    source: Source,
    last_cursor: Option<(i32, i32)>,
}
impl Session {
    pub fn new(origin: Origin) -> Self {
        Self {
            identity: Arc::default(),
            revision: 0,
            source: Source {
                unit: origin.unit.clone(),
                frame: origin.frame,
            },
            origin,
            last_cursor: None,
        }
    }
    pub fn request(&self, cursor: (i32, i32)) -> Request {
        Request {
            identity: self.identity.clone(),
            revision: self.revision,
            origin: self.origin.clone(),
            source: self.source.clone(),
            cursor,
            continuing: self.last_cursor == Some(cursor),
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
            self.last_cursor = Some(request.cursor);
            self.revision = self.revision.wrapping_add(1);
        }
    }
}
impl Request {
    pub fn source(&self) -> (&UnitId, PinRect) {
        (&self.source.unit, self.source.frame)
    }
    pub fn cascade_in_source(&self, frame: PinRect) -> PinRect {
        // 固定 A 的身份不随副本更换；只有成功提交才推进次数，鼠标在 A 内移动不重置。
        let offset = (i128::from(self.revision) + 1) * i128::from(self.origin.step);
        self.position(i128::from(frame.x) + offset, i128::from(frame.y) + offset)
    }
    pub fn unit(&self) -> &UnitId {
        &self.origin.unit
    }
    pub fn continuing(&self) -> bool {
        self.continuing
    }
    pub fn contains_cursor(&self, frame: PinRect) -> bool {
        let (x, y) = (i64::from(self.cursor.0), i64::from(self.cursor.1));
        x >= i64::from(frame.x)
            && y >= i64::from(frame.y)
            && x <= i64::from(frame.x) + i64::from(frame.width)
            && y <= i64::from(frame.y) + i64::from(frame.height)
    }
    pub fn frame(&self, current: Option<PinRect>) -> PinRect {
        // 静止鼠标的连续粘贴沿成功落点推进，不能因新图移出鼠标而退回相对定位。
        let anchor = if self.continuing {
            self.origin.frame
        } else {
            current
                .filter(|frame| valid(*frame))
                .unwrap_or(self.origin.frame)
        };
        let (x, y) = if self.continuing || self.contains_cursor(anchor) {
            (
                i128::from(anchor.x) + i128::from(self.origin.step),
                i128::from(anchor.y) + i128::from(self.origin.step),
            )
        } else {
            return self.relative_frame();
        };
        self.position(x, y)
    }
    pub fn relative_frame(&self) -> PinRect {
        self.position(
            i128::from(self.cursor.0) - i128::from(self.origin.offset.0),
            i128::from(self.cursor.1) - i128::from(self.origin.offset.1),
        )
    }
    fn position(&self, x: i128, y: i128) -> PinRect {
        PinRect {
            x: x.clamp(
                i128::from(i32::MIN),
                i128::from(i32::MAX) - i128::from(self.origin.frame.width),
            ) as i32,
            y: y.clamp(
                i128::from(i32::MIN),
                i128::from(i32::MAX) - i128::from(self.origin.frame.height),
            ) as i32,
            ..self.origin.frame
        }
    }
}

#[cfg(test)]
#[path = "clipboard_placement_tests.rs"]
mod tests;
