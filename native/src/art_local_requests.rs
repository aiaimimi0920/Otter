//! Local Art request owner: reuse graph_requests generations and associate the original lease so old downloads/deliveries cannot revive.
//! One Registry per current workspace; background work holds Pending without borrowing the UI registry or creating another thread pool.
use crate::{
    art_issuer_trust::{Authority, Authorization},
    device_authorization_lease::Lease,
    device_session_client::Client,
    graph_requests::{Request as Token, Requests},
    live_relay_http::art_prepare::{Prepared, Request},
    workspace_id::UnitId,
};
use std::sync::Arc;
#[path = "art_local_description.rs"]
pub mod description;
#[derive(Debug, PartialEq)]
pub enum Error {
    Workspace,
    Lease,
    Identity,
    Capacity,
    Stale,
    Trust,
    Download(crate::art_issuer_trust::Error),
    Configuration(crate::device_authorization_config::Error),
    Worker,
    Description(crate::live_relay_http::art_describe::Error),
}
struct Entry {
    unit: UnitId,
    token: Token,
    lease: Arc<Lease>,
    delivered: bool,
    running: bool,
}
pub struct Registry {
    workspace: String,
    requests: Requests,
    active: Vec<Entry>,
}
pub struct Pending {
    unit: UnitId,
    token: Token,
    request: Request,
    authorization: Authorization,
}
pub struct Completion {
    unit: UnitId,
    token: Token,
    result: Result<Prepared, Error>,
}
impl Registry {
    pub fn new(workspace: String) -> Self {
        Self {
            workspace,
            requests: Requests::default(),
            active: Vec::new(),
        }
    }
    /// The host supplies request workspace/instance/package; this owner creates a fresh requestId and rejects script reuse.
    /// Each attempt uses a separate Lease; never lend an in-use network-cancellation slot to another request.
    pub fn begin(
        &mut self,
        unit: UnitId,
        mut request: Request,
        lease: Arc<Lease>,
        authority: &mut Authority,
    ) -> Result<Pending, Error> {
        if request.binding.workspace_id != self.workspace {
            return Err(Error::Workspace);
        }
        let (token, authorization) =
            self.register(&unit, &request.binding.origin, lease, authority)?;
        request.binding.request_id = token.id.to_string();
        Ok(Pending {
            unit,
            token,
            request,
            authorization,
        })
    }
    /// Both entry points share generation/revocation transactions; description cannot bypass trust registration or create another request owner.
    fn register(
        &mut self,
        unit: &UnitId,
        origin: &str,
        lease: Arc<Lease>,
        authority: &mut Authority,
    ) -> Result<(Token, Authorization), Error> {
        if self.active.iter().any(|e| Arc::ptr_eq(&e.lease, &lease)) {
            return Err(Error::Lease);
        }
        self.requests
            .next_generation(unit)
            .map_err(|_| Error::Capacity)?;
        let id =
            unsafe { windows::Win32::System::Com::CoCreateGuid() }.map_err(|_| Error::Identity)?;
        let id = UnitId::parse(&format!("request:{id:?}")).ok_or(Error::Identity)?;
        let authorization = authority
            .authorize(origin, lease.clone())
            .map_err(|_| Error::Trust)?;
        self.active.try_reserve(1).map_err(|_| Error::Capacity)?;
        let token = self
            .requests
            .begin(unit.clone(), id)
            .map_err(|_| Error::Capacity)?;
        // Revoke the old request only after establishing the new generation; preparation failure preserves the original instance.
        if let Some(index) = self.active.iter().position(|e| &e.unit == unit) {
            self.active.remove(index).retire();
        }
        self.active.push(Entry {
            unit: unit.clone(),
            token: token.clone(),
            lease,
            delivered: false,
            running: false,
        });
        Ok((token, authorization))
    }
    fn claim(&mut self, unit: &UnitId, token: &Token) -> Result<(), Error> {
        if !self.requests.is_latest(unit, token) {
            return Err(Error::Stale);
        }
        let entry = self
            .active
            .iter_mut()
            .find(|e| &e.unit == unit && &e.token == token)
            .ok_or(Error::Stale)?;
        if entry.delivered || entry.lease.is_revoked() {
            return Err(Error::Stale);
        }
        entry.delivered = true;
        Ok(())
    }
    /// Accept exactly once; successful delivery is not Runtime takeover, and retirement before takeover must revoke the whole Lease.
    pub fn accept(&mut self, completion: Completion) -> Result<Prepared, Error> {
        self.claim(&completion.unit, &completion.token)?;
        match completion.result {
            Ok(prepared) => Ok(prepared),
            Err(error) => {
                self.cancel(&completion.unit);
                Err(error)
            }
        }
    }
    pub fn cancel(&mut self, unit: &UnitId) {
        self.requests.invalidate(unit);
        if let Some(index) = self.active.iter().position(|e| &e.unit == unit) {
            self.active.remove(index).retire();
        }
    }
    pub fn clear(&mut self) {
        for entry in self.active.drain(..) {
            entry.retire();
        }
        self.requests.clear();
    }
    /// Removing a node releases its generation budget; revoke the original Lease first, and still require a complete token match for late results.
    pub fn retain_units(&mut self, keep: impl Fn(&UnitId) -> bool) {
        self.active.retain(|entry| {
            if keep(&entry.unit) {
                true
            } else {
                entry.retire();
                false
            }
        });
        self.requests.retain_units(keep);
    }
    /// Register only after Runtime takeover succeeds; download delivery/window-start failure does not confer service-cleanup ownership.
    pub(crate) fn mark_running(&mut self, unit: &UnitId, token: &Token) {
        if let Some(entry) = self
            .active
            .iter_mut()
            .find(|entry| &entry.unit == unit && &entry.token == token && entry.delivered)
        {
            entry.running = true;
        }
    }
}
impl Entry {
    fn retire(&self) {
        if self.running {
            // The original Runtime stops execution, performs bounded serial cancel, and finally revokes authentication on exit.
            self.lease.stop_execution();
        } else {
            self.lease.revoke();
        }
    }
}
impl Drop for Registry {
    fn drop(&mut self) {
        self.clear();
    }
}
impl Pending {
    pub(crate) fn identity(&self) -> (UnitId, Token) {
        (self.unit.clone(), self.token.clone())
    }
    pub fn binding(&self) -> &crate::art_execution_grant::wire::Binding {
        &self.request.binding
    }
    pub fn fetch(self, client: &Client) -> Completion {
        let result = self
            .authorization
            .prepare(client, self.request)
            .map_err(Error::Download);
        Completion {
            unit: self.unit,
            token: self.token,
            result,
        }
    }
}
impl Completion {
    pub(crate) fn failed(unit: UnitId, token: Token, error: Error) -> Self {
        Self {
            unit,
            token,
            result: Err(error),
        }
    }
}
#[cfg(test)]
#[path = "art_local_requests_tests.rs"]
pub(crate) mod tests;
