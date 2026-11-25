//! A complete preview for one wheel transition; the host retains geometry/focus identity, but an empty region prevents old GPU frames from appearing.
use super::{PinRect, State};
use crate::desktop::{canvas::Canvas, display_snapshot::Stamp};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use windows::{
    core::Result,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::{
            HiDpi::GetDpiForWindow, Input::KeyboardAndMouse::GetDoubleClickTime,
            WindowsAndMessaging::*,
        },
    },
};
#[path = "pin_zoom_preview_window.rs"]
mod window;

#[derive(Clone, Copy)]
pub(super) struct Click {
    msg: u32,
    point: POINT,
    at: std::time::Instant,
}
// When the first click hits the preview and the second the host, preserve system double-click time and space boundaries.
pub(in crate::desktop) unsafe fn click_dispatch(
    hwnd: HWND,
    state: &State,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    if !matches!(
        msg,
        WM_LBUTTONDOWN | WM_NCLBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN
    ) {
        return None;
    }
    let click = state.pin_animation.click.take()?;
    let mut point = POINT {
        x: lp.0 as i16 as i32,
        y: (lp.0 >> 16) as i16 as i32,
    };
    if msg != WM_NCLBUTTONDOWN && !ClientToScreen(hwnd, &mut point).as_bool() {
        return None;
    }
    if click.msg != msg
        || click.at.elapsed().as_millis() > u128::from(GetDoubleClickTime())
        || (i64::from(point.x) - i64::from(click.point.x)).abs()
            > i64::from(GetSystemMetrics(SM_CXDOUBLECLK) / 2)
        || (i64::from(point.y) - i64::from(click.point.y)).abs()
            > i64::from(GetSystemMetrics(SM_CYDOUBLECLK) / 2)
    {
        return None;
    }
    let double = match msg {
        WM_LBUTTONDOWN => WM_LBUTTONDBLCLK,
        WM_NCLBUTTONDOWN => WM_NCLBUTTONDBLCLK,
        WM_RBUTTONDOWN => WM_RBUTTONDBLCLK,
        _ => WM_MBUTTONDBLCLK,
    };
    Some(SendMessageW(hwnd, double, Some(wp), Some(lp)))
}

pub(super) struct Preview {
    window: window::Window,
    canvas: RefCell<Canvas>,
    stamp: RefCell<Stamp>,
    padding: Cell<u32>,
}
pub(in crate::desktop) fn active(state: &State) -> bool {
    state.pin_animation.preview.borrow().is_some()
}
pub(super) fn shared(state: &State) -> bool {
    state
        .pin_animation
        .preview
        .borrow()
        .as_ref()
        .is_some_and(|p| p.window.shared())
}
pub(in crate::desktop) fn handle(state: &State) -> Option<HWND> {
    state
        .pin_animation
        .preview
        .borrow()
        .as_ref()
        .map(|p| p.window.hwnd)
}
pub(in crate::desktop) fn fresh(state: &State) -> bool {
    state
        .pin_animation
        .preview
        .borrow()
        .as_ref()
        .is_none_or(|p| {
            p.stamp.borrow().matches(
                &state.canvas.borrow(),
                state.resolved_display.borrow().as_ref(),
            )
        })
}
pub(super) unsafe fn begin(hwnd: HWND, state: &State, rect: PinRect) -> Result<()> {
    if active(state) {
        return Ok(());
    }
    let (canvas, stamp) = {
        let source = state.canvas.borrow();
        let frame = state.resolved_display.borrow();
        let canvas = source.zoom_snapshot(frame.as_ref())?.ok_or_else(|| {
            windows::core::Error::new(E_FAIL, "Zoom preview requires a complete document")
        })?;
        (canvas, Stamp::capture(&source, frame.as_ref()))
    };
    let preview = Rc::new(Preview {
        window: window::Window::new(hwnd, state)?,
        canvas: RefCell::new(canvas),
        stamp: RefCell::new(stamp),
        padding: Cell::new(0),
    });
    // Finish pixels, opacity, and border in memory first; do not cover a translucent host before spending time preparing the mask.
    let frame = preview.prepare(hwnd, state, rect)?;
    if !state.lifetime.owns(hwnd) {
        return Ok(());
    }
    *state.pin_animation.preview.borrow_mut() = Some(preview.clone());
    if let Err(error) = mask(hwnd).and_then(|_| preview.submit(frame, rect)) {
        finish(hwnd, state, true);
        return Err(error);
    }
    if let Err(error) = SetWindowPos(
        preview.window.hwnd,
        None,
        0,
        0,
        0,
        0,
        SWP_SHOWWINDOW
            | SWP_NOZORDER
            | SWP_NOOWNERZORDER
            | SWP_NOACTIVATE
            | SWP_NOMOVE
            | SWP_NOSIZE,
    ) {
        finish(hwnd, state, true);
        return Err(error);
    }
    crate::desktop::application::chrome::composite_frame(&state.chrome, hwnd, true);
    Ok(())
}
impl Preview {
    unsafe fn present(&self, hwnd: HWND, state: &State, rect: PinRect) -> Result<()> {
        self.submit(self.prepare(hwnd, state, rect)?, rect)
    }
    unsafe fn prepare(
        &self,
        hwnd: HWND,
        state: &State,
        rect: PinRect,
    ) -> Result<(crate::desktop::software_surface::Submission, u32)> {
        self.window.sync_affinity(hwnd)?;
        self.window.sync_order(hwnd)?;
        let dpi = GetDpiForWindow(hwnd).clamp(96, 768);
        // The active preview's image and border share one HWND/bitmap; no other window may chase them.
        let border = crate::desktop::application::chrome::border_visible(&state.chrome, hwnd);
        let padding = if border {
            crate::desktop::pin_border_style::width(dpi)
        } else {
            0
        };
        let pending = {
            let mut canvas = self.canvas.borrow_mut();
            canvas.viewport = state.canvas.borrow().viewport;
            canvas.paint_display(
                self.window.hwnd,
                rect.width,
                rect.height,
                dpi,
                state.resolved_display.borrow().as_ref(),
            )?;
            canvas
                .take_submission(false)
                .expect("preview owns a software target")
        };
        // The Canvas borrow is released; reentrant ULW callbacks hold only stable HWND+identity tokens.
        Ok((pending.prepare_preview(padding, border)?, padding))
    }
    unsafe fn submit(
        &self,
        frame: (crate::desktop::software_surface::Submission, u32),
        rect: PinRect,
    ) -> Result<()> {
        let (pending, padding) = frame;
        pending.present_at(
            self.window.hwnd,
            Some(POINT {
                x: rect.x.saturating_sub(padding as i32),
                y: rect.y.saturating_sub(padding as i32),
            }),
        )?;
        self.padding.set(padding);
        Ok(())
    }
}
pub(super) unsafe fn refresh_snapshot(hwnd: HWND, state: &State, rect: PinRect) -> Result<()> {
    let preview = state.pin_animation.preview.borrow().clone();
    let Some(preview) = preview else {
        return Ok(());
    };
    let (canvas, stamp) = {
        let source = state.canvas.borrow();
        let frame = state.resolved_display.borrow();
        let canvas = source.zoom_snapshot(frame.as_ref())?.ok_or_else(|| {
            windows::core::Error::new(E_FAIL, "Stationary pin requires a complete document")
        })?;
        (canvas, Stamp::capture(&source, frame.as_ref()))
    };
    let old = preview.canvas.replace(canvas);
    if let Err(error) = preview.present(hwnd, state, rect) {
        preview.canvas.replace(old);
        return Err(error);
    }
    *preview.stamp.borrow_mut() = stamp;
    Ok(())
}
pub(super) unsafe fn move_cached(hwnd: HWND, state: &State, rect: PinRect) -> Result<()> {
    let preview = state.pin_animation.preview.borrow().clone();
    if let Some(preview) = preview {
        preview.window.sync_order(hwnd)?;
        let dpi = GetDpiForWindow(hwnd).clamp(96, 768);
        let border = crate::desktop::application::chrome::border_visible(&state.chrome, hwnd);
        let expected_padding = if border {
            crate::desktop::pin_border_style::width(dpi)
        } else {
            0
        };
        let mut current = RECT::default();
        GetWindowRect(preview.window.hwnd, &mut current)?;
        if preview.padding.get() != expected_padding
            || current.right - current.left != (rect.width + 2 * expected_padding) as i32
            || current.bottom - current.top != (rect.height + 2 * expected_padding) as i32
        {
            // Cross-DPI/system resize is not pure movement; rebuild a complete frame at the corresponding size.
            super::fit_size(hwnd, state, (rect.width as i32, rect.height as i32));
            return preview.present(hwnd, state, rect);
        }
        let padding = preview.padding.get() as i32;
        let attached = crate::desktop::tool_panel::moving_positions(
            hwnd,
            state,
            RECT {
                left: rect.x,
                top: rect.y,
                right: rect.x + rect.width as i32,
                bottom: rect.y + rect.height as i32,
            },
        )?;
        // Move only the completed image/border frame, without redrawing/copying images or queuing historical-position catch-up.
        crate::desktop::application::chrome::move_frame(
            &state.chrome,
            hwnd,
            preview.window.hwnd,
            rect.x - padding,
            rect.y - padding,
            &attached,
        )?;
    }
    Ok(())
}
pub(super) unsafe fn present(hwnd: HWND, state: &State, rect: PinRect) -> Result<()> {
    let preview = state.pin_animation.preview.borrow().clone();
    if let Some(preview) = preview {
        preview.present(hwnd, state, rect)?;
    }
    Ok(())
}
pub(in crate::desktop) unsafe fn mask(hwnd: HWND) -> Result<()> {
    let region = CreateRectRgn(0, 0, 0, 0);
    if region.is_invalid() {
        return Err(windows::core::Error::from_win32());
    }
    if SetWindowRgn(hwnd, Some(region), false) == 0 {
        let _ = DeleteObject(region.into());
        return Err(windows::core::Error::from_win32());
    }
    Ok(())
}
pub(in crate::desktop) unsafe fn hit_update(hwnd: HWND, state: &State) -> Result<()> {
    if active(state) {
        mask(hwnd)
    } else {
        crate::desktop::hit_region::update_visible(hwnd, state)
    }
}
pub(super) unsafe fn finish(hwnd: HWND, state: &State, restore: bool) {
    // Move out of the slot and release the mask fence first; the local Rc keeps the preview until the host is fully repainted.
    let preview = state.pin_animation.preview.borrow_mut().take();
    let Some(preview) = preview else {
        return;
    };
    if restore && state.lifetime.owns(hwnd) {
        // Finish the final native frame while the region is still empty; do not expose an old-size GPU buffer first.
        let mut rect = RECT::default();
        if let Err(error) = GetWindowRect(hwnd, &mut rect) {
            *state.error.borrow_mut() = Some(error);
            *state.pin_animation.preview.borrow_mut() = Some(preview);
            return;
        }
        if state.pin_resize.rect.get().is_none() {
            state.pin_resize.rect.set(Some(PinRect {
                x: rect.left,
                y: rect.top,
                width: (rect.right - rect.left).max(1) as u32,
                height: (rect.bottom - rect.top).max(1) as u32,
            }));
            crate::desktop::procedure::resize_frame(hwnd, state);
            state.pin_resize.rect.set(None);
            let prepared = state.pin_resize.prepared.replace(false);
            if !prepared && state.lifetime.owns(hwnd) && IsWindowVisible(hwnd).as_bool() {
                // On failure retain the last complete frame and input entry; later clicks/cancellation may retry handoff, and destruction still reclaims resources.
                // Publish no unprepared old GPU frame and add no automatic retry timer.
                if state.error.borrow().is_none() {
                    *state.error.borrow_mut() = Some(windows::core::Error::new(
                        E_FAIL,
                        "Zoom handoff did not prepare the native frame",
                    ));
                }
                *state.pin_animation.preview.borrow_mut() = Some(preview);
                return;
            }
            // The host still has no visible pixels while waiting, preventing double blending of translucent images across two layers.
            if let Err(error) = windows::Win32::Graphics::Dwm::DwmFlush() {
                eprintln!("native-zoom-handoff: {error}");
            }
            if state.lifetime.owns(hwnd) && prepared {
                let _ = ValidateRect(Some(hwnd), None);
            }
        }
        if !state.lifetime.owns(hwnd) {
            return;
        }
        if let Err(error) = crate::desktop::application::chrome::prepare_frame(&state.chrome, hwnd)
        {
            *state.error.borrow_mut() = Some(error);
            *state.pin_animation.preview.borrow_mut() = Some(preview);
            return;
        }
        *state.hit_region.borrow_mut() = Default::default();
        if let Err(error) = crate::desktop::hit_region::update_visible(hwnd, state) {
            if let Err(mask_error) = mask(hwnd) {
                eprintln!("native-preview-handoff-mask: {mask_error}");
            }
            *state.error.borrow_mut() = Some(error);
            *state.pin_animation.preview.borrow_mut() = Some(preview);
            return;
        }
        if let Err(error) = crate::desktop::application::chrome::commit_frame(&state.chrome, hwnd) {
            if let Err(mask_error) = mask(hwnd) {
                eprintln!("native-preview-handoff-mask: {mask_error}");
            }
            *state.error.borrow_mut() = Some(error);
            *state.pin_animation.preview.borrow_mut() = Some(preview);
            return;
        }
    }
    // Remove the proxy immediately after restoring the host, without intervening border repaint/reordering or another DWM-frame wait.
    // Otherwise translucent content and two borders briefly overlap, causing a brightness pulse after input stops.
    drop(preview);
    crate::desktop::application::chrome::composite_frame(&state.chrome, hwnd, false);
}
