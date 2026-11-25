//! A bounded settings transaction; filesystem preparation and publication never run on the UI thread.
use super::{Outcome, RemoteRequest};
use crate::{
    app_settings_document::Document, display_language::DisplayLanguage, settings_source::Source,
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::Instant,
};

pub(super) struct Completed {
    pub source: Arc<Source>,
    pub warning: Option<String>,
}
pub(super) struct Job {
    pub remote: Option<RemoteRequest>,
    cancel: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<Completed, Outcome>>>,
}
impl Job {
    pub(super) fn start(
        source: Option<Arc<Source>>,
        document: Arc<Document>,
        target: Option<PathBuf>,
        language: DisplayLanguage,
        revision: u64,
        remote: Option<RemoteRequest>,
        notify: crate::desktop::loom_execution_signal::Notify,
    ) -> Result<Self, (Option<RemoteRequest>, String)> {
        let cancel = remote
            .as_ref()
            .map(|request| request.cancel.clone())
            .unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
        let request = remote
            .as_ref()
            .map(|request| (request.request_id.clone(), request.source.clone()));
        let deadline = remote.as_ref().map(|request| request.expires_at);
        let ready = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let completed = ready.clone();
        let worker = std::thread::Builder::new()
            .name("otter-display-language".into())
            .spawn(move || {
                let _completion =
                    crate::desktop::loom_execution_signal::CompletionSignal::new(completed, notify);
                check(&signal, deadline)?;
                let mut document = (*document).clone();
                document
                    .set_display_language(
                        language,
                        revision,
                        request
                            .as_ref()
                            .map(|(id, source)| (id.as_str(), source.as_str())),
                    )
                    .map_err(Outcome::Failed)?;
                publish(source, document, target, &signal, deadline)
            });
        let worker = match worker {
            Ok(worker) => worker,
            Err(error) => return Err((remote, error.to_string())),
        };
        Ok(Self {
            remote,
            cancel,
            ready,
            worker: Some(worker),
        })
    }
    pub(super) fn poll(&mut self) -> Option<Result<Completed, Outcome>> {
        if !self.worker.as_ref()?.is_finished() {
            return None;
        }
        Some(
            self.worker.take()?.join().unwrap_or_else(|_| {
                Err(Outcome::Failed("Display-language worker panicked".into()))
            }),
        )
    }
    pub(super) fn deadline(&self) -> Option<Instant> {
        self.worker.as_ref()?;
        crate::desktop::loom_execution_signal::reap_deadline(&self.ready)
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            self.cancel.store(true, Ordering::Release);
            let _ = worker.join();
        }
    }
}
fn check(cancel: &AtomicBool, deadline: Option<Instant>) -> Result<(), Outcome> {
    if cancel.load(Ordering::Acquire) || deadline.is_some_and(|deadline| Instant::now() >= deadline)
    {
        Err(Outcome::Expired)
    } else {
        Ok(())
    }
}
fn authorize(cancel: &AtomicBool, deadline: Option<Instant>) -> std::io::Result<()> {
    check(cancel, deadline).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "Display-language request expired",
        )
    })
}
fn publish(
    source: Option<Arc<Source>>,
    document: Document,
    target: Option<PathBuf>,
    cancel: &AtomicBool,
    deadline: Option<Instant>,
) -> Result<Completed, Outcome> {
    if let Some(source) = source {
        let staged = crate::settings_update::Prepared::prepare_document(&source, document, cancel)
            .map_err(Outcome::Failed)?;
        check(cancel, deadline)?;
        let (source, published) = staged
            .commit_checked(cancel, || authorize(cancel, deadline))
            .map_err(|failure| {
                if failure.error.kind() == std::io::ErrorKind::Interrupted
                    && failure.recovery.is_none()
                    && failure.restore_error.is_none()
                {
                    return Outcome::Expired;
                }
                Outcome::Failed(format!(
                    "{}; recovery={:?}; restore_error={:?}",
                    failure.error, failure.recovery, failure.restore_error
                ))
            })?;
        Ok(Completed {
            source,
            warning: published
                .retained_recovery
                .map(|(path, error)| format!("{}\n{error}", path.display())),
        })
    } else {
        let target = target.ok_or_else(|| Outcome::Failed("No settings profile target".into()))?;
        let parent = target
            .parent()
            .ok_or_else(|| Outcome::Failed("Missing settings profile directory".into()))?;
        crate::profile_directory::validate(parent)
            .map_err(|error| Outcome::Failed(error.to_string()))?;
        check(cancel, deadline)?;
        std::fs::create_dir_all(parent).map_err(|error| Outcome::Failed(error.to_string()))?;
        crate::profile_directory::validate(parent)
            .map_err(|error| Outcome::Failed(error.to_string()))?;
        let source = Source::from_document(target.clone(), document).map_err(Outcome::Failed)?;
        let staged = crate::session_file_write::Staged::prepare_bytes(
            &target,
            source.bytes().clone(),
            cancel,
        )
        .map_err(|error| Outcome::Failed(error.to_string()))?;
        check(cancel, deadline)?;
        staged
            .commit_new_checked(cancel, || authorize(cancel, deadline))
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::Interrupted {
                    Outcome::Expired
                } else {
                    Outcome::Failed(error.to_string())
                }
            })?;
        Ok(Completed {
            source,
            warning: None,
        })
    }
}
