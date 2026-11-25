//! UI registration coordination: at most 16 sources and one request in flight; retirement callbacks only set intent, without UI ACK waits.
use super::{
    asynchronous::{Owner, Reply},
    Operation, Status,
};
use crate::live_recovery_wire::{Record, MAX_RECORDS};
use std::{
    cell::Cell,
    io,
    os::windows::io::{AsRawHandle, BorrowedHandle},
    rc::Rc,
};
use windows::{
    core::{Error, Result as WinResult, HRESULT},
    Win32::{Foundation::*, System::Threading::WaitForSingleObject},
};

// E_PENDING; do not add an unrelated Urlmon API feature for a single HRESULT.
const RETIREMENT_PENDING: HRESULT = HRESULT(0x8000000A_u32 as i32);

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Requested,
    Registering,
    Registered,
    Retiring,
    Retired,
    Rejected,
    Unknown,
}
struct Shared {
    state: Cell<State>,
    retire: Cell<bool>,
    restore_required: Cell<bool>,
    alive: Rc<Cell<bool>>,
}
#[derive(Clone)]
pub struct Ticket(Rc<Shared>);
impl Ticket {
    pub fn available(&self) -> bool {
        self.0.alive.get()
    }
    pub fn retired(&self) -> bool {
        matches!(self.0.state.get(), State::Retired | State::Rejected)
    }
    /// Once retirement is requested, even in-flight registration stops accepting new leases.
    pub fn accepting_leases(&self) -> bool {
        self.available() && !self.0.retire.get() && !self.retired()
    }
    pub fn ready(&self) -> bool {
        self.0.alive.get() && !self.0.retire.get() && self.0.state.get() == State::Registered
    }
    pub fn rejected(&self) -> bool {
        self.0.state.get() == State::Rejected
    }
    /// Layering requests may already be claimed by the source; losing the last owner does not prove restoration, so settle explicitly before retirement.
    pub fn require_restore(&self) {
        self.0.restore_required.set(true);
    }
    /// Can directly serve as Lease::arm's retirement callback; retain the local recovery snapshot until ACK arrives.
    pub fn retire(&self) -> WinResult<()> {
        self.0.retire.set(true);
        if matches!(self.0.state.get(), State::Retired | State::Rejected) {
            Ok(())
        } else {
            Err(Error::new(
                RETIREMENT_PENDING,
                "Live recovery retirement is pending",
            ))
        }
    }
}
impl Drop for Ticket {
    fn drop(&mut self) {
        // Journal holds one reference; ordinary unborrowed tickets retire automatically, while unknown source requests must retain recovery responsibility.
        if Rc::strong_count(&self.0) == 2 && !self.0.restore_required.get() {
            self.0.retire.set(true);
        }
    }
}
struct Entry {
    id: u64,
    record: Record,
    shared: Rc<Shared>,
}
#[derive(Clone, Copy)]
enum Pending {
    Register(u64),
    Retire(u64),
    Shutdown,
}
pub struct Journal {
    owner: Owner,
    entries: Vec<Entry>,
    next_id: u64,
    pending: Option<Pending>,
    alive: Rc<Cell<bool>>,
    fault: Option<String>,
    closing: bool,
    shutdown: bool,
    stop_requested: bool,
    finished: bool,
}
impl Journal {
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture() -> Self {
        Self::with_owner(
            Owner::start_with(|| super::Client::start_command(super::fixture_command("serve")))
                .unwrap(),
        )
    }
    pub fn start() -> io::Result<Self> {
        Ok(Self::with_owner(Owner::start()?))
    }
    fn with_owner(owner: Owner) -> Self {
        Self {
            owner,
            entries: Vec::new(),
            next_id: 1,
            pending: None,
            alive: Rc::new(Cell::new(true)),
            fault: None,
            closing: false,
            shutdown: false,
            stop_requested: false,
            finished: false,
        }
    }
    pub fn register(&mut self, record: Record) -> io::Result<Ticket> {
        record.validate()?;
        if self.closing || self.fault.is_some() || self.entries.len() >= MAX_RECORDS {
            return Err(io::Error::other("Recovery journal unavailable or full"));
        }
        if self
            .entries
            .iter()
            .any(|entry| entry.record.hwnd == record.hwnd)
        {
            return Err(io::Error::other("Recovery source is already registered"));
        }
        let next = self
            .next_id
            .checked_add(1)
            .ok_or_else(|| io::Error::other("Recovery registration id exhausted"))?;
        self.entries.try_reserve(1).map_err(io::Error::other)?;
        let shared = Rc::new(Shared {
            state: Cell::new(State::Requested),
            retire: Cell::new(false),
            restore_required: Cell::new(false),
            alive: self.alive.clone(),
        });
        self.entries.push(Entry {
            id: self.next_id,
            record,
            shared: shared.clone(),
        });
        self.next_id = next;
        Ok(Ticket(shared))
    }
    pub fn completion_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.owner.completion_handle()
    }
    pub fn child_handle(&self) -> Option<BorrowedHandle<'_>> {
        if self.finished {
            None
        } else {
            self.owner.child_handle()
        }
    }
    pub fn fault(&self) -> Option<&str> {
        self.fault.as_deref()
    }
    pub fn finished(&self) -> bool {
        self.finished
    }
    /// Do not implicitly retire still-protected sources; callers first restore each source and request retirement, then wait for shutdown completion.
    pub fn close(&mut self) {
        self.closing = true;
        self.alive.set(false);
        for entry in &self.entries {
            if matches!(
                entry.shared.state.get(),
                State::Requested | State::Registering
            ) {
                entry.shared.retire.set(true);
            }
        }
    }
    pub fn poll(&mut self) {
        if self.finished {
            return;
        }
        if let Some(result) = self.owner.poll() {
            match result {
                Ok(Reply::Started) => {}
                Ok(Reply::Completed(status)) => self.complete(status),
                Err(error) => self.fail(error.to_string()),
            }
        }
        if self.owner.busy() {
            return;
        }
        if self.owner.child_handle().is_some_and(|handle| unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 0) } == WAIT_OBJECT_0) {
            if !self.shutdown { self.fail("Recovery process exited".into()); }
            // The recovery process is confirmed absent and cannot replay old snapshots; local windows remain the lease's restoration responsibility.
            for entry in &self.entries { entry.shared.state.set(State::Retired); }
            self.finished = true;
            return;
        }
        if self.fault.is_some() {
            // For sent records, only explicit retirement intent proves the caller restored/canceled; a broken channel must never
            // directly kill a recovery process still protecting hidden windows. Retain local snapshots until exit is confirmed.
            let restored = self.entries.iter().all(|entry| {
                entry.shared.retire.get()
                    || matches!(
                        entry.shared.state.get(),
                        State::Requested | State::Retired | State::Rejected
                    )
            });
            if restored && !self.stop_requested && self.owner.child_handle().is_some() {
                match self.owner.stop_after_restore() {
                    Ok(()) => self.stop_requested = true,
                    Err(error) => {
                        self.fault = Some(format!("Recovery stop after restore failed: {error}"));
                    }
                }
            }
            if self.owner.child_handle().is_none() {
                for entry in &self.entries {
                    // Only never-sent requests may be canceled directly. A worker panic may lose Client,
                    // but a missing handle does not mean the recovery child exited; Unknown responsibility cannot be retired on that basis.
                    if entry.shared.state.get() == State::Requested {
                        entry.shared.state.set(State::Retired);
                    }
                }
                self.finished = self.entries.iter().all(|entry| {
                    matches!(entry.shared.state.get(), State::Retired | State::Rejected)
                });
            }
            return;
        }
        if self.shutdown {
            return;
        }
        match self.owner.healthy() {
            Ok(true) => {}
            _ => {
                self.fail("Recovery journal is unavailable".into());
                return;
            }
        }
        for entry in &self.entries {
            if entry.shared.retire.get() && entry.shared.state.get() == State::Requested {
                entry.shared.state.set(State::Retired);
            }
        }
        self.entries
            .retain(|entry| !matches!(entry.shared.state.get(), State::Retired | State::Rejected));
        let action = self
            .entries
            .iter()
            .find(|entry| {
                entry.shared.retire.get() && entry.shared.state.get() == State::Registered
            })
            .map(|entry| {
                (
                    Pending::Retire(entry.id),
                    Operation::Retire { id: entry.id },
                )
            })
            .or_else(|| {
                self.entries
                    .iter()
                    .find(|entry| entry.shared.state.get() == State::Requested)
                    .map(|entry| {
                        (
                            Pending::Register(entry.id),
                            Operation::Register {
                                id: entry.id,
                                record: entry.record.clone(),
                            },
                        )
                    })
            })
            .or_else(|| {
                (self.closing && self.entries.is_empty())
                    .then_some((Pending::Shutdown, Operation::Shutdown))
            });
        if let Some((pending, command)) = action {
            if let Err(error) = self.owner.request(command) {
                self.fail(error.to_string());
                return;
            }
            for entry in &self.entries {
                match pending {
                    Pending::Register(id) if id == entry.id => {
                        entry.shared.state.set(State::Registering)
                    }
                    Pending::Retire(id) if id == entry.id => {
                        entry.shared.state.set(State::Retiring)
                    }
                    _ => {}
                }
            }
            self.pending = Some(pending);
        }
    }
    fn complete(&mut self, status: Status) {
        let Some(pending) = self.pending.take() else {
            self.fail("Unexpected recovery acknowledgement".into());
            return;
        };
        match pending {
            Pending::Shutdown if status == Status::Accepted => {
                self.shutdown = true;
                self.alive.set(false);
            }
            Pending::Shutdown => self.fail("Recovery shutdown rejected".into()),
            Pending::Register(id) => {
                if let Some(entry) = self.entries.iter().find(|entry| entry.id == id) {
                    entry.shared.state.set(if status == Status::Accepted {
                        State::Registered
                    } else {
                        State::Rejected
                    });
                }
            }
            Pending::Retire(id) => {
                if status == Status::Rejected {
                    self.fail("Recovery retirement rejected".into());
                } else if let Some(entry) = self.entries.iter().find(|entry| entry.id == id) {
                    entry.shared.state.set(State::Retired);
                }
            }
        }
    }
    fn fail(&mut self, error: String) {
        self.alive.set(false);
        self.fault.get_or_insert(error);
        for entry in &self.entries {
            if matches!(
                entry.shared.state.get(),
                State::Registering | State::Retiring
            ) {
                entry.shared.state.set(State::Unknown);
            }
        }
    }
}

impl Drop for Journal {
    fn drop(&mut self) {
        // Even if an external Ticket remains alive, losing the coordination owner prevents further authorization to hide.
        self.alive.set(false);
    }
}

#[cfg(test)]
#[path = "live_recovery_journal_tests.rs"]
mod tests;
