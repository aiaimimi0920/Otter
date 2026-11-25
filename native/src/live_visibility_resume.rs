//! User restores recalled/minimized A from B: fresh snapshot/restoration ACK precede transparent background rendering again.
use super::*;
use windows::Win32::UI::WindowsAndMessaging::IsIconic;
impl Protection {
    pub fn recalled(&self) -> bool {
        self.source.recalled.get()
    }
}
impl Owner {
    /// Call only for actual input intent on local B. false means old-record retirement/helper exit awaits settlement, never input-edge retry.
    pub fn resume_minimized(&mut self, protection: &Protection) -> io::Result<bool> {
        if self.closing {
            return Err(io::Error::other("Live recovery is closing"));
        }
        let source = &protection.source;
        if !self.sources.iter().any(|owned| Rc::ptr_eq(owned, source)) {
            return Err(io::Error::other("Live recovery source is not owned"));
        }
        source.target.validate().map_err(io::Error::other)?;
        if !source.recalled.get() || !unsafe { IsIconic(source.target.hwnd()) }.as_bool() {
            return Ok(true);
        }
        if self.draining || !source.ticket.borrow().retired() {
            return Ok(false);
        }
        // Old-record retirement is ACKed; clear its registry slot without overwriting the user's new minimized state with an old visible snapshot.
        self.registry
            .restore_source(&source.target)
            .map_err(io::Error::other)?;
        let snapshot = Snapshot::read(&source.target).map_err(io::Error::other)?;
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
            #[cfg(test)]
            let journal = if self.simulated_input {
                Journal::fixture()
            } else {
                Journal::start()?
            };
            #[cfg(not(test))]
            let journal = Journal::start()?;
            self.journal = Some(journal);
        }
        let ticket = self.journal.as_mut().unwrap().register(record)?;
        // All same-source peers share original Source; no separate parallel lease or omission of other B windows' restoration protection.
        *source.ticket.borrow_mut() = ticket;
        *source.snapshot.borrow_mut() = Some(snapshot);
        source.armed.set(false);
        *source.activation.borrow_mut() = None;
        source.recalled.set(false);
        Ok(true)
    }
}
