//! Viewer identities and pending-join fences reuse original Sessions; opened windows do not depend on initiating panels remaining alive.
use super::*;
use crate::surface_live_binding::Binding;
use std::sync::OnceLock;
#[path = "composition_relay_viewer_host.rs"]
mod host;
pub(crate) use host::ViewerHost;

pub struct ViewerKey {
    origin: String,
    session: String,
}
impl ViewerKey {
    pub(super) fn new(origin: &str, session: &str) -> Self {
        Self {
            origin: origin.into(),
            session: session.into(),
        }
    }
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn session(&self) -> &str {
        &self.session
    }
}
pub struct ViewerState {
    pub key: Arc<ViewerKey>,
    pub stopping: bool,
    pub shown: bool,
}
/// Lifecycle revision is not attachment identity; ordinary ACKs for the same attachment must not cancel joins already started.
pub(crate) fn same_binding(left: &Binding, right: &Binding) -> bool {
    left.matches(
        right.origin(),
        right.device(),
        right.node(),
        right.generation(),
    ) && left.instance() == right.instance()
        && left.attachment() == right.attachment()
}
impl Sessions {
    pub(in crate::desktop) fn take_local_triggers(
        &self,
    ) -> Vec<crate::art_live_trigger::ticket::Ticket> {
        self.entries
            .iter()
            .filter_map(|entry| entry.session.take_local_trigger())
            .collect()
    }
    #[cfg(test)]
    pub(crate) fn test_disconnect_viewer_control(&self, key: &Arc<ViewerKey>) -> bool {
        self.entries
            .iter()
            .find(|e| Arc::ptr_eq(&e.key, key))
            .is_some_and(|e| e.session.test_disconnect_control())
    }
    #[cfg(test)]
    pub(crate) fn test_viewer_control(&self, key: &Arc<ViewerKey>) -> Option<(bool, u64)> {
        self.entries
            .iter()
            .find(|e| Arc::ptr_eq(&e.key, key))
            .map(|e| e.session.test_control())
    }
    #[cfg(test)]
    pub(crate) fn test_disconnect_viewer_media(&self, key: &Arc<ViewerKey>) -> bool {
        self.entries
            .iter()
            .find(|e| Arc::ptr_eq(&e.key, key))
            .is_some_and(|e| e.session.test_disconnect_media())
    }
    #[cfg(test)]
    pub(crate) fn test_viewer_observation(
        &self,
        key: &Arc<ViewerKey>,
    ) -> Option<(
        windows::Win32::Foundation::HWND,
        windows::Win32::Foundation::HWND,
    )> {
        self.entries
            .iter()
            .find(|e| Arc::ptr_eq(&e.key, key))
            .and_then(|e| e.session.test_observation())
    }
    #[cfg(test)]
    pub(crate) fn test_viewer_input(
        &self,
        key: &Arc<ViewerKey>,
    ) -> Option<(
        windows::Win32::Foundation::HWND,
        crate::live_relay_control_runtime::interaction::Status,
    )> {
        self.entries
            .iter()
            .find(|entry| Arc::ptr_eq(&entry.key, key))
            .and_then(|entry| entry.session.test_input())
    }
    #[cfg(test)]
    pub(crate) fn test_frame(
        &self,
        key: &Arc<ViewerKey>,
    ) -> windows::core::Result<Option<(crate::live_relay_frame::Metadata, [u8; 4])>> {
        self.entries
            .iter()
            .find(|entry| Arc::ptr_eq(&entry.key, key))
            .map(|entry| entry.session.test_frame())
            .transpose()
            .map(Option::flatten)
    }
    pub fn viewers(&self) -> Vec<ViewerState> {
        self.entries
            .iter()
            .map(|entry| ViewerState {
                key: entry.key.clone(),
                stopping: entry.session.stopping(),
                shown: entry.session.has_window(),
            })
            .collect()
    }
    /// Only one view of the same source session per device; stopped but unjoined owners still occupy identity and four-slot capacity.
    pub fn start_viewing(
        &mut self,
        config: Config,
        lease: Lease,
        target: Target,
        binding: Binding,
    ) -> io::Result<()> {
        if self
            .entries
            .iter()
            .any(|entry| entry.key.origin == config.origin() && entry.key.session == target.session)
        {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        if binding.origin() != config.origin()
            || binding.device() != lease.device_id()
            || binding.instance() != target.surface
            || binding.attachment() != target.attachment
            || target
                .selected
                .as_ref()
                .is_none_or(|id| id.session() != target.session)
            || target.expected.is_some()
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        // The process shares one original-protocol maximum encoded-frame budget, not one quota per window; decoding retains the original global Pixels budget.
        self.start(config, lease, target, encoded_budget(), read_only)?;
        self.entries
            .last_mut()
            .expect("successful start inserts entry")
            .binding = Some(ViewerHost::Surface(binding));
        Ok(())
    }
    pub(crate) fn start_local_art_viewing(
        &mut self,
        config: Config,
        lease: Lease,
        target: crate::live_relay_control_runtime::LocalArtTarget,
    ) -> io::Result<()> {
        if self
            .entries
            .iter()
            .any(|entry| entry.key.origin == config.origin() && entry.key.session == target.session)
        {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        let binding = target.proof.binding();
        if binding.origin != config.origin()
            || binding.device_id != lease.device_id()
            || target.selected.session() != target.session
            || target.proof.validate().is_err()
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let host = ViewerHost::LocalArt(target.proof.clone());
        self.insert_session(config.origin().into(), target.session.clone(), || {
            crate::live_relay_control_runtime::Runtime::start_local_art_viewer(
                config,
                lease,
                target,
                encoded_budget(),
                read_only,
            )
            .map(Session::from_runtime)
        })?;
        self.entries
            .last_mut()
            .expect("successful start inserts entry")
            .binding = Some(host);
        Ok(())
    }
    pub(crate) fn start_direct_viewing(
        &mut self,
        config: Config,
        lease: Lease,
        selected: crate::live_relay_viewer_identity::Identity,
    ) -> io::Result<()> {
        if self
            .entries
            .iter()
            .any(|e| e.key.origin == config.origin() && e.key.session == selected.session())
        {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        self.insert_session(config.origin().into(), selected.session().into(), || {
            crate::live_relay_control_runtime::Runtime::start_direct_viewer(
                config,
                lease,
                selected,
                encoded_budget(),
            )
            .map(Session::from_runtime)
        })
    }
    pub fn stop_viewer(&mut self, key: &Arc<ViewerKey>) -> bool {
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| Arc::ptr_eq(&entry.key, key))
        else {
            return false;
        };
        entry.session.stop();
        true
    }
    pub fn cancel_viewers(&mut self) {
        for entry in &mut self.entries {
            entry.session.stop();
        }
    }
    /// Call before poll collects Ready; late success for old attachments creates no window. Already-Ready independent viewers end through original authorization.
    pub(crate) fn cancel_pending_viewers(&mut self, mut current: impl FnMut(&ViewerHost) -> bool) {
        for entry in &mut self.entries {
            if !entry.session.joined()
                && entry
                    .binding
                    .as_ref()
                    .is_some_and(|binding| !current(binding))
            {
                entry.session.stop();
            }
        }
    }
}
pub(super) fn encoded_budget() -> Arc<Budget> {
    static ENCODED: OnceLock<Arc<Budget>> = OnceLock::new();
    ENCODED
        .get_or_init(|| {
            Budget::new(
                crate::live_relay_frame::MAX_FRAME_BYTES + crate::live_relay_frame::HEADER_LEN,
            )
        })
        .clone()
}
/// The original Owner validates transactions and publishes Business/state first; this read-only adapter performs no source input or Surface actions.
/// Do not silently ACK unknown business events; the original Host retains known observations/audits for later UI reads of the same snapshot.
fn read_only(events: &[ControlEvent]) -> Result<(), ()> {
    // Loom already registered trigger_condition, and only source executes input_event; viewer must not execute again.
    if events.iter().all(|event| {
        matches!(
            event.kind(),
            "session_state" | "observation" | "trigger_event" | "input_event" | "trigger_condition"
        )
    }) {
        Ok(())
    } else {
        Err(())
    }
}
#[cfg(test)]
#[path = "composition_relay_viewers_tests.rs"]
mod tests;
