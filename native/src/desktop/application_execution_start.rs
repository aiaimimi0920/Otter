//! Application scheduling for manual/parameter requests; automatic triggers check policy and missing images, while window callbacks neither encode nor use the network.
use super::Application;
use crate::desktop::{
    document_state::new_unit_id,
    loom_execution_prepare::{Configured, Pending},
};
use windows::{
    core::{w, HSTRING},
    Win32::UI::WindowsAndMessaging::*,
};
#[cfg(test)]
#[path = "application_execution_start_tests.rs"]
mod tests;
impl Application {
    pub(super) unsafe fn start_requested_preparations(&mut self, visible: bool) {
        self.shader_cache
            .retain(&self.documents, self.shader_installation.is_some());
        for document in &self.documents {
            let state = &document.state;
            let stale = state
                .pending_start
                .borrow()
                .as_ref()
                .is_some_and(|request| !request.stamp.matches(state));
            if stale {
                state.pending_start.borrow_mut().take();
                continue;
            }
            // Retain the latest request while an old worker/uncollected result occupies the slot; completion notification reenters the main loop.
            if !crate::desktop::execution_start::ready(state)
                || self.input_jobs.len() >= 2
                || self.execution_jobs.len() >= 2
            {
                continue;
            }
            let Some(request) = state.pending_start.borrow_mut().take() else {
                continue;
            };
            let manual_trigger = request.trigger == crate::graph_execution_config::Trigger::Manual;
            let result = (|| -> Result<(), &'static str> {
                let selected = state.selected_art.borrow();
                let selected = selected
                    .as_ref()
                    .ok_or(crate::ui_text::text("app-capability-not-selected"))?;
                let capability = selected
                    .catalog
                    .handshake
                    .arts()
                    .iter()
                    .filter_map(crate::loom_capability::Capability::new)
                    .find(|capability| capability.id == selected.parameters.art_id())
                    .filter(|capability| capability.enabled)
                    .ok_or(crate::ui_text::text("app-capability-unavailable"))?;
                let mut runtime = capability.runtime();
                let shader = self.shader_installation.as_ref().filter(|_| {
                    runtime.shader && !runtime.surface && !runtime.formal_after_preview
                });
                if shader.is_some() {
                    // Lift only the ShaderOnly gate for the connected local code-fetch chain; do not relax automatic-trigger/Surface/committed-execution policy.
                    runtime.shader = false;
                }
                if selected.execution.gate(request.trigger, runtime).is_err() {
                    if manual_trigger {
                        return Err(crate::ui_text::text("app-capability-preview-only"));
                    }
                    *state.preparation_error.borrow_mut() = None;
                    return Ok(());
                }
                if !selected
                    .catalog
                    .handshake
                    .operations()
                    .iter()
                    .any(|op| op == "loom.hook.art.execute")
                {
                    return Err(crate::ui_text::text("app-capability-no-execution"));
                }
                let (plan, dependencies) = Self::execution_parameters(
                    &self.documents,
                    &self.workspace,
                    &state.unit_id,
                    &capability,
                    &selected.parameters,
                    shader,
                    |resolved| {
                        Self::image_inputs(
                            &self.documents,
                            &self.live_sessions,
                            &self.workspace,
                            &state.unit_id,
                            manual_trigger,
                            shader.map(|_| &capability),
                            Some(resolved),
                        )
                    },
                )?;
                let Some((configuration, shader, mut sources)) = plan else {
                    *state.preparation_error.borrow_mut() = None;
                    return Ok(());
                };
                if let Some(runtime) = &shader {
                    runtime.mark_context(&mut sources);
                }
                let ticket =
                    new_unit_id().map_err(|_| crate::ui_text::text("app-preparation-id-failed"))?;
                let request_id =
                    new_unit_id().map_err(|_| crate::ui_text::text("app-execution-id-failed"))?;
                let plan = Configured {
                    identity: selected.catalog.identity.clone(),
                    request_id,
                    configuration,
                    timeout: std::time::Duration::from_secs(120),
                    transport: crate::loom_art_wire::OutputTransport::SharedMemoryAndWebsocket,
                };
                self.input_jobs
                    .start(
                        ticket.clone(),
                        state.unit_id.clone(),
                        &state.canvas.borrow().editor,
                        sources,
                    )
                    .map_err(|_| crate::ui_text::text("app-input-preparation-start-failed"))?;
                *state.pending_preparation.borrow_mut() = Some(Pending {
                    shader,
                    dependencies,
                    ticket,
                    topology: self.workspace.topology_stamp(),
                    selection: request.stamp,
                    plan,
                });
                *state.preparation_error.borrow_mut() = None;
                Ok(())
            })();
            if let Err(message) = result {
                *state.preparation_error.borrow_mut() = Some(message.into());
                if visible && manual_trigger {
                    MessageBoxW(
                        Some(document.window.0),
                        &HSTRING::from(message),
                        w!("Otter"),
                        MB_OK | MB_ICONWARNING,
                    );
                }
            }
        }
    }
}
