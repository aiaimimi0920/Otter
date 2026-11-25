//! 用户从 B 恢复已召回且最小化的 A：新快照/恢复 ACK 先于再次透明后台渲染。
use super::*;
use windows::Win32::UI::WindowsAndMessaging::IsIconic;
impl Protection {
    pub fn recalled(&self) -> bool {
        self.source.recalled.get()
    }
}
impl Owner {
    /// 只由本地 B 的实际输入意图调用。false 是旧记录退休/helper 退出待结算，不重试输入边沿。
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
        // 旧记录已 ACK 退休，清理其 registry 槽；不得用旧可见快照覆盖用户的新最小化状态。
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
        // 原 Source 被全部同源 peer 共享；不另建并行 lease，不遗漏其他 B 的恢复保护。
        *source.ticket.borrow_mut() = ticket;
        *source.snapshot.borrow_mut() = Some(snapshot);
        source.armed.set(false);
        *source.activation.borrow_mut() = None;
        source.recalled.set(false);
        Ok(true)
    }
}
