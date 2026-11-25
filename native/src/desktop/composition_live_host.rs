//! UI-thread Live presentation controller; the host still owns HWND, message dispatch, and capture authorization.
use super::{notification::WindowSignal, *};
use crate::live_frames::{Consumer, Delivery};
use std::{cell::Cell, rc::Rc};
use windows::Win32::Foundation::{LPARAM, WPARAM};

#[derive(Debug, PartialEq, Eq)]
pub enum Update {
    Ignored,
    Idle,
    Frame,
    Closed,
}

/// Share committed geometry on the same UI thread without GPU/frame ownership; it stays readable while Host is borrowed out.
#[derive(Clone, Default)]
pub(super) struct Presented(Rc<Cell<Option<Geometry>>>);
#[derive(Clone, Copy)]
struct Geometry {
    size: [u32; 2],
    sequence: u64,
    viewport: crate::live_viewport::Viewport,
}
impl Presented {
    pub(super) fn get(&self, size: [u32; 2]) -> Option<(u64, crate::live_viewport::Viewport)> {
        self.0
            .get()
            .filter(|geometry| geometry.size == size)
            .map(|geometry| (geometry.sequence, geometry.viewport))
    }
    pub(super) fn clear(&self) {
        self.0.set(None);
    }
}

pub struct Host {
    signal: Option<WindowSignal<Texture>>,
    surface: Option<Surface>,
    pending: Option<Arc<Frame<Texture>>>,
    hwnd: HWND,
    failures: u64,
    opacity: f32,
    chrome_visible: bool,
    pin_selected: bool,
    pin_editor: Option<std::rc::Rc<crate::editor::Editor>>,
    presented: Presented,
}
impl Host {
    /// Do not start capture; consume an authorized source. Drop this object before HWND destruction.
    pub fn bind(hwnd: HWND, consumer: Consumer<Texture>) -> Result<Self> {
        Self::bind_presented(hwnd, consumer, Presented::default())
    }
    pub(super) fn bind_presented(
        hwnd: HWND,
        consumer: Consumer<Texture>,
        presented: Presented,
    ) -> Result<Self> {
        Ok(Self {
            signal: Some(WindowSignal::bind(hwnd, consumer)?),
            surface: None,
            pending: None,
            hwnd,
            failures: 0,
            opacity: 1.0,
            chrome_visible: true,
            pin_selected: false,
            pin_editor: None,
            presented,
        })
    }
    /// On Frame/Closed, the host should invalidate its window rather than draw every frame during message bursts.
    pub fn receive(&mut self, message: u32, wp: WPARAM, lp: LPARAM) -> Result<Update> {
        let delivery = self
            .signal
            .as_mut()
            .and_then(|signal| signal.receive(self.hwnd, message, wp, lp));
        match delivery {
            Some(delivery) => self.accept(delivery),
            None => Ok(Update::Ignored),
        }
    }
    pub fn notification_failures(&self) -> u64 {
        self.signal
            .as_ref()
            .map_or(self.failures, WindowSignal::notification_failures)
    }
    pub fn is_closed(&self) -> bool {
        self.signal.is_none()
    }
    /// Opacity belongs to window visuals, not baked frames; retain it before the first frame/during pauses and reapply after surface reconstruction.
    pub fn set_opacity(&mut self, opacity: f32) -> Result<()> {
        if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
            return Err(invalid("Invalid Live opacity"));
        }
        if let Some(surface) = self.surface.as_mut() {
            surface.set_opacity(opacity)?;
        }
        self.opacity = opacity;
        Ok(())
    }
    pub(in crate::desktop) fn set_chrome_visible(&mut self, visible: bool) {
        self.chrome_visible = visible;
    }
    pub(in crate::desktop) fn select_pin(&mut self, selected: bool) {
        self.pin_selected = selected;
    }
    pub(in crate::desktop) fn set_pin_editor(&mut self, editor: crate::editor::Editor) {
        self.pin_editor = Some(std::rc::Rc::new(editor));
    }
    pub(super) fn content_size(&self, source: [u32; 2]) -> [f64; 2] {
        self.pin_editor
            .as_ref()
            .map_or(source.map(f64::from), |editor| {
                let (bounds, _, _) = super::edit_view::geometry(editor);
                [bounds.width, bounds.height]
            })
    }
    /// Explicit snapshots collect the latest delivered frame, possibly not yet presented, without changing the presented identity used for input.
    /// Callers must retain the entire Arc; after closure, never fall back to an old image from a previous surface.
    pub fn snapshot_frame(&mut self) -> Result<Option<Arc<Frame<Texture>>>> {
        match self.refresh()? {
            Update::Closed => return Ok(None),
            Update::Frame => unsafe {
                // Snapshots consume delivery, so late frame notifications see only Idle. Keep one redraw request;
                // static sources must still present, and input-coordinate identity updates only after successful paint.
                if !windows::Win32::Graphics::Gdi::InvalidateRect(Some(self.hwnd), None, false)
                    .as_bool()
                {
                    return Err(Error::from_win32());
                }
            },
            _ => {}
        }
        Ok(self
            .pending
            .clone()
            .or_else(|| self.surface.as_ref().map(|surface| surface.frame.clone())))
    }
    /// Return only geometry successfully submitted for presentation; it is neither input authorization nor a guarantee of compositor scanout.
    /// Callers pass current client dimensions to reject stale coordinates after resize but before renewed presentation.
    pub fn presented_viewport(
        &self,
        size: [u32; 2],
    ) -> Option<(u64, crate::live_viewport::Viewport)> {
        self.presented.get(size)
    }
    #[cfg(test)]
    pub(in crate::desktop) fn applied_opacity_for_test(&self) -> Option<f32> {
        self.surface
            .as_ref()
            .map(|surface| surface.composition.opacity_for_test())
    }
    #[cfg(test)]
    pub(super) fn has_surface(&self) -> bool {
        self.surface.is_some()
    }
    #[cfg(test)]
    pub(in crate::desktop) fn frame_for_test(&self) -> Option<Arc<Frame<Texture>>> {
        self.surface.as_ref().map(|surface| surface.frame.clone())
    }
    /// Compensating reads through existing host events; failed terminal delivery with no later events still requires a host closure policy.
    pub fn refresh(&mut self) -> Result<Update> {
        match self.signal.as_mut() {
            Some(signal) => {
                let delivery = signal.refresh();
                self.accept(delivery)
            }
            None => Ok(Update::Closed),
        }
    }
    /// Pause output without unsubscribing; retain only the latest frame, with recovery independent of further source-image changes.
    pub fn suspend(&mut self) {
        self.presented.clear();
        if let Some(surface) = self.surface.take() {
            if self.pending.is_none() {
                self.pending = Some(surface.frame.clone());
            }
            // Release the presentation chain and imported bitmap, but not the shared frame's budget owner early.
            drop(surface);
        }
    }
    /// Return true only when the swap chain confirms submission. Zero dimensions release presentation resources without self-triggered redraw.
    pub fn paint(&mut self, size: D2D_SIZE_U) -> Result<bool> {
        self.paint_with(size, Surface::present)
    }
    fn paint_with(
        &mut self,
        size: D2D_SIZE_U,
        present: impl FnOnce(&Surface) -> Result<bool>,
    ) -> Result<bool> {
        self.refresh()?;
        if size.width == 0 || size.height == 0 {
            self.suspend();
            return Ok(false);
        }
        let result = self.paint_inner(size, present);
        if result.is_err() {
            // Device loss/invalid input retains no GPU owner and creates no redraw retry storm.
            self.close();
        }
        result
    }
    fn paint_inner(
        &mut self,
        size: D2D_SIZE_U,
        present: impl FnOnce(&Surface) -> Result<bool>,
    ) -> Result<bool> {
        if let Some(frame) = self.pending.take() {
            match self.surface.as_mut() {
                Some(surface) => {
                    surface.set_frame(frame)?;
                }
                None => self.surface = Some(Surface::new(self.hwnd, size, frame)?),
            }
        }
        let Some(surface) = self.surface.as_mut() else {
            return Ok(false);
        };
        if surface.composition.state.size != size {
            // ResizeBuffers replaces the old backbuffer; coordinates cannot be reused even if dimensions later revert.
            self.presented.clear();
            surface.resize(size)?;
        }
        surface.set_opacity(self.opacity)?;
        surface.set_pin_editor(self.pin_editor.clone())?;
        surface.draw_pin_chrome(self.chrome_visible, self.pin_selected)?;
        if !present(surface)? {
            // Nonconfirmation is not termination: retain the latest frame for snapshots/next draw, without active retry loops.
            return Ok(false);
        }
        let destination = [size.width, size.height];
        self.presented
            .0
            .set(surface.viewport(destination).map(|viewport| Geometry {
                size: destination,
                sequence: surface.frame.sequence,
                viewport,
            }));
        Ok(true)
    }
    fn accept(
        &mut self,
        delivery: std::result::Result<Option<Delivery<Texture>>, crate::live_frames::Error>,
    ) -> Result<Update> {
        match delivery {
            Ok(Some(delivery)) => {
                self.pending = Some(delivery.frame);
                Ok(Update::Frame)
            }
            Ok(None) => Ok(Update::Idle),
            Err(crate::live_frames::Error::Closed) => {
                self.close();
                Ok(Update::Closed)
            }
            Err(error) => {
                self.close();
                Err(invalid(&format!("Live consumer failed: {error:?}")))
            }
        }
    }
    /// Unsubscribe before releasing pending frames and GPU surfaces; the final consumer leaving wakes capture shutdown.
    pub fn close(&mut self) {
        // Releasing GPU/notification owners may reenter; revoke input geometry first.
        self.presented.clear();
        self.failures = self.notification_failures();
        self.signal = None;
        self.pending = None;
        self.surface = None;
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
#[path = "composition_live_host_tests.rs"]
mod tests;
