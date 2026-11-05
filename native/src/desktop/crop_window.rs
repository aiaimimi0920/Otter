//! 将已提交的源裁剪框同步到钉图物理窗口；Win32 调用前释放 Canvas 借用。
use super::State;
use crate::{crop_geometry::Rect, pin_geometry::PinRect};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
    },
};

fn placement(window: PinRect, old: Rect, new: Rect, scale: f64) -> Option<PinRect> {
    if !scale.is_finite()
        || scale <= 0.0
        || !new.width.is_finite()
        || !new.height.is_finite()
        || new.width <= 0.0
        || new.height <= 0.0
    {
        return None;
    }
    // 恢复完整源图可能超过当前缩放下的交换链预算，必要时降低显示比例。
    let scale = scale
        .min(8192.0 / new.width.max(new.height))
        .min((16_777_216.0 / (new.width * new.height)).sqrt());
    let width = (new.width * scale).floor().max(1.0) as u32;
    let height = (new.height * scale).floor().max(1.0) as u32;
    let x = window.x as f64 + (new.x - old.x) * scale;
    let y = window.y as f64 + (new.y - old.y) * scale;
    if !x.is_finite()
        || !y.is_finite()
        || x < i32::MIN as f64
        || y < i32::MIN as f64
        || x + width as f64 > i32::MAX as f64
        || y + height as f64 > i32::MAX as f64
    {
        return None;
    }
    Some(PinRect {
        x: x.round() as i32,
        y: y.round() as i32,
        width,
        height,
    })
}

pub(super) unsafe fn sync(hwnd: HWND, state: &State) -> Result<()> {
    let prepared = {
        let canvas = state.canvas.borrow();
        if !canvas.pinned {
            return Ok(());
        }
        let Some(image) = &canvas.image else {
            return Ok(());
        };
        let full = Rect {
            x: 0.0,
            y: 0.0,
            width: image.width as f64,
            height: image.height as f64,
        };
        let new = canvas.editor.annotation_frame();
        if new == canvas.applied_crop {
            return Ok(());
        }
        let mut window = RECT::default();
        GetWindowRect(hwnd, &mut window)?;
        let next = placement(
            PinRect {
                x: window.left,
                y: window.top,
                width: (window.right - window.left).max(1) as u32,
                height: (window.bottom - window.top).max(1) as u32,
            },
            canvas.applied_crop.unwrap_or(full),
            new.unwrap_or(full),
            canvas.viewport.scale as f64 * GetDpiForWindow(hwnd).max(1) as f64 / 96.0,
        )
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid cropped window placement"))?;
        (next, new)
    };
    let (next, crop) = prepared;
    SetWindowPos(
        hwnd,
        None,
        next.x,
        next.y,
        next.width as i32,
        next.height as i32,
        SWP_NOZORDER | SWP_NOACTIVATE,
    )?;
    state.canvas.borrow_mut().applied_crop = crop;
    super::pin_window::fit(hwnd, state);
    super::hit_region::update(hwnd, state)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }
    #[test]
    fn crop_reset_and_repeat_keep_negative_monitor_source_anchor() {
        let full = rect(0.0, 0.0, 400.0, 200.0);
        let crop = rect(100.0, 50.0, 200.0, 100.0);
        for scale in [0.5, 1.0, 1.5, 2.0] {
            let original = PinRect {
                x: -1000,
                y: -200,
                width: (400.0 * scale) as u32,
                height: (200.0 * scale) as u32,
            };
            let first = placement(original, full, crop, scale).unwrap();
            assert_eq!(first.x, original.x + (100.0 * scale) as i32);
            assert_eq!(first.y, original.y + (50.0 * scale) as i32);
            assert_eq!(placement(first, crop, full, scale), Some(original));
            let next = rect(120.0, 60.0, 100.0, 50.0);
            let second = placement(first, crop, next, scale).unwrap();
            assert_eq!(placement(second, next, full, scale), Some(original));
        }
    }
    #[test]
    fn reset_large_image_obeys_swapchain_budget_and_invalid_coordinates_reject() {
        let window = PinRect {
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        };
        let small = rect(500.0, 200.0, 400.0, 300.0);
        let large = rect(0.0, 0.0, 16000.0, 12000.0);
        let restored = placement(window, small, large, 2.0).unwrap();
        assert!(restored.width <= 8192 && restored.height <= 8192);
        assert!(u64::from(restored.width) * u64::from(restored.height) <= 16_777_216);
        assert!(placement(window, small, large, f64::NAN).is_none());
        assert!(placement(window, small, rect(f64::INFINITY, 0.0, 20.0, 20.0), 1.0).is_none());
    }
}
