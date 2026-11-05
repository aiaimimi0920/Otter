//! 双击缩略只改变宿主与呈现，文档/历史保持不变；原图像素共享，不烘焙标注。
use super::*;
use crate::{
    crop_geometry::Size,
    session_minified::{View, Views},
};
use windows::core::{Error, Result};
use windows::Win32::Graphics::Gdi::ClientToScreen;

pub(in crate::desktop) unsafe fn dispatch(
    hwnd: HWND,
    state: &State,
    msg: u32,
    lp: LPARAM,
) -> Option<LRESULT> {
    if !matches!(msg, WM_NCLBUTTONDBLCLK | WM_LBUTTONDBLCLK) {
        return None;
    }
    let result = if state.canvas.borrow().minified.is_some() {
        super::restore_minified(hwnd, state)
    } else {
        if !state.canvas.borrow().pinned || state.canvas.borrow().tool != crate::editor::Tool::Pan {
            return None;
        }
        let mut point = POINT {
            x: lp.0 as i16 as i32,
            y: (lp.0 >> 16) as i16 as i32,
        };
        if msg == WM_LBUTTONDBLCLK && !ClientToScreen(hwnd, &mut point).as_bool() {
            return Some(LRESULT(0));
        }
        collapse(hwnd, state, (point.x, point.y))
    };
    if let Err(error) = result {
        eprintln!("native-minified-toggle: {error}");
    }
    Some(LRESULT(0))
}

pub(in crate::desktop) unsafe fn collapse(
    hwnd: HWND,
    state: &State,
    screen: (i32, i32),
) -> Result<bool> {
    if !super::toggle_guard::allows(state) {
        return Ok(false);
    }
    let mut rect = RECT::default();
    GetWindowRect(hwnd, &mut rect)?;
    let before = PinRect {
        x: rect.left,
        y: rect.top,
        width: (rect.right - rect.left).max(1) as u32,
        height: (rect.bottom - rect.top).max(1) as u32,
    };
    let dpi = GetDpiForWindow(hwnd).max(1) as f64 / 96.0;
    let (after, size, offset) = geometry(before, screen, dpi)?;
    let opacity = {
        let mut canvas = state.canvas.borrow_mut();
        if canvas.minified.is_some() || canvas.tool != crate::editor::Tool::Pan {
            return Ok(false);
        }
        canvas.sync_raster_source();
        let image = canvas
            .minified_source
            .as_ref()
            .filter(|source| source.matches(&canvas.editor))
            .map(|source| source.image())
            .or(canvas.image.as_ref())
            .cloned();
        let annotation = View { size, offset };
        let image_view = if let Some(crop) = canvas.editor.crop_view() {
            let sx = size.width / crop.source.rect.width;
            let sy = size.height / crop.source.rect.height;
            View {
                size: Size {
                    width: crop.source.source.width * sx,
                    height: crop.source.source.height * sy,
                },
                offset: (
                    crop.source.rect.x * sx + offset.0,
                    crop.source.rect.y * sy + offset.1,
                ),
            }
        } else if let Some(image) = &image {
            let factor = (size.width / image.width as f64).min(size.height / image.height as f64);
            let fitted = Size {
                width: image.width as f64 * factor,
                height: image.height as f64 * factor,
            };
            View {
                size: fitted,
                offset: (
                    (offset.0 - (size.width - fitted.width) / 2.0)
                        .clamp(0.0, (fitted.width - 100.0).max(0.0)),
                    (offset.1 - (size.height - fitted.height) / 2.0)
                        .clamp(0.0, (fitted.height - 100.0).max(0.0)),
                ),
            }
        } else {
            annotation
        };
        let presentation = super::super::canvas::minified::Presentation::restored(
            image,
            Views {
                image: image_view,
                annotations: annotation,
            },
            Size {
                width: 100.0,
                height: 100.0,
            },
            &canvas.editor,
        )?;
        let opacity = canvas.window_opacity;
        canvas.minified = Some(presentation);
        canvas.minified_restore = Some(crate::minified_transition::Transition {
            initial: after,
            expanded: before,
            opacity,
            persist_expanded: true,
        });
        canvas.window_opacity = canvas.minified_opacity;
        opacity
    };
    if let Err(error) = SetWindowPos(
        hwnd,
        None,
        after.x,
        after.y,
        after.width as i32,
        after.height as i32,
        SWP_NOZORDER | SWP_NOACTIVATE,
    ) {
        let mut canvas = state.canvas.borrow_mut();
        canvas.minified = None;
        canvas.minified_restore = None;
        canvas.window_opacity = opacity;
        return Err(error);
    }
    super::super::hit_region::update(hwnd, state)?;
    let _ = InvalidateRect(Some(hwnd), None, false);
    Ok(true)
}

fn geometry(
    frame: PinRect,
    pointer: (i32, i32),
    scale: f64,
) -> Result<(PinRect, Size, (f64, f64))> {
    if !scale.is_finite() || scale <= 0.0 || frame.width == 0 || frame.height == 0 {
        return Err(Error::new(E_INVALIDARG, "Invalid minified window scale"));
    }
    let size = Size {
        width: frame.width as f64 / scale,
        height: frame.height as f64 / scale,
    };
    let offset = |position: i32, origin: i32, length: f64| {
        (((position as f64 - origin as f64) / scale).clamp(0.0, length) - 50.0)
            .clamp((length - 100.0).min(0.0), (length - 100.0).max(0.0))
    };
    let offset = (
        offset(pointer.0, frame.x, size.width),
        offset(pointer.1, frame.y, size.height),
    );
    let x = frame.x as f64 + (offset.0 * scale + 0.5).floor();
    let y = frame.y as f64 + (offset.1 * scale + 0.5).floor();
    let side = (100.0 * scale + 0.5).floor();
    if !(1.0..=8192.0).contains(&side)
        || side * side > 16_000_000.0
        || x < i32::MIN as f64
        || y < i32::MIN as f64
        || x + side > i32::MAX as f64
        || y + side > i32::MAX as f64
    {
        return Err(Error::new(
            E_INVALIDARG,
            "Minified window geometry overflow",
        ));
    }
    Ok((
        PinRect {
            x: x as i32,
            y: y as i32,
            width: side as u32,
            height: side as u32,
        },
        size,
        offset,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn small_image_uses_negative_offset_and_physical_rounding() {
        let frame = PinRect {
            x: -120,
            y: -60,
            width: 60,
            height: 30,
        };
        let (mini, _, offset) = geometry(frame, (-90, -45), 1.5).unwrap();
        assert_eq!(offset, (-30.0, -40.0));
        assert_eq!(
            mini,
            PinRect {
                x: -165,
                y: -120,
                width: 150,
                height: 150
            }
        );
        assert!(geometry(frame, (0, 0), f64::NAN).is_err());
        assert!(geometry(
            PinRect {
                x: i32::MAX,
                ..frame
            },
            (i32::MAX, 0),
            1.0
        )
        .is_err());
    }
}
