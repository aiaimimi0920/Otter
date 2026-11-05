//! UI 线程共享源可见性 lease；独立于帧消费者计数，arm 后才可隐藏，不自行创建恢复 journal。
use super::{snapshot::Snapshot, Target};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_INVALIDARG, E_OUTOFMEMORY},
};

const LIMIT: usize = 16;
struct Recovery {
    snapshot: Snapshot,
    retire: Box<dyn Fn() -> Result<()>>,
    restored: Cell<bool>,
    hide_attempted: Cell<bool>,
    hidden: Cell<bool>,
}
struct Entry {
    target: Target,
    leases: Cell<usize>,
    snapshot: RefCell<Option<Rc<Recovery>>>,
    busy: Cell<bool>,
}
struct Busy<'a>(&'a Cell<bool>);
impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
impl Entry {
    fn restore(&self) -> Result<()> {
        if self.busy.replace(true) {
            return Err(Error::new(E_ABORT, "Live visibility operation reentered"));
        }
        let _busy = Busy(&self.busy);
        let snapshot = self.snapshot.borrow().clone();
        if let Some(snapshot) = snapshot {
            // 不跨 Win32 调用持有 RefCell 借用，允许同步消息重入后保守拒绝。
            if !snapshot.restored.get() {
                snapshot.snapshot.restore_or_retire()?;
                snapshot.restored.set(true);
            }
            // 恢复记录退休也必须成功；不能清空本地状态却留下可被 watchdog 重放的旧记录。
            (snapshot.retire)()?;
            self.snapshot.borrow_mut().take();
        }
        Ok(())
    }
}

/// 一个应用 UI owner 持有一份 registry；Rc 禁止将窗口恢复职责转移到后台线程。
#[derive(Default)]
pub struct Registry {
    entries: Vec<Rc<Entry>>,
}
impl Registry {
    /// 显式召回不减 lease 引用；恢复后仍需等待登记退休，供应用级源 owner 重试。
    pub(crate) fn restore_source(&self, target: &Target) -> Result<()> {
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.target.same_source(target))
            .ok_or_else(|| Error::new(E_INVALIDARG, "Live visibility source is not owned"))?;
        entry.restore()
    }
    pub fn acquire(&mut self, target: Target) -> Result<Lease> {
        target.validate()?;
        self.prune();
        if let Some(entry) = self
            .entries
            .iter()
            .find(|entry| entry.target.identity == target.identity)
        {
            entry.target.validate()?;
            if entry.busy.get()
                || entry
                    .snapshot
                    .borrow()
                    .as_ref()
                    .is_some_and(|record| entry.leases.get() == 0 || record.restored.get())
            {
                return Err(Error::new(E_ABORT, "Live visibility operation in progress"));
            }
            if entry.leases.get() >= LIMIT {
                return Err(Error::new(
                    E_OUTOFMEMORY,
                    "Live visibility lease limit reached",
                ));
            }
            entry.leases.set(entry.leases.get() + 1);
            return Ok(Lease(entry.clone()));
        }
        if self.entries.len() >= LIMIT {
            return Err(Error::new(
                E_OUTOFMEMORY,
                "Live visibility source limit reached",
            ));
        }
        self.entries
            .try_reserve(1)
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot allocate Live visibility source"))?;
        let entry = Rc::new(Entry {
            target,
            leases: Cell::new(1),
            snapshot: RefCell::new(None),
            busy: Cell::new(false),
        });
        self.entries.push(entry.clone());
        Ok(Lease(entry))
    }
    /// 重试没有会话但仍有恢复快照的源；失败继续保留，返回首个错误而非静默丢弃。
    pub fn retry_pending(&mut self) -> Result<()> {
        let mut first = None;
        for entry in &self.entries {
            if entry.leases.get() == 0 {
                if let Err(error) = entry.restore() {
                    first.get_or_insert(error);
                }
            }
        }
        self.prune();
        first.map_or(Ok(()), Err)
    }
    fn prune(&mut self) {
        self.entries
            .retain(|entry| entry.leases.get() != 0 || entry.snapshot.borrow().is_some());
    }
}

pub struct Lease(Rc<Entry>);
impl Lease {
    /// 仅在独立恢复通道已登记成功后、首次修改窗口前调用。此方法本身不保证崩溃恢复。
    /// 第二个 peer 不覆盖首次快照；返回 false 表示已有共享恢复状态。
    pub fn arm(
        &self,
        snapshot: Snapshot,
        retire: impl Fn() -> Result<()> + 'static,
    ) -> Result<bool> {
        if self.0.busy.get() {
            return Err(Error::new(E_ABORT, "Live visibility operation in progress"));
        }
        self.0.target.validate()?;
        snapshot.target.validate()?;
        if snapshot.target.identity != self.0.target.identity {
            return Err(Error::new(
                E_INVALIDARG,
                "Live visibility snapshot belongs to another source",
            ));
        }
        let mut saved = self.0.snapshot.borrow_mut();
        if saved.as_ref().is_some_and(|record| record.restored.get()) {
            return Err(Error::new(E_ABORT, "Live recovery retirement is pending"));
        }
        if saved.is_some() {
            return Ok(false);
        }
        *saved = Some(Rc::new(Recovery {
            snapshot,
            retire: Box::new(retire),
            restored: Cell::new(false),
            hide_attempted: Cell::new(false),
            hidden: Cell::new(false),
        }));
        Ok(true)
    }
    pub fn has_recovery(&self) -> bool {
        self.0.snapshot.borrow().is_some()
    }
    pub fn peer_count(&self) -> usize {
        self.0.leases.get()
    }
    /// arm 必须基于独立恢复 ACK。false 表示同源 peer 已隐藏，不覆盖首次快照。
    /// 部分失败尝试回滚；回滚/退休失败保留状态，禁止在未知状态上再次隐藏。
    pub fn hide(&self) -> Result<bool> {
        if self.0.busy.replace(true) {
            return Err(Error::new(E_ABORT, "Live visibility operation reentered"));
        }
        let busy = Busy(&self.0.busy);
        let snapshot = self.0.snapshot.borrow().clone().ok_or_else(|| {
            Error::new(E_ABORT, "Live source hide requires recovery registration")
        })?;
        if snapshot.restored.get() {
            return Err(Error::new(E_ABORT, "Live recovery retirement is pending"));
        }
        if snapshot.hidden.get() {
            return Ok(false);
        }
        if snapshot.hide_attempted.replace(true) {
            return Err(Error::new(E_ABORT, "Live source hide rollback is pending"));
        }
        let result = snapshot.snapshot.apply_hidden();
        if result.is_ok() {
            snapshot.hidden.set(true);
        }
        drop(busy);
        match result {
            Ok(()) => Ok(true),
            Err(error) => {
                // 错误路径同样保留首次快照，直到窗口恢复与登记退休均成功。
                self.0.restore()?;
                Err(error)
            }
        }
    }
    /// 主动召回恢复所有 peer 的源，但保留全部 lease，不停止采集或输入 owner。
    pub fn restore(&self) -> Result<()> {
        self.0.restore()
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        let remaining = self.0.leases.get() - 1;
        self.0.leases.set(remaining);
        if remaining == 0 {
            if let Err(error) = self.0.restore() {
                // 仅报告错误码，避免暴露窗口标题；registry 与独立 journal 负责后续恢复。
                eprintln!("native-live-visibility-restore: {:?}", error.code());
            }
        }
    }
}

#[cfg(test)]
#[path = "live_window_visibility_tests.rs"]
mod tests;
