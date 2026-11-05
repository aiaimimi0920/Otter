//! 钉图缩放只改变窗口和视口，不重新解码源图或写入编辑历史。
use super::State;
use crate::{
    pin_geometry::{zoom, PinRect},
    viewport::Point,
};
use windows::Win32::{
    Foundation::*,
    Graphics::Gdi::InvalidateRect,
    UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
};
#[path = "pin_minified.rs"]
mod minified;
pub(super) use minified::restore as restore_minified;
#[path = "pin_minify.rs"]
mod minify;
pub(super) use minify::dispatch as dispatch_minified;
#[path = "pin_minified_input.rs"]
mod minified_input;
#[path = "pin_toggle_guard.rs"]
mod toggle_guard;
pub(super) use minified_input::dispatch as dispatch_minified_input;

pub unsafe fn wheel(hwnd: HWND, state: &State, wp: WPARAM, lp: LPARAM) {
    let mut rect = RECT::default();
    if GetWindowRect(hwnd, &mut rect).is_err() {
        return;
    }
    let before = PinRect {
        x: rect.left,
        y: rect.top,
        width: (rect.right - rect.left).max(1) as u32,
        height: (rect.bottom - rect.top).max(1) as u32,
    };
    let pointer = (lp.0 as i16 as i32, (lp.0 >> 16) as i16 as i32);
    let Some(after) = zoom(before, pointer, (wp.0 >> 16) as i16 as f32 / 120.0) else {
        return;
    };
    if after == before {
        return;
    }
    if SetWindowPos(
        hwnd,
        None,
        after.x,
        after.y,
        after.width as i32,
        after.height as i32,
        SWP_NOZORDER | SWP_NOACTIVATE,
    )
    .is_ok()
    {
        fit(hwnd, state);
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}

pub unsafe fn fit(hwnd: HWND, state: &State) {
    let mut rect = RECT::default();
    if GetClientRect(hwnd, &mut rect).is_err() {
        return;
    }
    let mut canvas = state.canvas.borrow_mut();
    let display = state.resolved_display.borrow();
    let image = display
        .as_ref()
        .map(|frame| &frame.image)
        .or(canvas.image.as_ref());
    if let Some(image) = image {
        let dpi = GetDpiForWindow(hwnd).max(1) as f32;
        let crop = display.as_ref().map_or_else(
            || canvas.editor.annotation_frame(),
            |frame| {
                frame
                    .editor
                    .as_deref()
                    .and_then(|editor| editor.annotation_frame())
            },
        );
        let (x, y, width, height) = crop
            .map_or((0.0, 0.0, image.width as f32, image.height as f32), |r| {
                (r.x as f32, r.y as f32, r.width as f32, r.height as f32)
            });
        canvas.viewport = fit_frame(
            (
                (rect.right - rect.left) as f32,
                (rect.bottom - rect.top) as f32,
            ),
            (x, y, width, height),
            dpi,
        );
    }
}

fn fit_frame(
    client: (f32, f32),
    frame: (f32, f32, f32, f32),
    dpi: f32,
) -> crate::viewport::Viewport {
    let (x, y, width, height) = frame;
    let scale = ((client.0 / width).min(client.1 / height) * 96.0 / dpi).max(0.0001);
    crate::viewport::Viewport {
        scale,
        origin: Point {
            x: -x * scale,
            y: -y * scale,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fractional_frame_fits_physical_client_without_using_ceil_buffer_extent() {
        for dpi in [96.0, 144.0, 192.0] {
            for (x, y) in [(0.0, 0.0), (1.25, 0.5)] {
                let view = fit_frame((725.0, 325.0), (x, y, 7.25, 3.25), dpi);
                let factor = dpi / 96.0;
                assert!(((view.origin.x + x * view.scale) * factor).abs() < 0.001);
                assert!(((view.origin.y + y * view.scale) * factor).abs() < 0.001);
                assert!(((view.origin.x + (x + 7.25) * view.scale) * factor - 725.0).abs() < 0.001);
                assert!(((view.origin.y + (y + 3.25) * view.scale) * factor - 325.0).abs() < 0.001);
            }
        }
    }
}
