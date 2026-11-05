//! 默认 profile 的接收 journal 生命周期；后台只打开一次，内存快照不授予联网或显示权。
use crate::{
    desktop::{
        device_authorization_ui::projection::recovery::View,
        loom_execution_signal::{CompletionSignal, Notify},
    },
    projection_delivery::journal::{Store, Ticket},
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

#[derive(Default)]
pub(in crate::desktop::application) enum Owner {
    #[default]
    Disabled,
    Opening(Job),
    Ready {
        store: Store,
        entries: Vec<Arc<Ticket>>,
        policy: Box<super::policy::Owner>,
    },
    Failed(String),
}
struct Loaded {
    store: Store,
    entries: Vec<Ticket>,
    policy: super::policy::Owner,
}
type Prepared = Result<Loaded, String>;
pub(in crate::desktop::application) struct Job {
    cancel: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    thread: Option<JoinHandle<Prepared>>,
}
impl Owner {
    pub(in crate::desktop::application) fn entries(&self) -> Option<&[Arc<Ticket>]> {
        match self {
            Self::Ready { entries, .. } => Some(entries),
            _ => None,
        }
    }
    pub(in crate::desktop::application) fn start(&mut self, root: PathBuf, notify: Notify) {
        if !matches!(self, Self::Disabled) {
            return; // 工作区切换不重开同一 profile 的独占 Store，也不隐式重试失败。
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let ready = Arc::new(AtomicBool::new(false));
        let completed = ready.clone();
        let worker = std::thread::Builder::new()
            .name("otter-projection-profile".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify);
                if signal.load(Ordering::Acquire) {
                    return Err("投送恢复已停止".into());
                }
                let store = Store::open(&root).map_err(|e| format!("投送恢复不可用：{e:?}"))?;
                let entries = store
                    .pending()
                    .map_err(|e| format!("投送恢复不可用：{e:?}"))?;
                if signal.load(Ordering::Acquire) {
                    return Err("投送恢复已停止".into());
                }
                let policy = super::policy::Owner::load(&root, &signal);
                Ok(Loaded {
                    store,
                    entries,
                    policy,
                })
            });
        *self = match worker {
            Ok(thread) => Self::Opening(Job {
                cancel,
                ready,
                thread: Some(thread),
            }),
            Err(error) => Self::Failed(format!("投送恢复未启动：{error}")),
        };
    }
    pub(in crate::desktop::application) fn poll(&mut self) {
        self.collect(false);
    }
    /// 仅应用终态：删除最后 Unit 仍须取得原 Store，不能把 Opening 当空 journal。
    pub(in crate::desktop::application) fn finish_open(&mut self) {
        self.collect(true);
    }
    fn collect(&mut self, terminal: bool) {
        let Self::Opening(job) = self else { return };
        if !terminal
            && !job.ready.load(Ordering::Acquire)
            && !job.thread.as_ref().unwrap().is_finished()
        {
            return;
        }
        let result = job
            .thread
            .take()
            .unwrap()
            .join()
            .unwrap_or_else(|_| Err("投送恢复任务异常退出".into()));
        *self = match result {
            Ok(loaded) => Self::Ready {
                store: loaded.store,
                entries: loaded.entries.into_iter().map(Arc::new).collect(),
                policy: Box::new(loaded.policy),
            },
            Err(error) => Self::Failed(error),
        };
    }
    pub(in crate::desktop::application) fn contains(&self, ticket: &Arc<Ticket>) -> bool {
        matches!(self, Self::Ready { entries, .. } if entries.iter().any(|old| Arc::ptr_eq(old, ticket)))
    }
    pub(in crate::desktop::application) fn recorded(&self, ticket: &Ticket) -> bool {
        matches!(self, Self::Ready { entries, .. } if entries.iter().any(|old| old.same_record(ticket)))
    }
    /// 只在任务已回收时同步 Store 的内存提交结果；即使完成通知被取消也不复活已退休项。
    pub(in crate::desktop::application) fn refresh(&mut self) {
        let Self::Ready { store, entries, .. } = self else {
            return;
        };
        match store.pending() {
            Ok(current) => {
                entries.retain(|old| current.iter().any(|ticket| ticket.same_record(old)));
                for ticket in current {
                    if !entries.iter().any(|old| ticket.same_record(old)) {
                        entries.push(Arc::new(ticket));
                    }
                }
            }
            Err(error) => *self = Self::Failed(format!("投送恢复不可用：{error:?}")),
        }
    }
    pub(in crate::desktop::application) fn sync(
        &self,
        view: &mut View,
        mut included: impl FnMut(&Ticket) -> bool,
    ) {
        let entries = match self {
            Self::Ready { entries, .. } => entries.as_slice(),
            _ => &[],
        };
        if view.entries.len() != entries.len()
            || !view
                .entries
                .iter()
                .zip(entries)
                .all(|(a, b)| Arc::ptr_eq(a, b))
        {
            view.entries = entries.to_vec();
        }
        view.saved
            .retain(|old| entries.iter().any(|entry| Arc::ptr_eq(old, entry)) && included(old));
        for entry in entries {
            if included(entry) && !view.saved.iter().any(|old| Arc::ptr_eq(old, entry)) {
                view.saved.push(entry.clone());
            }
        }
        let status = match self {
            Self::Opening(_) => "正在读取未结投送…",
            Self::Failed(error) => error,
            _ => "",
        };
        if view.status != status {
            view.status = status.into();
        }
    }
    pub(in crate::desktop::application) fn close(&mut self) {
        *self = Self::Disabled;
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        // 文件系统调用不能强制中止；只在应用终态等待回收，不遗留无 owner 线程。
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
