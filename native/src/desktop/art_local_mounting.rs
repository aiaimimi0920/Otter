//! Admit downloads and image preparation in stages; create a signed child only while the original token, host, and source all remain valid.
use super::*;
impl Owner {
    pub(super) fn prepare_windows(
        &mut self,
        documents: &[DocumentWindow],
        workspace: &mut Workspace,
        sessions: &[LiveSession],
    ) -> Option<String> {
        let mut status = None;
        while let Some(report) = self.tasks.take_settled() {
            let Some(entry) = self
                .entries
                .iter_mut()
                .find(|entry| entry.active && entry.unit == report.unit)
            else {
                continue;
            };
            let result = (|| {
                let prepared = report
                    .result
                    .map_err(|_| crate::ui_text::text("integration-art-prepare-failed"))?;
                let binding = prepared.binding();
                if !current(entry, documents, workspace, sessions)
                    || binding.instance_id != entry.instance
                    || binding.request_id != entry.token.id.as_str()
                    || binding.generation != entry.token.generation
                    || Some(binding.workspace_id.as_str()) != self.workspace.as_deref()
                {
                    return Err(crate::ui_text::text(
                        "integration-art-host-preparation-stale",
                    ));
                }
                ports::accept(entry, &prepared, documents, workspace)?;
                entry.images.start(
                    prepared,
                    documents,
                    sessions,
                    workspace,
                    &entry.unit,
                    self.notify.clone(),
                )
            })();
            if let Err(message) = result {
                entry.active = false;
                entry.images.cancel();
                self.tasks.cancel(&entry.unit);
                status = Some(message.into());
                self.status_owner = None;
            }
        }
        for entry in &mut self.entries {
            let Some(result) = entry.images.poll() else {
                continue;
            };
            if !entry.active {
                continue;
            }
            let result = (|| {
                let (prepared, content) = result?;
                if !current(entry, documents, workspace, sessions) {
                    return Err(crate::ui_text::text(
                        "integration-art-image-preparation-stale",
                    ));
                }
                // node is this host mapping's identity; historical UnitId values need not meet the network identifier character set.
                let view = crate::surface_live_binding::Binding::local_art(
                    prepared.binding(),
                    &entry.instance,
                    &entry.token.id.to_string(),
                )
                .map_err(|_| crate::ui_text::text("integration-art-view-create-failed"))?;
                entry.window = Some(
                    Window::start_with_content(prepared, view, self.notify.clone(), content)
                        .map_err(|_| crate::ui_text::text("integration-art-host-create-failed"))?,
                );
                self.tasks.mark_running(&entry.unit, &entry.token);
                Ok(())
            })();
            if let Err(message) = result {
                entry.active = false;
                entry.images.cancel();
                self.tasks.cancel(&entry.unit);
                status = Some(message.into());
                self.status_owner = None;
            }
        }
        status
    }
}
