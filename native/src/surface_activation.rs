//! Single-owner handoff for activation transactions; prohibit display while unconfirmed and never resend automatically on failure.
use crate::{
    surface_lifecycle_owner::Owner,
    surface_live_binding::{Binding, Lifecycle},
};
#[path = "surface_activation_batch.rs"]
mod batch;
pub struct Activation {
    binding: Binding,
    owner: Option<Owner>,
    attempted: bool,
    close_attempted: bool,
    pending: Option<Lifecycle>,
    pending_event: Option<String>,
    pending_batch: Option<Vec<String>>,
}
impl Activation {
    pub fn new(owner: Owner) -> Self {
        Self {
            binding: owner.binding().clone(),
            owner: Some(owner),
            attempted: false,
            close_attempted: false,
            pending: None,
            pending_event: None,
            pending_batch: None,
        }
    }
    pub fn binding(&self) -> &Binding {
        &self.binding
    }
    pub fn owner(&self) -> Option<&Owner> {
        self.owner.as_ref()
    }
    /// Local editing eligibility for a confirmed active attachment is not permission to claim the submission owner; the window checks leases separately.
    pub fn local_edit_active(&self) -> bool {
        self.pending.is_none()
            && !self.close_attempted
            && (self.owner.as_ref().is_some_and(Owner::confirmed_active)
                || (self.owner.is_none()
                    && (self.pending_event.is_some() || self.pending_batch.is_some())))
    }
    pub fn into_owner(self) -> Option<Owner> {
        self.owner
    }
    /// A click task exclusively owns the activated owner; it does not change lifecycle revision or allow reuse of an attempted event.
    pub fn take_event(&mut self, event: &crate::surface_click_event::ClickEvent) -> Option<Owner> {
        if event.attempted() {
            return None;
        }
        self.take_event_owner(event)
    }
    /// A decision task reuses the original event identity without resubmitting the click; window/HTTP boundaries recheck leases and deadlines.
    pub fn take_decision(
        &mut self,
        event: &crate::surface_click_event::ClickEvent,
        ack: &crate::surface_action_ack::Ack,
        decision: &crate::surface_confirmation::Decision,
    ) -> Option<Owner> {
        if !event.attempted() || decision.attempted() || !decision.matches(event, ack) {
            return None;
        }
        self.take_event_owner(event)
    }
    fn take_event_owner(
        &mut self,
        event: &crate::surface_click_event::ClickEvent,
    ) -> Option<Owner> {
        let owner = self.owner.as_ref()?;
        if !owner.confirmed_active()
            || owner.binding() != event.binding()
            || self.pending.is_some()
            || self.pending_event.is_some()
            || self.pending_batch.is_some()
        {
            return None;
        }
        self.pending_event = Some(event.id().to_owned());
        self.owner.take()
    }
    /// Submission tasks do not change the lifecycle; they must return an owner with the same event, binding, and confirmation state.
    pub fn restore_event(
        &mut self,
        event: &crate::surface_click_event::ClickEvent,
        owner: Owner,
    ) -> Result<(), Box<Owner>> {
        if self.owner.is_some()
            || self.pending_batch.is_some()
            || self.pending.is_some()
            || self.pending_event.as_deref() != Some(event.id())
            || event.binding() != &self.binding
            || owner.binding() != &self.binding
            || !owner.confirmed_active()
        {
            return Err(Box::new(owner));
        }
        self.owner = Some(owner);
        self.pending_event = None;
        Ok(())
    }
    pub fn take(&mut self) -> Option<Owner> {
        let owner = self.owner.as_ref()?;
        if self.attempted
            || owner.uncertain()
            || owner.disposed()
            || owner.binding().lifecycle() != Lifecycle::Mounted
        {
            return None;
        }
        self.attempted = true;
        self.pending = Some(Lifecycle::Active);
        self.owner.take()
    }
    /// Closing a local window only deactivates its attachment; it does not implicitly destroy workspace nodes or remote persistent instances.
    pub fn take_inactive(&mut self) -> Option<Owner> {
        let owner = self.owner.as_ref()?;
        if self.close_attempted
            || owner.uncertain()
            || owner.disposed()
            || !matches!(
                owner.binding().lifecycle(),
                Lifecycle::Mounted | Lifecycle::Active
            )
        {
            return None;
        }
        self.close_attempted = true;
        self.pending = Some(Lifecycle::Inactive);
        self.owner.take()
    }
    /// Return the supplied owner on rejection; do not lose recovery evidence for other attachments or late transactions.
    pub fn restore(&mut self, owner: Owner) -> Result<(), Box<Owner>> {
        let old = &self.binding;
        let next = owner.binding();
        let identity = old.origin() == next.origin()
            && old.device() == next.device()
            && old.node() == next.node()
            && old.instance() == next.instance()
            && old.attachment() == next.attachment()
            && old.generation() == next.generation();
        // Unsent/unknown may return the original confirmation state; success accepts only the next revision of this transaction's target state.
        let version = old == next
            || (old.lifecycle_revision().checked_add(1) == Some(next.lifecycle_revision())
                && Some(next.lifecycle()) == self.pending);
        if self.pending.is_none() || self.owner.is_some() || !identity || !version {
            return Err(Box::new(owner));
        }
        self.binding = next.clone();
        self.owner = Some(owner);
        self.pending = None;
        Ok(())
    }
}
