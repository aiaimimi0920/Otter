//! Window owner for authenticated mount results; local closing does not impersonate server disposed, and requests never replay automatically.
use super::surface_window::SurfaceWindow;
use crate::{
    device_authorization_lease::{Error as LeaseError, Lease, Notification},
    surface_attach_runtime::{AdmissionError, Outcome},
    surface_lifecycle_owner::Owner,
};
use std::os::windows::io::BorrowedHandle;
use windows::core::{Error, Result};
use windows::Win32::Foundation::E_FAIL;
#[path = "surface_confirmation_dialog.rs"]
pub(super) mod confirmation_dialog;
#[path = "surface_receipt_decision.rs"]
mod receipt_decision;
pub use receipt_decision::ReceiptDecisionError;
#[path = "surface_batch_delivery.rs"]
mod batch_delivery;
pub use batch_delivery::BatchReceipt;
#[path = "surface_batch_confirmation.rs"]
mod batch_confirmation;
pub use batch_confirmation::{BatchDecisionError, PreparedBatchDecision};
#[path = "surface_batch_decision_transfer.rs"]
mod batch_decision_transfer;
pub use batch_decision_transfer::{BatchDecisionInput, BatchDecisionStartError};
#[path = "surface_batch_continuation.rs"]
mod batch_continuation;
pub use batch_continuation::BatchContinuationInput;
#[path = "surface_event_delivery.rs"]
mod event_delivery;
pub use event_delivery::{EventReceipt, PresentationError};
#[path = "surface_window_resources.rs"]
mod resources;
pub use resources::ImageError;
#[path = "surface_window_admission.rs"]
mod admission;
#[path = "surface_window_javascript.rs"]
mod javascript;

#[derive(Debug)]
pub enum OpenError {
    Admission(AdmissionError),
    Notification(std::io::Error),
    Window(Error),
}
pub struct AuthorizedWindow {
    window: SurfaceWindow,
    lease: Lease,
    notice: Notification,
    lifecycle: crate::surface_activation::Activation,
    stopped: Option<LeaseError>,
    request: std::sync::Arc<crate::surface_attach_runtime::Request>,
    activation_error: Option<String>,
    rejected_owner: Option<Box<Owner>>,
    shown: bool,
    snapshot_revision: u64,
    result_binding_pending: bool,
    deferred_click: Option<super::surface_window::DeferredClick>,
    resources: std::cell::RefCell<Option<crate::surface_resource_snapshot::ResourceSnapshot>>,
    images: std::cell::RefCell<resources::InstalledImages>,
    image_requests: std::cell::RefCell<resources::Requests>,
    javascript: Option<javascript::Execution>,
}
impl AuthorizedWindow {
    pub fn is_alive(&self) -> bool {
        self.window.is_alive()
    }
    #[cfg(test)]
    pub(crate) fn edit_test_parent(&self) -> windows::Win32::Foundation::HWND {
        self.window.dialog_owner().unwrap()
    }
    pub fn is_shown(&self) -> bool {
        self.shown && self.is_alive()
    }
    pub fn lifecycle(&self) -> Option<&Owner> {
        self.lifecycle.owner()
    }
    pub fn binding(&self) -> &crate::surface_live_binding::Binding {
        self.lifecycle.binding()
    }
    /// Snapshot version of the currently presented scene; lifecycle ACKs cannot advance scene versions.
    pub fn snapshot_revision(&self) -> u64 {
        self.snapshot_revision
    }
    /// Only a newly mounted window gets one rebinding opportunity; ordinary results cannot reset this marker.
    pub fn result_binding_pending(&self) -> bool {
        self.result_binding_pending
    }
    pub fn bind_node_results(
        &mut self,
        now_ms: u64,
        bind: impl FnOnce(&crate::surface_live_binding::Binding) -> Result<bool>,
    ) -> Result<bool> {
        if !self.result_binding_pending {
            return Ok(false);
        }
        self.refresh(now_ms)?;
        if self.stopped.is_some()
            || !self.is_shown()
            || !self.lifecycle.owner().is_some_and(Owner::confirmed_active)
        {
            return Ok(false);
        }
        // refresh may synchronously close HWND; acquire node/View borrows only within the post-refresh callback.
        let bound = bind(self.binding())?;
        if bound {
            self.result_binding_pending = false;
        }
        Ok(bound)
    }
    /// Check authorization and current presentation identity before/after preparation; nothing was sent, so begin_event must revalidate again.
    pub fn prepare_click(
        &mut self,
        click: super::surface_window::Click,
        now_ms: u64,
    ) -> Result<Vec<crate::surface_click_event::ClickEvent>> {
        self.refresh(now_ms)?;
        if self.javascript.is_some()
            || self.stopped.is_some()
            || !self.is_shown()
            || !self.lifecycle.owner().is_some_and(Owner::confirmed_active)
        {
            return Err(Error::new(E_FAIL, "Surface input is not authorized"));
        }
        let binding = self.binding().clone();
        let revision = self.snapshot_revision;
        let events = self
            .window
            .click_events_for_candidate(click, &binding, revision)?;
        self.refresh(now_ms)?;
        if self.stopped.is_some()
            || !self.is_shown()
            || self.binding() != &binding
            || self.snapshot_revision != revision
            || !self.lifecycle.owner().is_some_and(Owner::confirmed_active)
        {
            return Err(Error::new(E_FAIL, "Surface input became stale"));
        }
        Ok(events)
    }
    /// When there is no candidate, avoid repeatedly preparing leases and client configuration in the main loop.
    pub fn has_pending_click(&self) -> bool {
        if self.javascript.is_some() {
            return self.javascript_has_input();
        }
        self.deferred_click.is_some()
            || self.window.has_pending_click()
            || self.window.has_pending_edit()
    }
    /// Application intent is not authorization; actual display and unique-owner state jointly determine local-input eligibility.
    pub fn sync_pointer(&mut self, requested: bool, now_ms: u64) -> Result<bool> {
        self.sync_input(requested, requested, now_ms)
    }
    /// Local edits may survive action-owner handoffs; commit eligibility still requires the sole confirmed owner and application admission.
    pub fn sync_input(&mut self, editing: bool, submitting: bool, now_ms: u64) -> Result<bool> {
        self.refresh(now_ms)?;
        // JS captures the next candidate only when listener/update/presentation receipts are all idle; emit still uses the original batch FIFO.
        let editing = editing
            && self.stopped.is_none()
            && self.is_shown()
            && self.lifecycle.local_edit_active();
        let enabled = editing
            && submitting
            && self.javascript_input_idle()
            && self.lifecycle.owner().is_some_and(Owner::confirmed_active);
        if !editing {
            self.deferred_click = None;
        }
        if self.is_alive() {
            self.window.set_input_policy(editing, enabled)?;
        }
        Ok(enabled && !self.schedule_javascript_animation(now_ms)?)
    }
    pub fn local_edit_enabled(&self) -> bool {
        self.window.local_edit_enabled()
    }
    pub(crate) fn disable_local_input(&mut self) -> Result<()> {
        self.deferred_click = None;
        if self.is_alive() {
            self.window.set_input_policy(false, false)?;
        }
        Ok(())
    }
    /// Prepare field prefixes through the same authorization gate; after reaching a click fence, later changes must not dispatch first.
    pub fn take_prepared_click(
        &mut self,
        now_ms: u64,
    ) -> Result<Option<Vec<crate::surface_click_event::ClickEvent>>> {
        if self.javascript.is_some() {
            return self.take_javascript_input(now_ms);
        }
        if self.deferred_click.is_none() {
            if let Some(click) = self.window.take_click() {
                self.deferred_click = Some(self.window.defer_click(
                    click,
                    self.binding(),
                    self.snapshot_revision,
                )?);
            }
        }
        if let Some(pending) = &self.deferred_click {
            use super::surface_edit_mount::FenceState;
            match self.window.deferred_state(pending) {
                FenceState::Invalid => {
                    self.deferred_click = None;
                    return Err(Error::new(
                        E_FAIL,
                        "Surface click dependency invalidated or expired",
                    ));
                }
                FenceState::InFlight => return Ok(None),
                FenceState::Ready => {
                    let pending = self.deferred_click.take().unwrap();
                    self.refresh(now_ms)?;
                    if self.stopped.is_some()
                        || !self.is_shown()
                        || !self.lifecycle.owner().is_some_and(Owner::confirmed_active)
                    {
                        return Err(Error::new(E_FAIL, "Surface deferred click unauthorized"));
                    }
                    let binding = self.binding().clone();
                    let revision = self.snapshot_revision;
                    let events = self.window.prepare_deferred(&pending, &binding, revision)?;
                    self.refresh(now_ms)?;
                    if self.stopped.is_some()
                        || !self.is_shown()
                        || self.binding() != &binding
                        || self.snapshot_revision != revision
                        || !self.lifecycle.owner().is_some_and(Owner::confirmed_active)
                    {
                        return Err(Error::new(E_FAIL, "Surface deferred click became stale"));
                    }
                    return Ok(Some(events));
                }
                FenceState::Pending => {}
            }
        }
        if self.window.has_pending_edit() {
            self.refresh(now_ms)?;
            if self.stopped.is_some()
                || !self.is_shown()
                || !self.lifecycle.owner().is_some_and(Owner::confirmed_active)
            {
                return Err(Error::new(E_FAIL, "Surface edit is not authorized"));
            }
            let binding = self.binding().clone();
            let revision = self.snapshot_revision;
            let events = self.window.prepare_edit_before(
                &binding,
                revision,
                self.deferred_click.as_ref(),
            )?;
            let refreshed = self.refresh(now_ms);
            if refreshed.is_err()
                || self.stopped.is_some()
                || !self.is_shown()
                || self.binding() != &binding
                || self.snapshot_revision != revision
                || !self.lifecycle.owner().is_some_and(Owner::confirmed_active)
            {
                self.window.cancel_edits(&events);
                refreshed?;
                return Err(Error::new(E_FAIL, "Surface edit became stale"));
            }
            if events.is_empty()
                && self.deferred_click.as_ref().is_some_and(|pending| {
                    self.window.deferred_state(pending)
                        == super::surface_edit_mount::FenceState::Ready
                })
            {
                return self.take_prepared_click(now_ms);
            }
            return Ok(Some(events));
        }
        let Some(click) = self.window.take_click() else {
            return Ok(None);
        };
        self.prepare_click(click, now_ms).map(Some)
    }
    pub fn cancel_prepared_input(&mut self, events: Vec<crate::surface_click_event::ClickEvent>) {
        if let Some(js) = self.javascript.as_mut() {
            // Complete unattempted emits whose owner was not transferred retain original IDs/bytes without rerunning listeners.
            if js.events.is_none() && events.iter().all(|event| !event.attempted()) {
                js.events = Some(events);
            }
        } else {
            self.window.cancel_edits(&events);
        }
    }
    /// Input tasks acquire the sole owner only from the currently displayed, still-authorized, version-matching window.
    pub fn begin_event(
        &mut self,
        event: &crate::surface_click_event::ClickEvent,
        now_ms: u64,
    ) -> Result<Option<Owner>> {
        self.refresh(now_ms)?;
        if self.javascript.is_some()
            || self.stopped.is_some()
            || !self.is_shown()
            || event.revision() != self.snapshot_revision
        {
            return Ok(None);
        }
        Ok(self.lifecycle.take_event(event))
    }
    /// Acquire the whole batch's sole owner from the currently presented window; transfer only after version/identity/capacity preflight.
    pub fn begin_batch(
        &mut self,
        events: &[crate::surface_click_event::ClickEvent],
        now_ms: u64,
    ) -> Result<Option<Owner>> {
        if let Err(error) = self.refresh(now_ms) {
            self.window.cancel_edits(events);
            return Err(error);
        }
        if !self.javascript_ready()
            || self.stopped.is_some()
            || !self.is_shown()
            || events
                .iter()
                .any(|event| event.is_javascript() != self.javascript.is_some())
        {
            self.window.cancel_edits(events);
            return Ok(None);
        }
        if let Err(error) = self.window.set_submission_enabled(false) {
            self.window.cancel_edits(events);
            return Err(error);
        }
        if self.javascript.is_some() {
            self.validate_javascript_input_viewport()?;
        }
        let owner = self.lifecycle.take_batch(events, self.snapshot_revision);
        if owner.is_none() {
            self.window.cancel_edits(events);
        }
        Ok(owner)
    }
    /// Restore only the batch owner without accepting any scenes; callers remain responsible for retaining all results.
    pub fn finish_batch(
        &mut self,
        events: &[crate::surface_click_event::ClickEvent],
        owner: Owner,
    ) -> std::result::Result<(), Box<Owner>> {
        self.lifecycle.restore_batch(events, owner)
    }
    /// Return only the owner; callers retain ACK/delivery eligibility, and restoring the owner does not redisplay the window.
    pub fn finish_event(
        &mut self,
        event: &crate::surface_click_event::ClickEvent,
        owner: Owner,
    ) -> std::result::Result<(), Box<Owner>> {
        self.lifecycle.restore_event(event, owner)
    }
    pub fn begin_activation(&mut self, now_ms: u64) -> Result<Option<Owner>> {
        self.refresh(now_ms)?;
        if self.stopped.is_some() || !self.is_alive() || !self.javascript_ready() {
            return Ok(None);
        }
        Ok(self.lifecycle.take())
    }
    pub fn finish_activation(&mut self, owner: Owner) -> std::result::Result<(), Box<Owner>> {
        self.lifecycle.restore(owner)
    }
    pub fn begin_deactivation(&mut self, now_ms: u64) -> Result<Option<Owner>> {
        self.refresh(now_ms)?;
        if self.is_alive() || self.stopped.is_some() || !self.javascript_finished() {
            return Ok(None);
        }
        Ok(self.lifecycle.take_inactive())
    }
    pub fn activation_error(&self) -> Option<&str> {
        self.activation_error.as_deref()
    }
    pub(super) fn failed_start(&mut self, error: crate::surface_lifecycle_runtime::StartError) {
        self.activation_error = Some(error.error.to_string());
        // StartError contractually returns the original owner; preserve rejection evidence even if that contract is broken.
        if let Err(owner) = self.finish_activation(*error.owner) {
            self.rejected_owner = Some(owner);
        }
    }
    pub fn rejected_owner(&self) -> Option<&Owner> {
        self.rejected_owner.as_deref()
    }
    pub(super) fn record_activation_error(&mut self, error: String) {
        self.activation_error = Some(error);
    }
    pub fn request(&self) -> &crate::surface_attach_runtime::Request {
        &self.request
    }
    pub fn stopped(&self) -> Option<LeaseError> {
        self.stopped
    }
    /// The application's wait set must register this handle and stop waiting at expires_at_ms.
    pub fn revocation_handle(&self) -> BorrowedHandle<'_> {
        self.notice.handle()
    }
    pub fn expires_at_ms(&self) -> u64 {
        self.images
            .borrow()
            .values()
            .map(|image| image.request.expires_at_ms())
            .min()
            .unwrap_or(u64::MAX)
            .min(self.lease.expires_at_ms())
            .min(self.javascript_deadline())
    }
    /// Call at application message-settlement points and before display; local closing still retains attachment bindings and versions.
    pub fn refresh(&mut self, now_ms: u64) -> Result<()> {
        if let Err(error) = self
            .lease
            .validate(self.lifecycle.binding().origin(), now_ms)
        {
            self.stopped = Some(error);
            self.resources.get_mut().take();
            self.images.get_mut().clear();
            *self.image_requests.get_mut() = Default::default();
            self.window.close()?;
        }
        if !self.window.is_alive() {
            self.stop_javascript();
            self.resources.get_mut().take();
            self.images.get_mut().clear();
            *self.image_requests.get_mut() = Default::default();
        } else if self.stopped.is_none() {
            if self.javascript.is_none() {
                self.refresh_images(now_ms)?;
            }
            // Expiry projection may call HWND APIs; recheck revocation after synchronous reentrancy.
            if let Err(error) = self.lease.validate(self.binding().origin(), now_ms) {
                self.stopped = Some(error);
                self.close()?;
            }
        }
        self.poll_javascript(now_ms)
    }
    pub fn show(&mut self, now_ms: u64) -> Result<()> {
        self.refresh(now_ms)?;
        if !self.lifecycle.owner().is_some_and(Owner::confirmed_active) {
            return Err(Error::new(E_FAIL, "Surface activation is not confirmed"));
        }
        if self.stopped.is_some() || !self.window.is_alive() {
            return Err(Error::new(E_FAIL, "Surface authorization ended"));
        }
        // Wait for the background synchronous Active ACK; not-yet-ready state is not a permanent activation error.
        if !self.javascript_ready() {
            return Ok(());
        }
        if !self.shown {
            // Activation ACK advances binding lifecycle version; do not reuse the mounted-stage binding.
            self.window.bind_edits(self.binding())?;
            self.window.show()?;
            self.shown = true;
        }
        self.refresh(now_ms)?;
        if self.stopped.is_some() {
            return Err(Error::new(E_FAIL, "Surface authorization ended"));
        }
        Ok(())
    }
    pub fn close(&self) -> Result<()> {
        self.stop_javascript();
        self.resources.borrow_mut().take();
        self.images.borrow_mut().clear();
        *self.image_requests.borrow_mut() = Default::default();
        self.window.close()
    }
    pub fn take_error(&self) -> Option<Error> {
        self.window.take_error()
    }
    /// After local window closure, hand the lifecycle owner and lease to a serial background transaction; no implicit requests.
    pub fn into_lifecycle(self) -> (Option<Owner>, Lease) {
        let Self {
            window,
            lease,
            lifecycle,
            ..
        } = self;
        drop(window);
        (lifecycle.into_owner(), lease)
    }
}
