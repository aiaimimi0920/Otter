//! Pin zoom and opacity change only window presentation, without re-decoding source images or writing edit history.
use super::State;
use crate::{pin_geometry::PinRect, viewport::Point};
use windows::Win32::{
    Foundation::*,
    Graphics::Gdi::InvalidateRect,
    UI::{HiDpi::GetDpiForWindow, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
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
#[path = "pin_zoom_animation.rs"]
pub(super) mod animation;
#[path = "pin_move_escape.rs"]
mod move_escape;
#[path = "pin_move_preview.rs"]
pub(super) mod movement;
#[path = "pin_zoom_present.rs"]
pub(super) mod presentation;
#[path = "pin_zoom_preview.rs"]
pub(super) mod preview;
#[path = "pin_zoom_stationary.rs"]
pub(super) mod stationary;
pub use animation::wheel;

pub(super) unsafe fn close_on_escape(hwnd: HWND, state: &State, lp: LPARAM) -> bool {
    let close = can_close_on_escape(state, lp);
    if close {
        // Release the model borrow first, then reclaim the host, panel, and application node through the existing WM_CLOSE path.
        SendMessageW(hwnd, WM_CLOSE, None, None);
    }
    close
}

pub(super) unsafe fn can_close_on_escape(state: &State, lp: LPARAM) -> bool {
    // Held/combined keys still only cancel operations; cancelling a draft must not continue deleting other pins.
    if lp.0 & (1 << 30) != 0
        || [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN]
            .iter()
            .any(|key| GetKeyState(key.0 as i32) < 0)
        || state.group_locked.get()
        || state.group_hidden.get()
        || !super::capture_input::exclusive_available(state)
        || super::group_lock::editing_busy(state)
        || state.copy_job.borrow().is_some()
        || state.export.borrow().is_some()
    {
        return false;
    }
    {
        let canvas = state.canvas.borrow();
        canvas.pinned
            && !canvas.crop_mode
            && !canvas.blocks_wheel()
            && canvas.drag.is_none()
            && canvas.scale_pending.is_none()
            && canvas.crop_pending.is_none()
    }
}

pub(super) unsafe fn opacity_wheel(hwnd: HWND, state: &State, wp: WPARAM) {
    let delta = (wp.0 >> 16) as i16;
    let changed = {
        let mut canvas = state.canvas.borrow_mut();
        // Ordinary and minimized views save opacity separately; minimized adjustments must not overwrite the expansion restore value.
        let opacity = if canvas.minified.is_some() {
            &mut canvas.minified_opacity
        } else {
            &mut canvas.normal_opacity
        };
        let next = (*opacity + f32::from(delta) * 0.001).clamp(0.0, 1.0);
        let changed = next != *opacity;
        *opacity = next;
        canvas.window_opacity = next;
        changed
    };
    if changed {
        // Reuse WM_PAINT's GPU/CPU opacity commit; never hold a Canvas borrow across Win32 calls.
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}

pub unsafe fn fit(hwnd: HWND, state: &State) {
    let mut rect = RECT::default();
    if GetClientRect(hwnd, &mut rect).is_err() {
        return;
    }
    fit_size(
        hwnd,
        state,
        (rect.right - rect.left, rect.bottom - rect.top),
    );
}

unsafe fn fit_size(hwnd: HWND, state: &State, size: (i32, i32)) {
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
        canvas.viewport = fit_frame((size.0 as f32, size.1 as f32), (x, y, width, height), dpi);
    }
}

fn fit_frame(
    client: (f32, f32),
    frame: (f32, f32, f32, f32),
    dpi: f32,
) -> crate::viewport::Viewport {
    let (x, y, width, height) = frame;
    let scale = ((client.0 / width).min(client.1 / height) * 96.0 / dpi).max(0.0001);
    // The client area uses physical pixels and the viewport logical pixels; split padding evenly before offsetting the crop origin.
    let logical = (client.0 * 96.0 / dpi, client.1 * 96.0 / dpi);
    crate::viewport::Viewport {
        scale,
        origin: Point {
            x: (logical.0 - width * scale) * 0.5 - x * scale,
            y: (logical.1 - height * scale) * 0.5 - y * scale,
        },
    }
}

#[cfg(test)]
#[path = "pin_escape_tests.rs"]
mod escape_tests;

#[cfg(test)]
#[path = "pin_wheel_tests.rs"]
mod wheel_tests;

#[cfg(test)]
#[path = "pin_zoom_tests.rs"]
mod zoom_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fitted_content_centers_horizontal_and_vertical_letterboxing_at_each_dpi() {
        for dpi in [96.0, 120.0, 144.0, 192.0] {
            for (client, expected) in [
                ((200.0, 100.0), (50.0, 0.0, 150.0, 100.0)),
                ((100.0, 200.0), (0.0, 50.0, 100.0, 150.0)),
                ((100.0, 100.0), (0.0, 0.0, 100.0, 100.0)),
            ] {
                assert_frame(client, (0.0, 0.0, 100.0, 100.0), dpi, expected);
            }
        }
    }
    #[test]
    fn cropped_fractional_and_non_square_frames_center_without_changing_mapping() {
        for dpi in [96.0, 120.0, 144.0, 192.0] {
            assert_frame(
                (200.0, 200.0),
                (12.5, 7.25, 100.0, 50.0),
                dpi,
                (0.0, 50.0, 200.0, 150.0),
            );
            assert_frame(
                (200.0, 100.0),
                (1.25, 0.5, 3.25, 3.25),
                dpi,
                (50.0, 0.0, 150.0, 100.0),
            );
        }
    }
    fn assert_frame(
        client: (f32, f32),
        frame: (f32, f32, f32, f32),
        dpi: f32,
        expected: (f32, f32, f32, f32),
    ) {
        let view = fit_frame(client, frame, dpi);
        let factor = dpi / 96.0;
        let (x, y, width, height) = frame;
        for (actual, expected) in [
            ((view.origin.x + x * view.scale) * factor, expected.0),
            ((view.origin.y + y * view.scale) * factor, expected.1),
            (
                (view.origin.x + (x + width) * view.scale) * factor,
                expected.2,
            ),
            (
                (view.origin.y + (y + height) * view.scale) * factor,
                expected.3,
            ),
        ] {
            assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
        }
        let center = view.image_point(Point {
            x: client.0 / factor * 0.5,
            y: client.1 / factor * 0.5,
        });
        assert!((center.x - x - width * 0.5).abs() < 0.001);
        assert!((center.y - y - height * 0.5).abs() < 0.001);
    }
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
