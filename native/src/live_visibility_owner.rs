//! Application-wide source-visibility coordination: shared same-source registration, Session leases, and separate background ACK/UI restoration.
use crate::{
    live_reclaim::{Activation, Intent},
    live_recovery_client::journal::{Journal, Ticket},
    live_window_target::{
        snapshot::Snapshot,
        visibility::{retain_restore_error, Lease, Registry, RETIREMENT_PENDING},
        Target,
    },
};
use std::{
    cell::{Cell, RefCell},
    io,
    os::windows::io::BorrowedHandle,
    rc::Rc,
};

struct Source {
    target: Target,
    snapshot: RefCell<Option<Snapshot>>,
    ticket: RefCell<Ticket>,
    armed: Cell<bool>,
    recalled: Cell<bool>,
    activation: RefCell<Option<Activation>>,
}
pub struct Protection {
    // Destroy lease first, completing restoration/recording retirement intent before releasing the Source reference.
    lease: Lease,
    source: Rc<Source>,
}
impl Protection {
    pub fn poll(&self) -> io::Result<()> {
        let source = &self.source;
        // Explicit recall exits only the hiding transaction, not capture sessions; no further hiding after helper retirement/close.
        if source.recalled.get() {
            return Ok(());
        }
        let registration = {
            let ticket = source.ticket.borrow();
            if !ticket.available() || ticket.rejected() {
                return Err(io::Error::other("Live recovery registration unavailable"));
            }
            if !ticket.ready() {
                return Ok(());
            }
            source
                .snapshot
                .borrow_mut()
                .take()
                .map(|snapshot| (snapshot, ticket.clone()))
        }; // Win32 hiding/rollback may synchronously enter window procedures; do not borrow ticket across calls.
        if let Some((snapshot, ticket)) = registration {
            self.lease
                .arm(snapshot, move || ticket.retire())
                .map_err(io::Error::other)?;
            source.armed.set(true);
        }
        self.lease.hide().map_err(io::Error::other)?;
        Ok(())
    }
}

#[path = "live_visibility_popup_layers.rs"]
pub mod popup_layers;
#[path = "live_visibility_resume.rs"]
pub mod resume;

#[derive(Default)]
pub struct Owner {
    popup_layers: popup_layers::Layers,
    watcher: Option<crate::live_reclaim_watch::Watcher>,
    #[cfg(test)]
    simulated_input: bool,
    registry: Registry,
    sources: Vec<Rc<Source>>,
    journal: Option<Journal>,
    closing: bool,
    draining: bool,
}
impl Owner {
    #[cfg(test)]
    pub fn fixture() -> Self {
        Self {
            journal: Some(Journal::fixture()),
            simulated_input: true,
            ..Default::default()
        }
    }
    pub fn acquire(&mut self, target: Target) -> io::Result<Protection> {
        if self.closing || self.draining {
            return Err(io::Error::other("Live recovery is closing"));
        }
        target.validate().map_err(io::Error::other)?;
        let existing = self
            .sources
            .iter()
            .find(|source| source.target.same_source(&target));
        // Reject old registrations with retirement requested but no ACK yet; never add consumers to them again.
        if existing.is_some_and(|source| !source.ticket.borrow().accepting_leases()) {
            return Err(io::Error::other("Live recovery source is retiring"));
        }
        let lease = self
            .registry
            .acquire(target.clone())
            .map_err(io::Error::other)?;
        if let Some(source) = existing {
            return Ok(Protection {
                lease,
                source: source.clone(),
            });
        }
        if self.sources.len() >= 16 {
            return Err(io::Error::other("Live recovery source limit reached"));
        }
        self.sources.try_reserve(1).map_err(io::Error::other)?;
        let snapshot = Snapshot::read(&target).map_err(io::Error::other)?;
        let record = snapshot.recovery_record().map_err(io::Error::other)?;
        #[cfg(test)]
        let watch = !self.simulated_input;
        #[cfg(not(test))]
        let watch = true;
        if watch && self.watcher.is_none() {
            self.watcher =
                Some(crate::live_reclaim_watch::Watcher::install().map_err(io::Error::other)?);
        }
        if self.journal.is_none() {
            self.journal = Some(Journal::start()?);
        }
        let ticket = self.journal.as_mut().unwrap().register(record)?;
        let source = Rc::new(Source {
            target,
            snapshot: RefCell::new(Some(snapshot)),
            ticket: RefCell::new(ticket),
            armed: Cell::new(false),
            recalled: Cell::new(false),
            activation: RefCell::new(None),
        });
        self.sources.push(source.clone());
        Ok(Protection { lease, source })
    }
    pub fn poll(&mut self) -> io::Result<()> {
        if let Some(watcher) = &self.watcher {
            let (foreground, current, now) = watcher.snapshot().map_err(io::Error::other)?;
            self.apply_reclaim(foreground, current, now, |target| unsafe {
                windows::Win32::UI::WindowsAndMessaging::IsIconic(target.hwnd()).as_bool()
            });
        }
        if let Some(journal) = &mut self.journal {
            journal.poll();
        }
        // Failed restoration of the last lease cannot directly retire; registry continues retrying the original snapshot.
        let mut restore = self.restore_pending();
        for source in &self.sources {
            if Rc::strong_count(source) == 1 && !source.armed.get() {
                let _ = source.ticket.borrow().retire();
            }
        }
        if restore.is_ok() {
            self.sources.retain(|source| {
                Rc::strong_count(source) != 1 || !source.ticket.borrow().retired()
            });
        }
        if self
            .sources
            .iter()
            .all(|source| source.ticket.borrow().retired())
            && self.popup_layers.is_empty()
        {
            if let Some(journal) = &mut self.journal {
                journal.close();
                self.draining = true;
            }
        }
        if let Some(journal) = &mut self.journal {
            journal.poll();
        }
        if self.journal.as_ref().is_some_and(Journal::finished) {
            // The second poll may just confirm child exit; settle newly completed retirement first to avoid
            // remaining local entries without wait handles, forcing the host to fail exit or wait idly for another UI message.
            let previous = restore
                .err()
                .filter(|error| error.code() != RETIREMENT_PENDING);
            restore = self.restore_pending();
            if let Some(error) = previous {
                // Later successful settlement cannot erase a real restoration failure already observed this round.
                restore = Err(error);
            }
            if restore.is_ok() {
                self.sources.retain(|source| {
                    Rc::strong_count(source) != 1 || !source.ticket.borrow().retired()
                });
            }
            if self
                .sources
                .iter()
                .all(|source| source.ticket.borrow().retired())
                && self.popup_layers.is_empty()
                && restore.is_ok()
            {
                self.journal.take();
                self.draining = false;
            }
        }
        if self.sources.iter().all(|source| source.recalled.get()) {
            self.watcher.take();
        }
        // E_PENDING means asynchronous retirement wait, not window-restoration failure.
        match restore {
            Err(error) if error.code() != RETIREMENT_PENDING => Err(io::Error::other(error)),
            _ => Ok(()),
        }
    }
    fn restore_pending(&mut self) -> windows::core::Result<()> {
        let mut first = self.registry.retry_pending().err();
        if let Err(error) = self.popup_layers.poll(self.closing) {
            retain_restore_error(&mut first, error);
        }
        for source in &self.sources {
            if source.recalled.get() && source.armed.get() && Rc::strong_count(source) > 1 {
                if let Err(error) = self.registry.restore_source(&source.target) {
                    retain_restore_error(&mut first, error);
                }
            }
        }
        first.map_or(Ok(()), Err)
    }
    /// Input watchers pass the latest, possibly cancelled, intent snapshot; foreground changes alone do not authorize recall.
    /// Initial observation absorbs old intent and must occur before hiding starts; source events drive later calls.
    pub fn observe_reclaim(
        &mut self,
        foreground: usize,
        current: Option<Intent>,
        now_ms: u64,
        iconic: impl FnMut(&Target) -> bool,
    ) -> io::Result<()> {
        self.apply_reclaim(foreground, current, now_ms, iconic);
        self.poll()
    }
    fn apply_reclaim(
        &mut self,
        foreground: usize,
        current: Option<Intent>,
        now_ms: u64,
        mut iconic: impl FnMut(&Target) -> bool,
    ) {
        for source in &self.sources {
            if source.recalled.get() {
                continue;
            }
            let reclaim = {
                let mut activation = source.activation.borrow_mut();
                if !source.armed.get() {
                    *activation = Some(Activation::new(current, foreground));
                }
                let state = activation.get_or_insert_with(|| Activation::new(current, foreground));
                source.armed.get()
                    && state.observe(
                        foreground,
                        source.target.hwnd().0 as usize,
                        iconic(&source.target),
                        current,
                        now_ms,
                    )
            };
            if reclaim {
                source.recalled.set(true);
                // If hiding has not actually happened, later polls no longer attempt it; restoration/retirement may remain asynchronously pending.
            }
        }
    }
    pub fn close(&mut self) {
        self.watcher.take();
        self.closing = true;
        if let Some(journal) = &mut self.journal {
            journal.close();
        }
    }
    pub fn finished(&self) -> bool {
        self.sources.is_empty() && self.popup_layers.is_empty() && self.journal.is_none()
    }
    pub fn completion_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.journal.as_ref().and_then(Journal::completion_handle)
    }
    pub fn deadline(&self) -> Option<std::time::Instant> {
        self.popup_layers.deadline()
    }
    pub fn input_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.watcher
            .as_ref()
            .map(crate::live_reclaim_watch::Watcher::handle)
    }
    pub fn child_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.journal.as_ref().and_then(Journal::child_handle)
    }
}

#[cfg(test)]
#[path = "live_visibility_owner_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "live_visibility_runtime_tests.rs"]
mod runtime_tests;
