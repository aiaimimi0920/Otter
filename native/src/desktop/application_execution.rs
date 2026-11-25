//! The application thread delivers completed node results with short model borrows, no networking, and no nested message loop.
use super::Application;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "application_execution_tests.rs"]
mod tests;
impl Application {
    pub(super) fn cancel_requested_executions(&mut self) {
        for document in &self.documents {
            super::super::execution_configuration::retire_changed(
                &document.state,
                &mut self.executions,
                &self.workspace,
            );
            let Some(cancel) = document.state.execution_cancel.borrow_mut().take() else {
                continue;
            };
            if let Some(ticket) = cancel.preparation {
                let current = document
                    .state
                    .pending_preparation
                    .borrow()
                    .as_ref()
                    .is_some_and(|pending| pending.ticket == ticket);
                if current {
                    document.state.pending_preparation.borrow_mut().take();
                    *document.state.preparation_error.borrow_mut() = None;
                }
                self.input_jobs.cancel(&ticket);
            }
            if let Some(request) = cancel.execution {
                if self.executions.finish(&document.state.unit_id, &request) {
                    super::super::execution_configuration::clear(&document.state, &request);
                    if document
                        .state
                        .pending_execution
                        .borrow()
                        .as_ref()
                        .is_some_and(|execution| execution.request == request)
                    {
                        document.state.pending_execution.borrow_mut().take();
                    }
                    document
                        .state
                        .execution_results
                        .borrow_mut()
                        .cancel(&request);
                }
            }
        }
        self.execution_jobs.retire_stale(&self.executions);
    }
    pub(super) fn start_pending_executions(&mut self) {
        for document in &self.documents {
            // Workers being cleaned up still occupy slots; temporary contention is neither execution failure nor reason to discard encoded input.
            if self.execution_jobs.len() >= 2 {
                break;
            }
            let Some(mut execution) = document.state.pending_execution.borrow_mut().take() else {
                continue;
            };
            let unit = execution.unit.clone();
            let request = execution.request.clone();
            // A source window cannot borrow another node's request; a stale queued request cannot finish its replacement.
            if unit != document.state.unit_id || !self.executions.is_latest(&unit, &request) {
                continue;
            }
            let current = execution.inputs.freshness().is_ok_and(|freshness| {
                freshness.is_current(|unit, stamp| {
                    stamp.matches_owners(
                        unit,
                        &self.documents,
                        &self.live_sessions,
                        &self.workspace,
                    )
                })
            });
            let mut results = document.state.execution_results.borrow_mut();
            results.begin(request.clone());
            if !current
                || !super::super::execution_configuration::matches(
                    &document.state,
                    &request,
                    &self.workspace,
                )
            {
                results.fail(
                    &request,
                    crate::ui_text::text("app-execution-document-changed"),
                );
                self.executions.finish(&unit, &request);
                super::super::execution_configuration::clear(&document.state, &request);
                continue;
            }
            // If the registration token has been replaced, cancel the old worker first; an owner still cleaning up continues occupying the slot.
            self.execution_jobs.retire_stale(&self.executions);
            self.shader_cache.bind(&document.state, &mut execution);
            if self
                .execution_jobs
                .start(execution, &self.executions)
                .is_err()
            {
                results.fail(&request, crate::ui_text::text("app-execution-start-failed"));
                self.executions.finish(&unit, &request);
                super::super::execution_configuration::clear(&document.state, &request);
            }
        }
    }
    pub(super) fn collect_execution_updates(&self) {
        for document in &self.documents {
            let Some(updates) = self.execution_jobs.take_updates_checked(
                &document.state.unit_id,
                &self.executions,
                |unit, stamp| {
                    stamp.matches_owners(
                        unit,
                        &self.documents,
                        &self.live_sessions,
                        &self.workspace,
                    )
                },
            ) else {
                continue;
            };
            let mut results = document.state.execution_results.borrow_mut();
            if let Some(progress) = updates.progress {
                results.set_progress(&updates.request, progress);
            }
            if let Some(crate::desktop::loom_execution_session::Update::Preview {
                revision,
                mut delivery,
            }) = updates.preview
            {
                // Session publishes only previews whose leases are released; still check delivery identity and collection eligibility here.
                if delivery.unit() == &document.state.unit_id
                    && delivery.request() == &updates.request
                {
                    if let Ok(batch) = delivery.take_released() {
                        let _ = results.preview(&updates.request, revision, batch);
                    }
                }
            }
        }
    }
    pub(super) fn collect_execution_results(&mut self) {
        while let Some(mut report) = self.execution_jobs.take_settled(&self.executions) {
            let Some(document) = self
                .documents
                .iter()
                .find(|document| document.state.unit_id == report.unit)
            else {
                self.executions.finish(&report.unit, &report.request);
                continue;
            };
            // Self-referencing output may also be a dependency; perform a brief read-only check before borrowing target results, with no intervening UI reentry.
            let current = report.dependencies_current(|unit, stamp| {
                stamp.matches_owners(unit, &self.documents, &self.live_sessions, &self.workspace)
            });
            let mut results = document.state.execution_results.borrow_mut();
            let result = report.commit_checked(&mut self.executions, &mut results, |_, _| current);
            let committed = result.is_ok();
            if committed {
                results.bind_candidate_configuration(
                    super::super::configuration_stamp::Stamp::capture(&document.state),
                );
            }
            if let Err(error) = result {
                // Do not put raw service responses/authentication information into UI state; types distinguish detailed causes.
                let message = match error {
                    crate::desktop::loom_execution_tasks::CommitError::Stale => {
                        crate::ui_text::text("app-execution-result-stale")
                    }
                    crate::desktop::loom_execution_tasks::CommitError::ExecutionFailed => {
                        crate::ui_text::text("app-execution-failed")
                    }
                    crate::desktop::loom_execution_tasks::CommitError::PendingCleanup => {
                        crate::ui_text::text("app-execution-cleanup-unconfirmed")
                    }
                    crate::desktop::loom_execution_tasks::CommitError::Output(_) => {
                        crate::ui_text::text("app-execution-result-unreadable")
                    }
                    crate::desktop::loom_execution_tasks::CommitError::State(_) => {
                        crate::ui_text::text("app-execution-result-limit")
                    }
                };
                report.reject(&mut self.executions, &mut results, message);
            }
            super::super::execution_configuration::clear(&document.state, &report.request);
            drop(results);
            if committed {
                self.shader_cache.record(&document.state, &mut report);
                // Notify only after the result transaction ends and borrows are released; dispatch admission still belongs to the main loop.
                unsafe {
                    self.propagate_result(&report.unit);
                }
            }
        }
    }
}
