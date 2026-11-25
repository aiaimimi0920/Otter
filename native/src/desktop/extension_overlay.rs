//! Edit-strip cached-result actions and catalog/source generation changes; cache keys retain lightweight evidence only, without paint/drag attachment parsing.
use super::{
    extension_document::{self, Stamp},
    extension_visuals, State,
};
use crate::{extension_visual, loom_extension_snapshot::Snapshot};
use std::sync::{Arc, Weak};
use windows::{core::Result, Win32::Foundation::*};
#[derive(Default)]
pub(super) struct Cache {
    catalog: Weak<Snapshot>,
    stamp: Option<Stamp>,
    missing: bool,
    language_redraw: bool,
}
impl Cache {
    fn matches(
        &self,
        catalog: Option<&Arc<Snapshot>>,
        document: &extension_document::Owner,
    ) -> bool {
        let same = match catalog {
            Some(snapshot) => self.catalog.ptr_eq(&Arc::downgrade(snapshot)),
            None => self.missing,
        };
        same && self
            .stamp
            .as_ref()
            .is_some_and(|stamp| stamp.matches(document))
    }
}
pub(super) unsafe fn sync(hwnd: HWND, state: &State) -> Result<bool> {
    extension_document::observe(state);
    let snapshot = state.extensions.borrow().snapshot.clone();
    let document = state.extension_document.borrow();
    let mut cache = state.extension_overlay.borrow_mut();
    if cache.matches(snapshot.as_ref(), &document) {
        drop(cache);
        drop(document);
        return refresh_language(hwnd, state);
    }
    let mut canvas = state.canvas.borrow_mut();
    // Revoke old projections first; failed preparation must not keep displaying already-revoked/invalid old scenes.
    let captured = extension_visuals::clear(&mut canvas);
    cache.catalog = snapshot.as_ref().map(Arc::downgrade).unwrap_or_default();
    cache.stamp = Some(document.stamp());
    cache.missing = snapshot.is_none();
    cache.language_redraw = false;
    let result = if let Some(revision) = document.revision() {
        crate::surface_viewport::Viewport::from_window(hwnd)
            .and_then(|viewport| {
                extension_visuals::Views::prepare(
                    extension_visual::project(snapshot.as_deref(), &document.store, revision),
                    viewport.logical(),
                )
            })
            .map(|views| {
                canvas.extension_views =
                    Some(views.bind(&canvas, state.resolved_display.borrow().as_ref()));
            })
    } else {
        Ok(())
    };
    drop(canvas);
    drop(document);
    drop(cache);
    if captured && windows::Win32::UI::Input::KeyboardAndMouse::GetCapture() == hwnd {
        let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
    }
    super::hit_region::update(hwnd, state)?;
    extension_visuals::invalidate(hwnd);
    super::pin_window::stationary::refresh(hwnd, state)?;
    result.map(|_| true)
}
pub(super) unsafe fn refresh_language(hwnd: HWND, state: &State) -> Result<bool> {
    let views = {
        let snapshot = state.extensions.borrow();
        let document = state.extension_document.borrow();
        if !state
            .extension_overlay
            .borrow()
            .matches(snapshot.snapshot.as_ref(), &document)
        {
            return Ok(false);
        }
        let canvas = state.canvas.borrow();
        canvas
            .extension_views
            .as_ref()
            .and_then(|bound| bound.views(&canvas, state.resolved_display.borrow().as_ref()))
    };
    let Some(views) = views else {
        return Ok(false);
    };
    let changed = views.refresh_language(crate::ui_text::current_language())?;
    redraw_language(&state.extension_overlay, changed, || {
        extension_visuals::invalidate(hwnd);
        super::pin_window::stationary::refresh(hwnd, state)
    })
}
fn redraw_language(
    cache: &std::cell::RefCell<Cache>,
    changed: bool,
    repaint: impl FnOnce() -> Result<()>,
) -> Result<bool> {
    {
        let mut cache = cache.borrow_mut();
        cache.language_redraw |= changed;
        if !cache.language_redraw {
            return Ok(false);
        }
    }
    // Release all owner borrows before synchronous window calls. A failed repaint retries at normal settlement.
    repaint()?;
    cache.borrow_mut().language_redraw = false;
    Ok(true)
}
pub(super) fn current(state: &State) -> bool {
    let cache = state.extension_overlay.borrow();
    cache.catalog.upgrade().is_some_and(|catalog| {
        state
            .extensions
            .borrow()
            .snapshot
            .as_ref()
            .is_some_and(|s| Arc::ptr_eq(s, &catalog))
    }) && cache
        .stamp
        .as_ref()
        .is_some_and(|s| s.matches(&state.extension_document.borrow()))
}
pub(super) fn available(state: &State, translation: bool) -> bool {
    super::extension_commands::available(state, translation)
}
pub(super) unsafe fn toggle(hwnd: HWND, state: &State, translation: bool) -> Result<()> {
    super::extension_commands::request(hwnd, state, translation)
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "extension_overlay_redraw_tests.rs"]
mod redraw_tests;
