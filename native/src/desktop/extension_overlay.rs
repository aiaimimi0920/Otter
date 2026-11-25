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
}
pub(super) unsafe fn sync(hwnd: HWND, state: &State) -> Result<bool> {
    extension_document::observe(state);
    let snapshot = state.extensions.borrow().snapshot.clone();
    let document = state.extension_document.borrow();
    let mut cache = state.extension_overlay.borrow_mut();
    let same = match &snapshot {
        Some(s) => cache.catalog.ptr_eq(&Arc::downgrade(s)),
        None => cache.missing,
    };
    if same && cache.stamp.as_ref().is_some_and(|s| s.matches(&document)) {
        return Ok(false);
    }
    let mut canvas = state.canvas.borrow_mut();
    // Revoke old projections first; failed preparation must not keep displaying already-revoked/invalid old scenes.
    let captured = extension_visuals::clear(&mut canvas);
    cache.catalog = snapshot.as_ref().map(Arc::downgrade).unwrap_or_default();
    cache.stamp = Some(document.stamp());
    cache.missing = snapshot.is_none();
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
