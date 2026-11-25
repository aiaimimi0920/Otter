//! Position frozen Live pins using the image region in the client area at commit time; remove window borders/letterboxing without resampling pixels.
use super::*;
use crate::pin_geometry::PinRect;
impl LiveWindow {
    /// The caller uses the application's per-monitor DPI context; dimensions must come from this retained frame.
    pub fn freeze_rect(&self, source: [u32; 2]) -> Result<PinRect> {
        if !self.is_alive() {
            return Err(Error::new(E_ABORT, "Live window is closed"));
        }
        unsafe {
            if IsIconic(self.hwnd).as_bool() {
                return Err(Error::new(E_ABORT, "Live window is minimized"));
            }
            let mut client = RECT::default();
            GetClientRect(self.hwnd, &mut client)?;
            let mut origin = POINT::default();
            if !ClientToScreen(self.hwnd, &mut origin).as_bool() {
                return Err(Error::from_win32());
            }
            let content = self
                .state
                .host
                .borrow()
                .as_ref()
                .map_or(source.map(f64::from), |host| host.content_size(source));
            physical_content_rect(
                source,
                [
                    (client.right - client.left).max(0) as u32,
                    (client.bottom - client.top).max(0) as u32,
                ],
                [origin.x, origin.y],
                content,
            )
            .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid Live freeze geometry"))
        }
    }
}
#[cfg(test)]
fn physical_rect(source: [u32; 2], client: [u32; 2], origin: [i32; 2]) -> Option<PinRect> {
    physical_content_rect(source, client, origin, source.map(f64::from))
}
fn physical_content_rect(
    source: [u32; 2],
    client: [u32; 2],
    origin: [i32; 2],
    content: [f64; 2],
) -> Option<PinRect> {
    let view = crate::live_viewport::Viewport::fit_content(source, client, content)?;
    let [left, top, right, bottom] = view
        .destination()
        .map(|value| f64::from(value).round() as i64);
    let x = i64::from(origin[0]).checked_add(left)?;
    let y = i64::from(origin[1]).checked_add(top)?;
    let width = u32::try_from(right.checked_sub(left)?).ok()?;
    let height = u32::try_from(bottom.checked_sub(top)?).ok()?;
    if width == 0
        || height == 0
        || width > 8192
        || height > 8192
        || u64::from(width) * u64::from(height) > 16_777_216
        || x + i64::from(width) > i64::from(i32::MAX)
        || y + i64::from(height) > i64::from(i32::MAX)
    {
        return None;
    }
    Some(PinRect {
        x: i32::try_from(x).ok()?,
        y: i32::try_from(y).ok()?,
        width,
        height,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_region_keeps_negative_screen_origin_and_removes_letterboxing() {
        assert_eq!(
            physical_rect([200, 100], [400, 400], [-500, -200]),
            Some(PinRect {
                x: -500,
                y: -100,
                width: 400,
                height: 200
            })
        );
        assert_eq!(
            physical_rect([100, 200], [400, 400], [20, 30]),
            Some(PinRect {
                x: 120,
                y: 30,
                width: 200,
                height: 400
            })
        );
        assert_eq!(
            physical_rect([2, 2], [301, 200], [0, 0]),
            Some(PinRect {
                x: 51,
                y: 0,
                width: 200,
                height: 200
            })
        );
    }
    #[test]
    fn empty_oversized_and_coordinate_overflow_are_not_clamped_into_other_geometry() {
        assert!(physical_rect([0, 2], [400, 400], [0, 0]).is_none());
        assert!(physical_rect([2, 2], [0, 400], [0, 0]).is_none());
        assert!(physical_rect([2, 2], [10000, 10000], [0, 0]).is_none());
        assert!(physical_rect([2, 2], [400, 400], [i32::MAX, 0]).is_none());
    }
}
