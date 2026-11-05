//! 应用历史的单后台槽；载入/缩略图/保存串行，空闲无线程，退出join不遗留任务。
use crate::capture_history::{self as history, store::Store, Data};
use crate::desktop::capture_history_ui::Shared;
use std::{path::PathBuf, thread::JoinHandle};
enum Output {
    Failed,
    Loaded(Result<(Store, Data), String>),
    Captured(Result<history::Screenshot, String>),
    Saved(Store, Result<(), String>),
}
pub(in crate::desktop::application) struct Owner {
    pub view: Shared,
    job: Option<JoinHandle<Output>>,
    store: Option<Store>,
    saved: u64,
    saving: u64,
    failed: bool,
    ready: std::sync::Arc<std::sync::atomic::AtomicBool>,
    notify: std::sync::Arc<dyn Fn() + Send + Sync>,
}
impl Owner {
    pub fn new(view: Shared, notify: std::sync::Arc<dyn Fn() + Send + Sync>) -> Self {
        Self {
            view,
            job: None,
            store: None,
            saved: 0,
            saving: 0,
            failed: false,
            ready: Default::default(),
            notify,
        }
    }
    pub fn install(&mut self, root: Option<PathBuf>) {
        if let Some(root) = root {
            self.start(move || {
                let result = crate::desktop::Apartment::new()
                    .map_err(|e| e.to_string())
                    .and_then(|_com| Store::open(&root));
                Output::Loaded(result)
            });
        }
    }
    fn start(&mut self, run: impl FnOnce() -> Output + Send + 'static) {
        debug_assert!(self.job.is_none());
        let notify = self.notify.clone();
        self.ready
            .store(false, std::sync::atomic::Ordering::Release);
        let ready = self.ready.clone();
        match std::thread::Builder::new()
            .name("otter-history".into())
            .spawn(move || {
                let output = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run))
                    .unwrap_or(Output::Failed);
                ready.store(true, std::sync::atomic::Ordering::Release);
                notify();
                output
            }) {
            Ok(job) => self.job = Some(job),
            Err(error) => self.error(error.to_string()),
        }
    }
    fn error(&mut self, error: String) {
        self.failed = true;
        // 不记录截图/颜色/文件内容；短状态只在历史窗显示。
        eprintln!("native-capture-history: operation failed");
        self.view.borrow_mut().status = Some(format!("历史记录未保存：{error}"));
    }
    fn settle(&mut self) {
        let Some(job) = self.job.take() else { return };
        let output = match job.join() {
            Ok(output) => output,
            Err(_) => {
                self.error("History worker failed".into());
                return;
            }
        };
        match output {
            Output::Failed => self.error("History worker failed".into()),
            Output::Loaded(result) => match result {
                Ok((store, mut loaded)) => {
                    let mut view = self.view.borrow_mut();
                    let revision = view.revision;
                    loaded.merge_recent(std::mem::take(&mut view.data));
                    view.data = loaded;
                    if view
                        .data
                        .screenshots
                        .iter()
                        .any(|entry| entry.preview.is_none())
                    {
                        view.status = Some("部分截图预览不可用，原记录已保留".into());
                    }
                    view.changed();
                    self.saved = if revision == 0 { view.revision } else { 0 };
                    self.store = Some(store);
                }
                Err(error) => self.error(error),
            },
            Output::Captured(result) => match result {
                Ok(entry) => {
                    let mut view = self.view.borrow_mut();
                    view.data.screenshot(entry);
                    view.changed();
                }
                Err(error) => {
                    self.view.borrow_mut().status = Some(format!("截图未加入历史：{error}"));
                }
            },
            Output::Saved(store, result) => {
                self.store = Some(store);
                match result {
                    Ok(()) => self.saved = self.saving,
                    Err(error) => self.error(error),
                }
            }
        }
    }
    pub fn poll(&mut self) {
        if self.job.is_some() && self.ready.load(std::sync::atomic::Ordering::Acquire) {
            self.settle();
        }
        if self.job.is_some() {
            return;
        }
        let capture = self.view.borrow_mut().captures.pop_front();
        if let Some(capture) = capture {
            self.start(move || {
                Output::Captured(history::image::capture(
                    capture.id,
                    &capture.image,
                    capture.at,
                ))
            });
        } else if !self.failed && self.view.borrow().revision != self.saved {
            if let Some(mut store) = self.store.take() {
                let view = self.view.borrow();
                let data = view.data.clone();
                self.saving = view.revision;
                drop(view);
                self.start(move || {
                    let result = store.save(&data);
                    Output::Saved(store, result)
                });
            }
        }
    }
    pub fn shutdown(&mut self) {
        // 历史是便利缓存，不阻止工作区退出；但所有本批worker始终有join owner。
        loop {
            if self.job.is_some() {
                self.settle();
            }
            self.poll();
            if self.job.is_none() {
                break;
            }
        }
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
#[path = "capture_history_worker_tests.rs"]
mod tests;
