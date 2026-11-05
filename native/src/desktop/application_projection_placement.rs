//! 新接收图在当前工作区首窗口所在显示器工作区居中，等比缩小、不放大或复制旧窗口尺寸。
use super::*;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};

pub(super) unsafe fn bounds(hwnd: HWND, space: Space) -> Result<Rect> {
    let mut monitor = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !GetMonitorInfoW(
        MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST),
        &mut monitor,
    )
    .as_bool()
    {
        return Err(stale());
    }
    let rect = monitor.rcWork;
    let width = u32::try_from(i64::from(rect.right) - i64::from(rect.left))
        .map_err(|_| stale())?
        .min(1600);
    let height = u32::try_from(i64::from(rect.bottom) - i64::from(rect.top))
        .map_err(|_| stale())?
        .min(1000);
    let x = i64::from(rect.left)
        + (i64::from(rect.right) - i64::from(rect.left) - i64::from(width)) / 2;
    let y = i64::from(rect.top)
        + (i64::from(rect.bottom) - i64::from(rect.top) - i64::from(height)) / 2;
    space
        .logical(
            crate::pin_geometry::PinRect {
                x: i32::try_from(x).map_err(|_| stale())?,
                y: i32::try_from(y).map_err(|_| stale())?,
                width,
                height,
            },
            None,
        )
        .map_err(|_| stale())
}
pub(in crate::desktop::application) fn fit(
    bounds: Rect,
    space: Space,
    size: (u32, u32),
) -> Result<Rect> {
    if !bounds.valid()
        || !space.scale.is_finite()
        || space.scale <= 0.0
        || size.0 == 0
        || size.1 == 0
    {
        return Err(stale());
    }
    let (width, height) = (
        f64::from(size.0) / space.scale,
        f64::from(size.1) / space.scale,
    );
    let scale = (bounds.w * 0.8 / width)
        .min(bounds.h * 0.8 / height)
        .min(1.0);
    let rect = Rect {
        x: bounds.x + (bounds.w - width * scale) / 2.0,
        y: bounds.y + (bounds.h - height * scale) / 2.0,
        w: width * scale,
        h: height * scale,
    };
    space.physical(rect).map_err(|_| stale())?;
    Ok(rect)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projection_placement_keeps_aspect_pixels_and_negative_monitor_origin_at_fractional_scale() {
        let space = Space {
            origin: (-1920, -200),
            scale: 1.5,
        };
        let bounds = Rect {
            x: 0.0,
            y: 0.0,
            w: 1000.0,
            h: 600.0,
        };
        let large = fit(bounds, space, (6000, 3000)).unwrap();
        assert_eq!(large.w / large.h, 2.0);
        assert!(large.w <= 800.0 && large.h <= 480.0);
        let small = space
            .physical(fit(bounds, space, (30, 15)).unwrap())
            .unwrap();
        assert_eq!((small.width, small.height), (30, 15));
        assert!(small.x < 0 && small.y > -200);
    }
    #[test]
    fn projection_placement_rejects_empty_invalid_and_unrepresentable_geometry() {
        let bounds = Rect {
            x: 0.0,
            y: 0.0,
            w: 1000.0,
            h: 600.0,
        };
        for scale in [0.0, -1.0, f64::INFINITY, f64::NAN] {
            assert!(fit(
                bounds,
                Space {
                    origin: (0, 0),
                    scale
                },
                (10, 10)
            )
            .is_err());
        }
        assert!(fit(
            bounds,
            Space {
                origin: (0, 0),
                scale: 1.0
            },
            (0, 10)
        )
        .is_err());
    }
}
