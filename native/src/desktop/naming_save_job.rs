//! 配置另存为 worker：编码与暂存，不在后台发布；UI 检查凭证后消费 Staged。
use super::loom_execution_signal::{CompletionSignal, Notify};
use crate::{
    app_settings_document::Document, file_naming_settings::Settings, session_file_write::Staged,
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

pub(super) struct Job {
    cancel: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<Ready, String>>>,
}
pub(super) enum Ready {
    New(Staged, Arc<crate::settings_source::Source>),
    Update(crate::settings_update::Prepared),
}
impl Job {
    pub fn start(
        path: PathBuf,
        source: Arc<Document>,
        naming: Settings,
        policy: crate::cache_policy::Policy,
        recycle: crate::session_library::recycle::policy::Policy,
        create_parent: bool,
        notify: Notify,
    ) -> std::io::Result<Self> {
        Self::spawn(notify, move |signal| {
            // 只在显式保存时复制有界配置；不在 UI 热路径或空闲时复制。
            let mut document = (*source).clone();
            document.set_naming(naming)?;
            document.set_cache_policy(policy)?;
            document.set_recycle_policy(recycle)?;
            let next = crate::settings_source::Source::from_document(path.clone(), document)?;
            if create_parent {
                if signal.load(Ordering::Acquire) {
                    return Err("Naming save cancelled".into());
                }
                let parent = path.parent().ok_or("Missing profile directory")?;
                crate::profile_directory::validate(parent).map_err(|e| e.to_string())?;
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                crate::profile_directory::validate(parent).map_err(|e| e.to_string())?;
            }
            let staged = Staged::prepare_bytes(&path, next.bytes().clone(), signal)
                .map_err(|e| e.to_string())?;
            Ok(Ready::New(staged, next))
        })
    }
    pub fn start_update(
        source: Arc<crate::settings_source::Source>,
        naming: Settings,
        policy: crate::cache_policy::Policy,
        recycle: crate::session_library::recycle::policy::Policy,
        notify: Notify,
    ) -> std::io::Result<Self> {
        Self::spawn(notify, move |signal| {
            crate::settings_update::Prepared::prepare_with_policy(
                &source, naming, policy, recycle, signal,
            )
            .map(Ready::Update)
        })
    }
    fn spawn(
        notify: Notify,
        operation: impl FnOnce(&AtomicBool) -> Result<Ready, String> + Send + 'static,
    ) -> std::io::Result<Self> {
        let cancel = Arc::new(AtomicBool::new(false));
        let ready = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let completed = ready.clone();
        let worker = std::thread::Builder::new()
            .name("otter-naming-save".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify);
                if signal.load(Ordering::Acquire) {
                    return Err("Naming save cancelled".into());
                }
                operation(&signal)
            })?;
        Ok(Self {
            cancel,
            ready,
            worker: Some(worker),
        })
    }
    pub fn poll(&mut self) -> Option<Result<Ready, String>> {
        if !self.ready.load(Ordering::Acquire) && !self.worker.as_ref()?.is_finished() {
            return None;
        }
        Some(
            self.worker
                .take()?
                .join()
                .map_err(|_| "Naming save worker panicked".into())
                .and_then(|result| result),
        )
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            // join 返回的未发布 Staged 随结果析构，只清理本任务临时文件。
            let _ = worker.join();
        }
    }
}
