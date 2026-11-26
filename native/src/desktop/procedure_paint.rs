//! WM_PAINT owner: stage model drawing, borrow-free software submission, identity revalidation, and existing error policy.
use super::{paint_scope::PaintScope, State};
use windows::{
    core::Result,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
    },
};

struct Cycle<'a> {
    state: &'a State,
    hwnd: HWND,
}
impl Drop for Cycle<'_> {
    fn drop(&mut self) {
        self.state.render_active.set(false);
        if self.state.render_deferred.replace(false) && self.state.lifetime.owns(self.hwnd) {
            unsafe {
                let _ = InvalidateRect(Some(self.hwnd), None, false);
            }
        }
    }
}
unsafe fn draw(hwnd: HWND, state: &State) -> Result<Option<super::super::canvas::Pending>> {
    let mut rect = RECT::default();
    GetClientRect(hwnd, &mut rect)?;
    let (width, height) = state.pin_resize.rect.get().map_or(
        (
            (rect.right - rect.left).max(0) as u32,
            (rect.bottom - rect.top).max(0) as u32,
        ),
        |next| (next.width, next.height),
    );
    let dpi = GetDpiForWindow(hwnd);
    let display = state.resolved_display.borrow();
    let mut canvas = state.canvas.borrow_mut();
    let normal = display.is_none() && canvas.minified.is_none();
    if display.is_some() {
        canvas.paint_display(hwnd, width, height, dpi, display.as_ref())?;
    } else {
        canvas.paint(hwnd, width, height, dpi)?;
    }
    Ok(if width > 0 && height > 0 {
        canvas.take_submission(normal)
    } else {
        None
    })
}
unsafe fn frame(hwnd: HWND, state: &State) -> Result<bool> {
    let owns = || state.lifetime.owns(hwnd);
    let allow_restore = || !state.group_hidden.get() && !super::super::tray::is_hidden(state);
    let mut removed_layered = false;
    if state.canvas.borrow().needs_gpu_style() {
        // Protected by render_active with no Canvas borrow; style writes may reenter source replacement/destruction.
        // End the style transaction before a new GPU might attach; attached failure must never trigger layered rollback.
        let style =
            super::super::software_surface::window_style::Transition::gpu_guarded(hwnd, &owns)?;
        removed_layered = style.removes_layered();
        style.commit()?;
    }
    let mut visibility = super::super::render_window_visibility::Reset::begin(
        hwnd,
        removed_layered,
        &owns,
        &allow_restore,
    )?;
    let result = draw_frame(hwnd, state);
    let restored = visibility.restore();
    match (result, restored) {
        (Err(error), Err(restore)) => Err(windows::core::Error::new(
            error.code(),
            format!("{error}; render visibility restore failed: {restore}"),
        )),
        (Err(error), _) => Err(error),
        (_, Err(error)) => Err(error),
        (Ok(value), Ok(())) => Ok(value),
    }
}
unsafe fn draw_frame(hwnd: HWND, state: &State) -> Result<bool> {
    // All Canvas and resolved_display RefCell borrows have ended when draw returns.
    if let Some(pending) = draw(hwnd, state)? {
        if !super::software_present::submit(hwnd, state, pending)? {
            return Ok(false);
        }
    }
    if !state.lifetime.owns(hwnd) {
        return Ok(false);
    }
    if state.pin_resize.rect.get().is_none() {
        super::super::hit_region::update(hwnd, state)?;
    }
    Ok(true)
}
pub(super) unsafe fn paint(hwnd: HWND, state: &State) -> LRESULT {
    execute(hwnd, state, true)
}
pub(in crate::desktop) unsafe fn resize_frame(hwnd: HWND, state: &State) {
    execute(hwnd, state, false);
}
pub(super) unsafe fn defer_nested(hwnd: HWND, state: &State, validate: bool) -> LRESULT {
    state.render_deferred.set(true);
    drop(validate.then(|| PaintScope::begin(hwnd)));
    LRESULT(0)
}
unsafe fn execute(hwnd: HWND, state: &State, validate: bool) -> LRESULT {
    if state.render_active.get() {
        return defer_nested(hwnd, state, validate);
    }
    if super::super::pin_window::preview::active(state)
        && super::super::pin_window::stationary::active(state)
    {
        if !super::super::pin_window::stationary::eligible(state) {
            super::super::pin_window::animation::stop(hwnd, state);
        } else {
            if let Err(error) = super::super::pin_window::stationary::refresh(hwnd, state) {
                *state.error.borrow_mut() = Some(error);
            } else {
                state.error.borrow_mut().take();
            }
            drop(validate.then(|| PaintScope::begin(hwnd)));
            return LRESULT(0);
        }
    }
    if super::super::pin_window::preview::active(state) {
        // The temporary complete frame owns visible output; the host only updates geometry, repainting the original GPU/CPU once at handoff.
        drop(validate.then(|| PaintScope::begin(hwnd)));
        return LRESULT(0);
    }
    state.render_active.set(true);
    let _cycle = Cycle { state, hwnd };
    let scope = validate.then(|| PaintScope::begin(hwnd));
    if super::super::tray::is_resident(state)
        && !IsWindowVisible(hwnd).as_bool()
        && state.pin_resize.rect.get().is_none()
        && state.resolved_display.borrow().is_none()
    {
        let canvas = state.canvas.borrow();
        if !canvas.pinned
            && canvas.image.is_none()
            && !canvas.live_pin
            && canvas.extension_views.is_none()
            && !canvas.has_target()
        {
            // Consume the invalid region without allocating a swapchain for a hidden standby host.
            // The render cycle already guards synchronous nested WM_PAINT and schedules its redraw.
            return LRESULT(0);
        }
    }
    if state.canvas.borrow().creation_blocked() || state.canvas.borrow().render_suspended() {
        // Consume the invalid region without clearing errors, rebuilding backends, or discarding the document; no automatic unlock or timed retry.
        drop(scope);
        return LRESULT(0);
    }
    let result = frame(hwnd, state);
    drop(scope);
    if !state.lifetime.owns(hwnd) {
        return LRESULT(0);
    }
    match result {
        Ok(false) => {}
        Ok(true) => {
            if state.pin_resize.rect.get().is_some() {
                state.pin_resize.prepared.set(true);
            }
            state.error.borrow_mut().take();
            state.render_retries.set(0);
        }
        Err(error) => {
            if state.canvas.borrow().creation_blocked() {
                eprintln!("native-render: {error}; retry=false document=retained reason=unconfirmed-association");
                *state.error.borrow_mut() = Some(error);
                return LRESULT(0);
            }
            let downgraded = state.canvas.borrow_mut().recover_render_after(error.code());
            let recoverable = matches!(
                error.code(),
                D2DERR_RECREATE_TARGET
                    | windows::Win32::Graphics::Dxgi::DXGI_ERROR_DEVICE_REMOVED
                    | windows::Win32::Graphics::Dxgi::DXGI_ERROR_DEVICE_RESET
            );
            let retry = !downgraded
                && recoverable
                && state.render_retries.get() == 0
                && state.canvas.borrow_mut().retry_render_without_composition();
            eprintln!("native-render: {error}; asynchronous-recovery={downgraded}");
            *state.error.borrow_mut() = Some(error);
            if downgraded {
                state.render_retries.set(1);
                super::render_recovery::start(hwnd, state);
            } else if retry {
                state.render_retries.set(1);
                let _ = InvalidateRect(Some(hwnd), None, false);
            } else {
                state.canvas.borrow_mut().suspend_render();
            }
        }
    }
    LRESULT(0)
}
