//! Edit write-ahead journal: per-profile exclusivity, full disk CAS, single in-flight request; network replies never automatically confirm workspace saving.
use super::{Document, Objects, Request, Route};
use crate::{pixels::Pixels, session_projection_scope::Scope};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
#[path = "projection_edit_journal_file.rs"]
mod file;
#[path = "projection_edit_journal_record.rs"]
mod record;
#[path = "projection_edit_journal_rejection.rs"]
mod rejection;
#[path = "projection_edit_journal_source.rs"]
mod source;
use record::{Entry, Ledger, Pending};
pub const MAX_ENTRIES: usize = 64;
pub const MAX_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_ENTRY: usize = 8 * 1024 * 1024;
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
    Busy,
    Stopped,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Context {
    scope: Scope,
    unit: String,
    origin: String,
    device: String,
}
impl Context {
    pub fn new(scope: Scope, unit: &str, origin: &str, device: &str) -> Result<Self, Error> {
        let value = Self {
            scope,
            unit: unit.into(),
            origin: origin.into(),
            device: device.into(),
        };
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<(), Error> {
        if !super::identifier(&self.unit)
            || !super::identifier(&self.device)
            || crate::device_session_attempt::canonical_origin(&self.origin)
                .ok()
                .as_deref()
                != Some(self.origin.as_str())
        {
            return Err(Error::Format);
        }
        Ok(())
    }
    pub(crate) fn same_key(&self, other: &Self) -> bool {
        self.scope == other.scope && self.unit == other.unit && self.origin == other.origin
    }
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn device(&self) -> &str {
        &self.device
    }
    pub fn unit(&self) -> &str {
        &self.unit
    }
    pub fn scope(&self) -> Scope {
        self.scope
    }
}
struct State {
    original: Option<Pixels>,
    entries: Vec<Entry>,
    retained: bool,
}
struct Inner {
    path: PathBuf,
    state: Mutex<State>,
    active: AtomicBool,
    _lease: File,
    _directories: Vec<File>,
}
#[derive(Clone)]
pub struct Store(Arc<Inner>);
pub struct Ticket {
    store: Store,
    entry: Entry,
}
pub(crate) struct Active<'a>(&'a Store);
impl Drop for Active<'_> {
    fn drop(&mut self) {
        self.0 .0.active.store(false, Ordering::Release);
    }
}
impl Store {
    /// Background only; no discovery/creation of real profiles or environment-variable reads.
    pub fn open(root: &Path, cancel: &AtomicBool) -> Result<Self, Error> {
        file::open(root, cancel)
    }
    /// Persist the complete immutable request/local view first; reentry reuses only identical pending state without replacing unknown side effects.
    pub fn begin(
        &self,
        context: Context,
        route: Route,
        request: Request,
        previous: Option<Document>,
        view: Objects,
        cancel: &AtomicBool,
    ) -> Result<Ticket, Error> {
        self.begin_checked(
            context,
            Pending {
                route,
                request,
                previous,
                view,
                rejected: None,
            },
            cancel,
            false,
        )
    }
    /// The same session already read by the product may advance no-pending baselines; never change session/basis or overwrite unknown network effects.
    pub(crate) fn begin_observed(
        &self,
        context: Context,
        route: Route,
        request: Request,
        previous: Document,
        view: Objects,
        cancel: &AtomicBool,
    ) -> Result<Ticket, Error> {
        self.begin_checked(
            context,
            Pending {
                route,
                request,
                previous: Some(previous),
                view,
                rejected: None,
            },
            cancel,
            true,
        )
    }
    fn begin_checked(
        &self,
        context: Context,
        pending: Pending,
        cancel: &AtomicBool,
        observed: bool,
    ) -> Result<Ticket, Error> {
        if pending.request.is_read() {
            return Err(Error::Format);
        }
        pending.validate()?;
        context.validate()?;
        let mut state = self.0.state.lock().map_err(|_| Error::Changed)?;
        self.baseline(&state, cancel)?;
        let mut entries = state.entries.clone();
        let entry = if let Some(old) = entries.iter_mut().find(|e| e.context.same_key(&context)) {
            if old.context != context {
                return Err(Error::Changed);
            }
            if let Some(existing) = &old.pending {
                if existing != &pending {
                    return Err(Error::Busy);
                }
                return Ok(Ticket {
                    store: self.clone(),
                    entry: old.clone(),
                });
            }
            if old.document != pending.previous {
                if !observed
                    || !matches!((&pending.previous, &old.document),
                    (Some(next), Some(before)) if next.follows(before).is_ok())
                {
                    return Err(Error::Changed);
                }
                old.document = pending.previous.clone();
            }
            old.generation = old
                .generation
                .checked_add(1)
                .filter(|v| super::revision(*v))
                .ok_or(Error::Limit)?;
            old.pending = Some(pending);
            old.clone()
        } else {
            if entries.len() >= MAX_ENTRIES {
                return Err(Error::Limit);
            }
            let entry = Entry {
                context,
                generation: 1,
                document: pending.previous.clone(),
                pending: Some(pending),
                source: None,
            };
            entries.push(entry.clone());
            entry
        };
        self.persist(&mut state, entries, cancel)?;
        Ok(Ticket {
            store: self.clone(),
            entry,
        })
    }
    /// Return original requests awaiting recovery; no networking/automatic replay or overwriting current workspaces with old views.
    pub fn pending(&self, cancel: &AtomicBool) -> Result<Vec<Ticket>, Error> {
        let state = self.0.state.lock().map_err(|_| Error::Changed)?;
        self.baseline(&state, cancel)?;
        Ok(state
            .entries
            .iter()
            .filter(|e| e.pending.is_some())
            .map(|entry| Ticket {
                store: self.clone(),
                entry: entry.clone(),
            })
            .collect())
    }
    /// Match the logical key, not the grant: callers must not mistake another
    /// device's pending operation for an empty slot.
    pub(crate) fn pending_for(
        &self,
        context: &Context,
        cancel: &AtomicBool,
    ) -> Result<Option<Ticket>, Error> {
        let state = self.0.state.lock().map_err(|_| Error::Changed)?;
        self.baseline(&state, cancel)?;
        Ok(state
            .entries
            .iter()
            .find(|entry| entry.pending.is_some() && entry.context.same_key(context))
            .map(|entry| Ticket {
                store: self.clone(),
                entry: entry.clone(),
            }))
    }
    /// Accept validated reads only without pending; missing initialization journals cannot infer original local edits from reads.
    pub fn observe(
        &self,
        context: &Context,
        document: Document,
        cancel: &AtomicBool,
    ) -> Result<(), Error> {
        document.validate().map_err(|_| Error::Format)?;
        let mut state = self.0.state.lock().map_err(|_| Error::Changed)?;
        self.baseline(&state, cancel)?;
        let mut entries = state.entries.clone();
        let old = entries
            .iter_mut()
            .find(|e| e.context == *context)
            .ok_or(Error::Changed)?;
        if old.pending.is_some() {
            return Err(Error::Busy);
        }
        document
            .follows(old.document.as_ref().ok_or(Error::Changed)?)
            .map_err(|_| Error::Changed)?;
        old.document = Some(document);
        self.persist(&mut state, entries, cancel)
    }
}
impl Ticket {
    fn pending_record(&self) -> &Pending {
        self.entry.pending.as_ref().expect("ticket owns pending")
    }
    pub fn context(&self) -> &Context {
        &self.entry.context
    }
    pub fn request(&self) -> &Request {
        &self.pending_record().request
    }
    pub fn route(&self) -> Route {
        self.pending_record().route
    }
    pub fn previous(&self) -> Option<&Document> {
        self.pending_record().previous.as_ref()
    }
    pub fn view(&self) -> &Objects {
        &self.pending_record().view
    }
    pub(crate) fn check(&self, cancel: &AtomicBool) -> Result<(), Error> {
        let state = self.store.0.state.lock().map_err(|_| Error::Changed)?;
        self.store.baseline(&state, cancel)?;
        if !state.entries.contains(&self.entry) {
            return Err(Error::Changed);
        }
        Ok(())
    }
    pub(crate) fn activate(&self, cancel: &AtomicBool) -> Result<Active<'_>, Error> {
        self.store
            .0
            .active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| Error::Busy)?;
        let active = Active(&self.store);
        self.check(cancel)?;
        Ok(active)
    }
    /// Callers first verify current Workspace/exact canonical checkpoint; HTTP success alone cannot invoke this entry.
    /// Failure/conflict/cancellation never calls this method; original pending/view remain for explicit recovery.
    pub fn commit_after_checkpoint(
        &self,
        document: Document,
        cancel: &AtomicBool,
    ) -> Result<(), Error> {
        if self.rejection().is_some() {
            return Err(Error::Changed);
        }
        let bytes = serde_json::to_vec(&document).map_err(|_| Error::Format)?;
        self.request()
            .response(&bytes, self.previous())
            .map_err(|_| Error::Changed)?;
        let mut state = self.store.0.state.lock().map_err(|_| Error::Changed)?;
        self.store.baseline(&state, cancel)?;
        let mut entries = state.entries.clone();
        let old = entries
            .iter_mut()
            .find(|e| **e == self.entry)
            .ok_or(Error::Changed)?;
        old.document = Some(document);
        old.pending = None;
        self.store.persist(&mut state, entries, cancel)
    }
}
impl std::fmt::Debug for Ticket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectionEditTicket")
            .field("unit", &self.context().unit())
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
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "projection_edit_journal_tests.rs"]
mod tests;
