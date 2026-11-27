//! One display-language owner for local and authenticated remote requests; save before publishing UI state.
use super::*;
use crate::display_language::DisplayLanguage;
use std::{
    cell::Cell,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};
#[path = "application_locale_job.rs"]
mod job;
pub(super) use job::{publish as publish_settings, Completed as SavedSettings};
#[path = "application_locale_refresh.rs"]
mod refresh;

#[derive(Debug)]
pub(super) struct RemoteRequest {
    pub request_id: String,
    pub source: String,
    pub language: DisplayLanguage,
    pub expected_revision: u64,
    pub expires_at: Instant,
    pub cancel: Arc<AtomicBool>,
}
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    Applied,
    Conflict,
    Expired,
    Failed(String),
}
pub(super) struct Receipt {
    pub request_id: String,
    pub source: String,
    pub language: DisplayLanguage,
    pub revision: u64,
    pub outcome: Outcome,
}
pub(super) struct Status {
    pub language: DisplayLanguage,
    pub revision: u64,
    pub last_request_id: Option<String>,
    pub last_request_source: Option<String>,
}
#[derive(Default)]
pub(super) struct Owner {
    pub local: Rc<Cell<Option<DisplayLanguage>>>,
    remote: Option<RemoteRequest>,
    result: Option<Receipt>,
    job: Option<job::Job>,
}
impl Owner {
    pub(super) fn with_local(local: Rc<Cell<Option<DisplayLanguage>>>) -> Self {
        Self {
            local,
            ..Self::default()
        }
    }
}
impl Application {
    pub(super) fn display_language_status(&self) -> Status {
        let last = self
            .naming_document
            .last_display_language_request()
            .expect("settings document validated at admission");
        Status {
            language: self
                .naming_document
                .display_language()
                .expect("settings document validated at admission"),
            revision: self
                .naming_document
                .display_language_revision()
                .expect("settings document validated at admission"),
            last_request_id: last.map(|(id, _)| id.to_owned()),
            last_request_source: last.map(|(_, source)| source.to_owned()),
        }
    }
    pub(super) fn request_display_language(
        &mut self,
        request: RemoteRequest,
    ) -> std::result::Result<(), String> {
        if self.exit_requested.get() || self.exit_after_save || self.session_end.paused() {
            return Err("unavailable".into());
        }
        if self.locale.remote.is_some() || self.locale.job.is_some() || self.locale.result.is_some()
        {
            return Err("busy".into());
        }
        if request.request_id.is_empty()
            || request.request_id.len() > 128
            || request.source.is_empty()
            || request.source.len() > 2048
            || request.request_id.chars().any(char::is_control)
            || request.source.chars().any(char::is_control)
            || request.expected_revision > 9_007_199_254_740_991
        {
            return Err("invalid_request".into());
        }
        let status = self.display_language_status();
        let duplicate = status.last_request_id.as_deref() == Some(request.request_id.as_str())
            && status.last_request_source.as_deref() == Some(request.source.as_str());
        let receipt = self
            .naming_document
            .last_display_language_receipt()
            .expect("settings document validated at admission");
        let outcome =
            if request.cancel.load(Ordering::Acquire) || Instant::now() >= request.expires_at {
                Some(Outcome::Expired)
            } else if duplicate {
                Some(
                    if receipt.is_some_and(|(language, revision)| {
                        language == request.language
                            && request.expected_revision.checked_add(1) == Some(revision)
                    }) {
                        Outcome::Applied
                    } else {
                        Outcome::Conflict
                    },
                )
            } else if status.revision != request.expected_revision {
                Some(Outcome::Conflict)
            } else {
                None
            };
        if let Some(outcome) = outcome {
            let applied_revision = (duplicate && outcome == Outcome::Applied)
                .then(|| receipt.expect("duplicate receipt validated").1);
            self.finish_language_request(request, outcome);
            if let Some(revision) = applied_revision {
                self.locale
                    .result
                    .as_mut()
                    .expect("receipt just stored")
                    .revision = revision;
            }
        } else {
            self.locale.remote = Some(request);
            self.execution_wake.callback()();
        }
        Ok(())
    }
    pub(super) fn take_display_language_result(&mut self) -> Option<Receipt> {
        self.locale.result.take()
    }
    fn finish_language_request(&mut self, request: RemoteRequest, outcome: Outcome) {
        let status = self.display_language_status();
        self.locale.result = Some(Receipt {
            request_id: request.request_id,
            source: request.source,
            language: status.language,
            revision: status.revision,
            outcome,
        });
    }
    pub(super) fn locale_busy(&self) -> bool {
        self.locale.job.is_some() || self.shortcut_remote.saving()
    }
    pub(super) fn locale_blocks_exit(&self) -> bool {
        self.locale_busy() || self.locale.remote.is_some() || self.locale.local.get().is_some()
    }
    pub(super) fn locale_reap_deadline(&self) -> Option<Instant> {
        if self.modals.blocked() {
            return None;
        }
        self.locale.job.as_ref()?.deadline()
    }
    pub(super) unsafe fn process_display_language(&mut self) {
        if self.modals.blocked() {
            return;
        }
        if self.locale.remote.as_ref().is_some_and(|request| {
            request.cancel.load(Ordering::Acquire) || Instant::now() >= request.expires_at
        }) {
            let request = self.locale.remote.take().expect("expired request exists");
            self.finish_language_request(request, Outcome::Expired);
            self.execution_wake.callback()();
        }
        if let Some(result) = self.locale.job.as_mut().and_then(job::Job::poll) {
            let mut pending = self.locale.job.take().expect("polled language job exists");
            let outcome = match result {
                Ok(completed) => {
                    self.naming_document = completed.source.document().clone();
                    self.naming_source = Some(completed.source);
                    self.refresh_display_language();
                    if let Some(warning) = completed.warning {
                        self.language_notice("display-language-save-warning", &warning);
                    }
                    Outcome::Applied
                }
                Err(outcome) => outcome,
            };
            if let Some(request) = pending.remote.take() {
                self.finish_language_request(request, outcome);
            } else if let Outcome::Failed(error) = outcome {
                self.exit_requested.set(false);
                self.language_notice("display-language-save-failed", &error);
            }
            // Reenter exit/settings settlement even when the completed worker was the only wake source.
            self.execution_wake.callback()();
        }
        if self.session_end.paused() {
            return;
        }
        if self.locale_busy() || self.naming_save_job.is_some() || self.naming_import_job.is_some()
        {
            return;
        }
        let local = self.locale.local.take();
        let remote = if local.is_none() {
            self.locale.remote.take()
        } else {
            None
        };
        let Some(language) = local.or_else(|| remote.as_ref().map(|request| request.language))
        else {
            return;
        };
        let status = self.display_language_status();
        if let Some(request) = &remote {
            let outcome =
                if request.cancel.load(Ordering::Acquire) || Instant::now() >= request.expires_at {
                    Some(Outcome::Expired)
                } else if request.expected_revision != status.revision {
                    Some(Outcome::Conflict)
                } else {
                    None
                };
            if let Some(outcome) = outcome {
                self.finish_language_request(remote.expect("remote checked"), outcome);
                return;
            }
        }
        let Some(revision) = status
            .revision
            .checked_add(1)
            .filter(|value| *value <= 9_007_199_254_740_991)
        else {
            let error = "Display-language revision limit reached";
            if let Some(request) = remote {
                self.finish_language_request(request, Outcome::Failed(error.into()));
            } else {
                self.language_notice("display-language-save-failed", error);
            }
            return;
        };
        // The settings owner is now exclusive until this worker is reaped; naming drafts remain in memory.
        match job::Job::start(
            self.naming_source.clone(),
            self.naming_document.clone(),
            self.naming_profile_target.clone(),
            language,
            revision,
            remote,
            self.execution_wake.callback(),
        ) {
            Ok(job) => self.locale.job = Some(job),
            Err((Some(request), error)) => {
                self.finish_language_request(request, Outcome::Failed(error))
            }
            Err((None, error)) => self.language_notice("display-language-save-failed", &error),
        }
    }
    unsafe fn language_notice(&self, key: &'static str, detail: &str) {
        let flag = self
            .documents
            .iter()
            .find(|document| document.is_alive())
            .map(|document| document.state.file_dialog.clone());
        let _guard = flag
            .as_ref()
            .and_then(|flag| crate::desktop::file_dialog::Guard::begin(flag));
        MessageBoxW(
            None,
            &HSTRING::from(format!("{}\n{detail}", crate::ui_text::text(key))),
            w!("Otter"),
            MB_OK | MB_ICONWARNING,
        );
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "application_locale_tests.rs"]
mod tests;
