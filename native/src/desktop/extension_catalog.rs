//! 应用级扩展目录投影；启动请求支持声明式快捷键，后台推送刷新，失败仅显式重开编辑栏重试。
use super::{
    extension_catalog_job::{Event, Job},
    loom_execution_signal::Notify,
    State,
};
use crate::loom_extension_snapshot::Snapshot;
use std::{cell::RefCell, rc::Rc, sync::Arc};
use windows::{
    core::Result,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Idle,
    Loading,
    Ready,
    Failed,
    Closed,
}
#[derive(Default)]
pub(super) struct View {
    pub(super) snapshot: Option<Arc<Snapshot>>,
    phase: Phase,
    requested: bool,
}
pub(super) type Shared = Rc<RefCell<View>>;
pub(super) struct Owner {
    pub(super) view: Shared,
    job: Job,
    notify: Notify,
}
pub(super) unsafe fn request(hwnd: HWND, state: &State) -> Result<()> {
    request_for(hwnd, &state.extensions)
}
unsafe fn request_for(hwnd: HWND, shared: &Shared) -> Result<()> {
    let mut view = shared.borrow_mut();
    if view.requested || !matches!(view.phase, Phase::Idle | Phase::Failed) {
        return Ok(());
    }
    view.requested = true;
    if let Err(error) = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) {
        view.requested = false;
        return Err(error);
    }
    Ok(())
}
impl Owner {
    pub(super) fn new(view: Shared, notify: Notify) -> Self {
        Self {
            view,
            job: Job::default(),
            notify,
        }
    }
    pub(super) fn process(&mut self) -> bool {
        self.process_with(|| {
            super::capability_job::Source::Discovery(crate::loom_discovery::Config::from_env())
        })
    }
    pub(super) fn deadline(&self) -> Option<std::time::Instant> {
        self.job.deadline()
    }
    fn process_with(&mut self, source: impl FnOnce() -> super::capability_job::Source) -> bool {
        let mut changed = false;
        if let Some(event) = self.job.poll() {
            let mut view = self.view.borrow_mut();
            match event {
                Event::Snapshot(snapshot) => {
                    view.snapshot = Some(snapshot);
                    view.phase = Phase::Ready;
                }
                Event::Revoked => {
                    view.snapshot = None;
                    view.phase = Phase::Failed;
                }
                Event::Stopped(result) => {
                    view.snapshot = None;
                    view.phase = Phase::Failed;
                    if let Err(error) = result {
                        eprintln!("native-extension-catalog: {error}");
                    }
                }
            }
            changed = true;
        }
        // 撤权已可见，但旧线程可能还在通知尾段；保留用户显式重试，不自动重连。
        if !self.job.is_busy() && std::mem::take(&mut self.view.borrow_mut().requested) {
            let mut view = self.view.borrow_mut();
            view.snapshot = None;
            view.phase = Phase::Loading;
            if let Err(error) = self.job.start(source(), self.notify.clone()) {
                view.phase = Phase::Failed;
                eprintln!("native-extension-catalog-start: {error}");
            }
            changed = true;
        }
        changed
    }
    pub(super) fn shutdown(&mut self) {
        {
            let mut view = self.view.borrow_mut();
            view.phase = Phase::Closed;
            view.requested = false;
            view.snapshot = None;
        }
        self.job.shutdown();
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.shutdown();
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "extension_catalog_tests.rs"]
mod tests;
