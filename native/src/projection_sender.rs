//! Independent write-ahead owner for signed creation; no networking/workspace-binding authority or retirement of uncertain remote side effects.
use super::{envelope, Envelope};
use crate::pixels::Pixels;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
#[path = "projection_sender_activity.rs"]
mod activity;
#[path = "projection_sender_file.rs"]
mod file;
#[path = "projection_sender_record.rs"]
mod record;
#[path = "projection_sender_response.rs"]
mod response;
#[path = "projection_sender_route.rs"]
mod route;
pub(crate) use route::Operation;
#[path = "projection_sender_snapshot.rs"]
pub(crate) mod snapshot;
pub(crate) use activity::Active;
pub use activity::Request;
use record::{Document, Prepared, Record};
#[path = "projection_sender_stop.rs"]
mod stop;
pub use record::{MAX_BYTES, MAX_ENTRIES};
pub use response::{Response, Status};
#[path = "projection_sender_binding.rs"]
pub mod binding;
#[path = "projection_sender_update.rs"]
pub(crate) mod update;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Path,
    Lock,
    Read,
    Format,
    Limit,
    Changed,
    Write,
    RecoveryRetained,
    Snapshot,
    Identity,
    Cancelled,
    Stopped,
    Busy,
    Response,
}
/// Different save destinations require distinct stable scopes; repeatable UnitId alone must never authorize cross-workspace retries.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    scope: String,
    origin: String,
    unit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    offline: Option<route::Offline>,
}
impl Selection {
    pub fn new(scope: &str, origin: &str, unit: &str, target: &str) -> Result<Self, Error> {
        // Validate borrowed fields first, before malformed disk fields can cause tens of MiB of copies.
        validate_selection(scope, origin, unit, Some(target))?;
        let value = Self {
            scope: scope.into(),
            origin: origin.into(),
            unit: unit.into(),
            target: Some(target.into()),
            offline: None,
        };
        value.validate()?;
        Ok(value)
    }
    /// Untargeted invitations and targeted deliveries have different purposes; missing target cannot downgrade to arbitrary device identity.
    pub(crate) fn invitation(scope: &str, origin: &str, unit: &str) -> Result<Self, Error> {
        validate_selection(scope, origin, unit, None)?;
        Ok(Self {
            scope: scope.into(),
            origin: origin.into(),
            unit: unit.into(),
            target: None,
            offline: None,
        })
    }
    fn validate(&self) -> Result<(), Error> {
        if let Some(route) = &self.offline {
            if crate::projection_delivery::targets::Route::offline_id(
                &route.peer_id,
                &route.remote_device_id,
            )
            .as_deref()
                != self.target.as_deref()
            {
                return Err(Error::Format);
            }
            validate_selection(
                &self.scope,
                &self.origin,
                &self.unit,
                Some(&route.remote_device_id),
            )
        } else {
            validate_selection(
                &self.scope,
                &self.origin,
                &self.unit,
                self.target.as_deref(),
            )
        }
    }
}
#[derive(Clone, Copy)]
pub enum Kind {
    Sticker,
    Art,
}
impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Sticker => "sticker",
            Self::Art => "art",
        }
    }
}
pub struct Frame {
    pub png: Pixels,
    pub width: u32,
    pub height: u32,
    pub kind: Kind,
}
pub struct Preparation<'a> {
    pub selection: &'a Selection,
    pub frame: Frame,
    pub identity_root: &'a Path,
    pub device: &'a str,
    pub now_ms: u64,
}
struct State {
    original: Option<Pixels>,
    entries: Vec<Record>,
    retained: bool,
}
struct Inner {
    path: PathBuf,
    state: Mutex<State>,
    activity: activity::Registry,
    _lease: File,
    _directories: Vec<File>,
}
#[derive(Clone)]
pub struct Store(Arc<Inner>);
/// Read-only snapshots are not network permits; network owners separately check current cancellation/authorization/source lifecycle.
#[derive(Clone)]
pub struct Intent {
    store: Store,
    record: Record,
}
impl Store {
    /// Background only, with explicitly selected preexisting profiles; no environment discovery, identity creation, or receiver-file writes.
    pub fn open(root: &Path, cancel: &AtomicBool) -> Result<Self, Error> {
        file::open(root, cancel)
    }
    pub fn prepare(&self, request: Preparation<'_>, cancel: &AtomicBool) -> Result<Intent, Error> {
        self.prepare_tracked(request, cancel, |_| {})
    }
    /// Background only. Register original intent immediately after successful disk commit, including commits returning errors for retained recovery copies.
    /// tracker grants no network permission and cannot reenter Store; original Job still hands off privilege-reduction responsibility on failure.
    pub(crate) fn prepare_tracked(
        &self,
        request: Preparation<'_>,
        cancel: &AtomicBool,
        track: impl FnOnce(Intent),
    ) -> Result<Intent, Error> {
        check(cancel)?;
        request.selection.validate()?;
        // prepare on the same Store also serializes identity reads, preventing cooperative identity-file locks from causing same-key concurrent failures.
        let mut state = self.0.state.lock().map_err(|_| Error::Changed)?;
        self.baseline(&state, cancel)?;
        let identity_store = crate::device_identity_store::Store::new(request.identity_root)
            .map_err(|_| Error::Identity)?;
        let loaded = identity_store
            .load_existing()
            .map_err(|_| Error::Identity)?;
        let identity = loaded.identity();
        if identity.device_id() != Some(request.device) {
            return Err(Error::Identity);
        }
        // Serialize same-key operations through persistence, without locks across HTTP. After first write, new frames cannot replace original signed identity.
        if let Some(old) = state
            .entries
            .iter()
            .find(|old| &old.selection == request.selection)
        {
            old.prepared
                .envelope
                .verify_source(identity)
                .map_err(|_| Error::Identity)?;
            if old.cancelled {
                return Err(Error::Cancelled);
            }
            check(cancel)?;
            let intent = self.intent(old.clone());
            track(intent.clone());
            return Ok(intent);
        }
        if state.entries.len() >= MAX_ENTRIES {
            return Err(Error::Limit);
        }
        let snapshot = snapshot::Snapshot::from_png(
            &request.frame.png,
            request.frame.width,
            request.frame.height,
            cancel,
        )?;
        let envelope = Envelope::sign_projection(
            identity,
            &request.selection.origin,
            &request.selection.unit,
            request.frame.kind.as_str(),
            &snapshot.digest,
            request.now_ms,
        )
        .map_err(|_| Error::Identity)?;
        let record = Record {
            selection: request.selection.clone(),
            prepared: Arc::new(Prepared { envelope, snapshot }),
            cancelled: false,
            stop: None,
        };
        let mut next = state.entries.clone();
        next.push(record.clone());
        let published = self.persist(&mut state, next, cancel);
        if state.entries.iter().any(|entry| entry.same_intent(&record)) {
            track(self.intent(record.clone()));
        }
        published?;
        Ok(self.intent(record))
    }
    pub fn pending(&self, cancel: &AtomicBool) -> Result<Vec<Intent>, Error> {
        check(cancel)?;
        let state = self.0.state.lock().map_err(|_| Error::Changed)?;
        self.baseline(&state, cancel)?;
        Ok(state
            .entries
            .iter()
            .cloned()
            .map(|record| self.intent(record))
            .collect())
    }
    /// Cancellation is an independent privilege-reducing transaction; use neither already-set network cancel nor deletion of PNG/original envelope.
    pub fn cancel(&self, intent: &Intent) -> Result<(), Error> {
        if !Arc::ptr_eq(&self.0, &intent.store.0) {
            return Err(Error::Changed);
        }
        // Revoke exact active requests first; disk conflicts/write failures must not let old requests continue creation.
        self.0.activity.stop(&intent.record);
        let cancel = AtomicBool::new(false);
        let mut state = self.0.state.lock().map_err(|_| Error::Changed)?;
        // activate may register between first stop and obtaining state; revoke again in state -> registry order to close that race.
        self.0.activity.stop(&intent.record);
        self.baseline(&state, &cancel)?;
        let index = state
            .entries
            .iter()
            .position(|old| old.same_intent(&intent.record))
            .ok_or(Error::Changed)?;
        if state.entries[index].cancelled {
            return Ok(());
        }
        let mut next = state.entries.clone();
        next[index].cancelled = true;
        self.persist(&mut state, next, &cancel)
    }
    fn intent(&self, record: Record) -> Intent {
        Intent {
            store: self.clone(),
            record,
        }
    }
}
impl Intent {
    pub(crate) fn scope(&self) -> &str {
        &self.record.selection.scope
    }
    pub fn envelope(&self) -> &Envelope {
        &self.record.prepared.envelope
    }
    pub fn target(&self) -> Option<&str> {
        self.record.selection.target.as_deref()
    }
    pub fn unit(&self) -> &str {
        &self.record.selection.unit
    }
    pub fn digest(&self) -> &str {
        &self.record.prepared.snapshot.digest
    }
    pub fn dimensions(&self) -> (u32, u32) {
        (
            self.record.prepared.snapshot.width,
            self.record.prepared.snapshot.height,
        )
    }
    pub fn cancelled(&self) -> bool {
        self.record.cancelled
    }
}
impl std::fmt::Debug for Intent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectionSenderIntent")
            .field("cancelled", &self.cancelled())
            .finish_non_exhaustive()
    }
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Stopped)
    } else {
        Ok(())
    }
}
fn validate_selection(
    scope: &str,
    origin: &str,
    unit: &str,
    target: Option<&str>,
) -> Result<(), Error> {
    if !envelope::digest(scope)
        || envelope::canonical_origin(origin).as_deref() != Ok(origin)
        || !envelope::identifier(unit)
        || target.is_some_and(|target| {
            !envelope::identifier(target) || target.starts_with("peer-target:")
        })
    {
        return Err(Error::Format);
    }
    Ok(())
}
#[cfg(test)]
#[path = "projection_sender_tests.rs"]
pub(crate) mod tests;
