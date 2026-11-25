//! Local-result collection and official-port transactions on the UI thread; no reentrancy or writing Art output into source screenshots.
use super::*;
impl Owner {
    pub(in crate::desktop) fn apply_results(
        &mut self,
        documents: &[DocumentWindow],
        workspace: &Workspace,
        sessions: &[LiveSession],
    ) -> (Vec<UnitId>, Option<String>) {
        let mut changed = Vec::new();
        if changed.try_reserve(self.entries.len()).is_err() {
            return (
                changed,
                Some(crate::ui_text::text("integration-art-output-notification-failed").into()),
            );
        }
        let mut error = None;
        for entry in &mut self.entries {
            if entry.window.is_none() {
                continue;
            }
            let valid = entry.active && current(entry, documents, workspace, sessions);
            let unit = &entry.unit;
            let stamp = &entry.stamp;
            let inputs = &entry.inputs;
            let images = &entry.images;
            let Some(window) = &mut entry.window else {
                continue;
            };
            let host = || {
                valid
                    && inputs.matches(workspace)
                    && images.current(documents, sessions, workspace)
                    && host_current(unit, stamp, documents, workspace)
            };
            let Some(result) = window.take_result(&host) else {
                continue;
            };
            let trigger = window.result_trigger();
            let applied = (|| {
                let document = documents
                    .iter()
                    .find(|doc| &doc.state.unit_id == unit)
                    .ok_or(())?;
                // Only actual Art nodes may own official ports; ordinary screenshots merely initiate the UI action and are not implicitly upgraded.
                if !host() || !document.state.is_art() || !window.can_commit_result() {
                    return Err(());
                }
                let mut results = document.state.execution_results.borrow_mut();
                if let Some(ticket) = &trigger {
                    ticket
                        .commit(ticket.binding(), |allowed| {
                            results.apply_local_result_if(&entry.result_stamp, result, &|| {
                                allowed() && window.result_authorized()
                            })
                        })
                        .ok_or(())?
                        .map_err(|_| ())?;
                } else {
                    results
                        .apply_local_result_if(&entry.result_stamp, result, &|| {
                            window.result_authorized()
                        })
                        .map_err(|_| ())?;
                }
                entry.result_stamp = results.output_stamp();
                window.result_applied();
                Ok(())
            })();
            if applied.is_ok() {
                if !changed.contains(unit) {
                    changed.push(unit.clone());
                }
            } else {
                entry.active = false;
                entry.images.cancel();
                self.tasks.cancel(unit);
                let _ = window.stop();
                error = Some(crate::ui_text::text("integration-art-output-uncommitted").into());
                self.status_owner = None;
            }
        }
        (changed, error)
    }
}
