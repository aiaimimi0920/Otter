//! Publish the staged policy in the same settings transaction used by display-language changes.
use super::{locale, Application};
use crate::desktop::loom_execution_signal::{reap_deadline, CompletionSignal};
use crate::desktop::shortcut_registry::{self, Chords};
use crate::{
    device_authorization_lease::Lease, loom_locale_protocol::Receipt,
    loom_shortcut_protocol::Request, shortcut_policy::Policy,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::Instant,
};
pub(super) struct Save {
    pub id: String,
    pub source: String,
    previous: Policy,
    held: Chords,
    cancel: Arc<AtomicBool>,
    _lease: Arc<Lease>,
    ready: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<locale::SavedSettings, locale::Outcome>>>,
}
pub(super) fn receipt(id: String, code: &'static str, revision: Option<u64>) -> Receipt {
    Receipt {
        request_id: id,
        status: if revision.is_some() {
            "applied"
        } else if code == "shortcut_revision_conflict" {
            "conflict"
        } else {
            "rejected"
        },
        code,
        applied_revision: revision,
    }
}
impl Application {
    unsafe fn switch_shortcuts(
        &self,
        policy: Policy,
        rollback: Option<Chords>,
    ) -> Result<(), String> {
        *self.shortcut_policy.borrow_mut() = policy;
        let doc = self
            .documents
            .iter()
            .find(|doc| doc.is_alive() && crate::desktop::shortcut_registry::owns(&doc.state))
            .ok_or_else(|| String::from("No active shortcut host"))?;
        if rollback.is_some() {
            // An already occupied old primary must not abort restoration of
            // auxiliaries or cause the rejected new primary to be reinstalled.
            shortcut_registry::resume(doc.window.0, &doc.state)
        } else {
            shortcut_registry::configure(doc.window.0, &doc.state, doc.state.hotkey_enabled.get())
        }
        .map_err(|e| e.to_string())?;
        if rollback.is_some_and(|before| !shortcut_registry::recovered(&doc.state, before)) {
            return Err("Previously registered shortcut unavailable after rollback".into());
        }
        for doc in self.documents.iter().filter(|doc| doc.is_alive()) {
            crate::desktop::tool_panel::refresh_language(&doc.state);
        }
        Ok(())
    }
    pub(super) unsafe fn start_shortcut_save(
        &mut self,
        source: String,
        request: Request,
        until: Instant,
        cancel: Arc<AtomicBool>,
        lease: Arc<Lease>,
    ) -> Result<Save, Receipt> {
        let reject = |code| receipt(request.request_id.clone(), code, None);
        if cancel.load(Ordering::Acquire) || Instant::now() >= until {
            return Err(reject("request_expired"));
        }
        if self.modals.blocked()
            || self.capture_active.get()
            || self.locale_busy()
            || self.naming_save_job.is_some()
            || self.naming_import_job.is_some()
            || self.exit_requested.get()
            || self.exit_after_save
            || self.exit_committed
            || self.checkpoint_exit_waiting()
            || self.session_end.paused()
        {
            return Err(reject("settings_busy"));
        }
        let mut document = (*self.naming_document).clone();
        let previous = self.shortcut_policy.borrow().clone();
        let current = document
            .shortcut_settings()
            .map_err(|_| reject("settings_invalid"))?;
        let revision = document
            .set_shortcuts(
                &source,
                &request.request_id,
                request.expected_revision,
                request.policy.clone(),
            )
            .map_err(|e| {
                reject(match e.as_str() {
                    "shortcut_revision_conflict" => "shortcut_revision_conflict",
                    "shortcut_request_id_conflict" => "shortcut_request_id_conflict",
                    _ => "shortcut_policy_invalid",
                })
            })?;
        if revision == current.revision {
            return Err(receipt(request.request_id, "applied", Some(revision)));
        }
        if self
            .extensions
            .view
            .borrow()
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| {
                snapshot.shortcuts().map_or(true, |items| {
                    items.iter().any(|item| {
                        let domain = if item.global {
                            crate::shortcut_policy::GLOBAL
                        } else {
                            crate::shortcut_policy::CANVAS
                        };
                        domain.iter().any(|(id, _)| {
                            request.policy.chord(id).is_some_and(|c| {
                                previous.chord(id) != Some(c)
                                    && u32::from(c.key) == item.chord.key
                                    && u32::from(c.modifiers) == item.chord.modifiers
                            })
                        })
                    })
                })
            })
        {
            return Err(reject("extension_shortcut_conflict"));
        }
        let target = self.naming_profile_target.clone();
        let disk = self.naming_source.clone();
        if target.is_none()
            || disk.as_ref().is_some_and(|s| {
                s.recovery_target().is_some() || Some(s.path()) != target.as_deref()
            })
        {
            return Err(reject("settings_read_only"));
        }
        let held = self
            .documents
            .iter()
            .find(|doc| doc.is_alive() && shortcut_registry::owns(&doc.state))
            .map(|doc| shortcut_registry::held(&doc.state))
            .ok_or_else(|| reject("registration_failed"))?;
        if let Err(e) = self.switch_shortcuts(request.policy, None) {
            eprintln!("native-shortcut-register: {e}");
            if let Err(e) = self.switch_shortcuts(previous, Some(held)) {
                eprintln!("native-shortcut-rollback: {e}");
                return Err(reject("registration_rollback_failed"));
            }
            return Err(reject("registration_failed"));
        }
        let ready = Arc::new(AtomicBool::new(false));
        let complete = ready.clone();
        let signal = cancel.clone();
        let notify = self.execution_wake.callback();
        let worker = std::thread::Builder::new()
            .name("otter-shortcut-save".into())
            .spawn(move || {
                let _done = CompletionSignal::new(complete, notify);
                locale::publish_settings(disk, document, target, &signal, Some(until))
            });
        match worker {
            Ok(worker) => Ok(Save {
                id: request.request_id,
                source,
                previous,
                held,
                cancel,
                _lease: lease,
                ready,
                worker: Some(worker),
            }),
            Err(_) => {
                if let Err(e) = self.switch_shortcuts(previous, Some(held)) {
                    eprintln!("native-shortcut-rollback: {e}");
                    return Err(reject("registration_rollback_failed"));
                }
                Err(reject("settings_save_failed"))
            }
        }
    }
}
impl Save {
    #[cfg(test)]
    pub(super) fn test_wait_finished(&self) {
        crate::desktop::loom_execution_tasks::reap_fixture::wait_finished(
            self.worker.as_ref().unwrap(),
        );
    }
    pub(super) fn stop(&self) {
        self.cancel.store(true, Ordering::Release);
        self._lease.revoke();
    }
    pub(super) fn deadline(&self) -> Option<Instant> {
        self.worker.as_ref()?;
        reap_deadline(&self.ready)
    }
    pub(super) unsafe fn poll(&mut self, app: &mut Application) -> Option<Receipt> {
        if !self.worker.as_ref()?.is_finished() {
            return None;
        }
        let result = self.worker.take()?.join();
        let (mut code, revision) = match result {
            Ok(Ok(saved)) => {
                let revision = saved
                    .source
                    .document()
                    .shortcut_settings()
                    .expect("validated policy")
                    .revision;
                app.naming_document = saved.source.document().clone();
                app.naming_source = Some(saved.source);
                if let Some(warning) = saved.warning {
                    eprintln!("native-shortcut-published: {warning}");
                }
                ("applied", Some(revision))
            }
            Ok(Err(locale::Outcome::Expired)) => ("request_expired", None),
            _ => ("settings_save_failed", None),
        };
        if revision.is_none() {
            if let Err(e) = app.switch_shortcuts(self.previous.clone(), Some(self.held)) {
                eprintln!("native-shortcut-rollback: {e}");
                code = "registration_rollback_failed";
            }
        }
        Some(receipt(self.id.clone(), code, revision))
    }
}
impl Drop for Save {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            self.stop();
            let _ = worker.join();
        }
    }
}
