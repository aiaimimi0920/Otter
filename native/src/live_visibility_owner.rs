//! 应用级源可见性协调：同源共享登记，Session 持 lease，后台 ACK 与 UI 恢复分离。
use crate::{
    live_reclaim::{Activation, Intent},
    live_recovery_client::journal::{Journal, Ticket},
    live_window_target::{
        snapshot::Snapshot,
        visibility::{Lease, Registry},
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
    ticket: Ticket,
    armed: Cell<bool>,
    recalled: Cell<bool>,
    activation: RefCell<Option<Activation>>,
}
pub struct Protection {
    // lease 必须先析构，完成恢复/设置退休意图后才放弃 Source 引用。
    lease: Lease,
    source: Rc<Source>,
}
impl Protection {
    pub fn poll(&self) -> io::Result<()> {
        let source = &self.source;
        // 显式召回只退出隐藏事务，不释放采集会话；退休/关闭 helper 后也不能再隐藏。
        if source.recalled.get() {
            return Ok(());
        }
        if !source.ticket.available() || source.ticket.rejected() {
            return Err(io::Error::other("Live recovery registration unavailable"));
        }
        if !source.ticket.ready() {
            return Ok(());
        }
        let snapshot = source.snapshot.borrow_mut().take();
        if let Some(snapshot) = snapshot {
            let ticket = source.ticket.clone();
            self.lease
                .arm(snapshot, move || ticket.retire())
                .map_err(io::Error::other)?;
            source.armed.set(true);
        }
        self.lease.hide().map_err(io::Error::other)?;
        Ok(())
    }
}

#[derive(Default)]
pub struct Owner {
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
    pub(crate) fn fixture() -> Self {
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
        let lease = self
            .registry
            .acquire(target.clone())
            .map_err(io::Error::other)?;
        if let Some(source) = self
            .sources
            .iter()
            .find(|source| source.target.same_source(&target))
        {
            if source.ticket.retired() || !source.ticket.available() {
                return Err(io::Error::other("Live recovery source is retiring"));
            }
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
            ticket,
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
        // 最后 lease 恢复失败时不能直接 retire；继续由 registry 重试原快照。
        let mut restore = self.restore_pending();
        for source in &self.sources {
            if Rc::strong_count(source) == 1 && !source.armed.get() {
                let _ = source.ticket.retire();
            }
        }
        if restore.is_ok() {
            self.sources
                .retain(|source| Rc::strong_count(source) != 1 || !source.ticket.retired());
        }
        if self.sources.iter().all(|source| source.ticket.retired()) {
            if let Some(journal) = &mut self.journal {
                journal.close();
                self.draining = true;
            }
        }
        if let Some(journal) = &mut self.journal {
            journal.poll();
        }
        if self.journal.as_ref().is_some_and(Journal::finished) {
            // 本次第二次 poll 可能刚确认子进程退出；先结算新完成的退休，避免
            // 剩下本地条目却已无等待句柄，宿主只能错误退出或空等下一条 UI 消息。
            restore = self.restore_pending();
            if restore.is_ok() {
                self.sources
                    .retain(|source| Rc::strong_count(source) != 1 || !source.ticket.retired());
            }
            if self.sources.iter().all(|source| source.ticket.retired()) && restore.is_ok() {
                self.journal.take();
                self.draining = false;
            }
        }
        if self.sources.iter().all(|source| source.recalled.get()) {
            self.watcher.take();
        }
        // E_PENDING 是异步退休等待，不是窗口恢复失败。
        match restore {
            Err(error) if error.code().0 != 0x8000000A_u32 as i32 => Err(io::Error::other(error)),
            _ => Ok(()),
        }
    }
    fn restore_pending(&mut self) -> windows::core::Result<()> {
        let mut first = self.registry.retry_pending().err();
        for source in &self.sources {
            if source.recalled.get() && source.armed.get() && Rc::strong_count(source) > 1 {
                if let Err(error) = self.registry.restore_source(&source.target) {
                    first.get_or_insert(error);
                }
            }
        }
        first.map_or(Ok(()), Err)
    }
    /// 输入 watcher 传入最新（可已取消）的意图快照；前台变化本身不授予召回。
    /// 初次观察吸收旧意图，必须在隐藏开始前调用，之后由源事件驱动调用。
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
                // 若隐藏尚未真正发生，后续 poll 也不再尝试隐藏；恢复/退休允许异步待定。
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
        self.sources.is_empty() && self.journal.is_none()
    }
    pub fn completion_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.journal.as_ref().and_then(Journal::completion_handle)
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
