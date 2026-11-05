//! 应用调度手动/参数请求；自动触发检查策略及缺图，窗口回调不编码或联网。
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
            // 旧 worker/待领取结果仍占槽时保留最新请求；完成通知会再次进入主循环。
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
                let selected = selected.as_ref().ok_or("尚未选择能力")?;
                let capability = selected
                    .catalog
                    .handshake
                    .arts()
                    .iter()
                    .filter_map(crate::loom_capability::Capability::new)
                    .find(|capability| capability.id == selected.parameters.art_id())
                    .filter(|capability| capability.enabled)
                    .ok_or("所选能力不可用")?;
                let mut runtime = capability.runtime();
                let shader = self.shader_installation.as_ref().filter(|_| {
                    runtime.shader && !runtime.surface && !runtime.formal_after_preview
                });
                if shader.is_some() {
                    // 只解除已接通本地取码链的 ShaderOnly 门；自动触发/Surface/正式执行策略不放宽。
                    runtime.shader = false;
                }
                if selected.execution.gate(request.trigger, runtime).is_err() {
                    if manual_trigger {
                        return Err("此能力仅支持本地预览，原生预览运行时尚未接入，未发送后端请求");
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
                    return Err("当前服务未声明节点执行操作");
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
                let ticket = new_unit_id().map_err(|_| "无法创建准备标识")?;
                let request_id = new_unit_id().map_err(|_| "无法创建执行标识")?;
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
                    .map_err(|_| "无法启动输入准备，后台任务可能仍在清理")?;
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
