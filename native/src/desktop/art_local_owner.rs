//! Application 的本地 Art 所有权：宿主选择凭证、下载 Registry 和待回收窗口归一个工作区。
//! reset 只撤销/关窗，不 join；失效记录保留到后台终态，不能用新工作区接纳旧帧。
use super::composition::live::session::Session as LiveSession;
use super::{
    art_local_tasks::Tasks, art_local_window::Window, configuration_stamp::Stamp,
    document_window::DocumentWindow,
};
use crate::{
    art_issuer_trust::Authority, art_local_requests::description,
    device_authorization_config::Config, device_authorization_lease::Lease,
    graph_requests::Request as Token, workspace::Workspace, workspace_id::UnitId,
};
use std::sync::Arc;
const MAX_RECORDS: usize = 4;
#[path = "art_local_dependencies.rs"]
mod dependencies;
#[path = "art_local_images.rs"]
mod images;
#[path = "art_local_mounting.rs"]
mod mounting;
#[path = "art_local_ports.rs"]
mod ports;
struct Entry {
    unit: UnitId,
    instance: String,
    token: Token,
    stamp: Stamp,
    inputs: dependencies::Stamp,
    images: images::Images,
    result_stamp: super::loom_result_state::OutputStamp,
    active: bool,
    window: Option<Window>,
}
pub(super) struct Owner {
    workspace: Option<String>,
    tasks: Tasks,
    entries: Vec<Entry>,
    notify: Arc<dyn Fn() + Send + Sync>,
    status_owner: Option<Token>,
}
impl Owner {
    #[cfg(test)]
    pub(super) fn test_result_ready(&self) -> bool {
        self.entries
            .iter()
            .any(|e| e.active && e.window.as_ref().is_some_and(Window::test_result_ready))
    }
    pub(super) fn accept_triggers(
        &mut self,
        tickets: Vec<crate::art_live_trigger::ticket::Ticket>,
    ) {
        for ticket in tickets {
            let window = self
                .entries
                .iter_mut()
                .filter(|entry| entry.active)
                .filter_map(|entry| entry.window.as_mut())
                .find(|window| {
                    window
                        .viewer_proof()
                        .is_some_and(|proof| proof.binding() == ticket.binding())
                });
            if let Some(window) = window {
                window.accept_trigger(ticket);
            } else {
                ticket.complete(crate::art_live_trigger::Receipt::Cancelled);
            }
        }
    }
    pub(super) fn confirmation_handles(&self) -> Vec<windows::Win32::Foundation::HWND> {
        self.entries
            .iter()
            .filter(|entry| entry.active)
            .filter_map(|entry| entry.window.as_ref().and_then(Window::confirmation_handle))
            .collect()
    }
    #[cfg(test)]
    pub(super) fn test_window(&self) -> Option<&super::surface_window::SurfaceWindow> {
        self.entries
            .iter()
            .find(|entry| entry.active)
            .and_then(|entry| entry.window.as_ref())
            .and_then(Window::test_window)
    }
    #[cfg(test)]
    pub(super) fn test_counts(&self) -> (usize, usize) {
        (
            self.entries.len(),
            self.entries
                .iter()
                .filter(|entry| entry.active && entry.window.as_ref().is_some_and(Window::ready))
                .count(),
        )
    }
    pub fn new(notify: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self {
            workspace: None,
            tasks: Tasks::new(String::new(), notify.clone()),
            entries: Vec::new(),
            notify,
            status_owner: None,
        }
    }
    pub fn busy(&self) -> bool {
        self.tasks.busy()
            || self.entries.iter().any(|entry| {
                entry.images.busy()
                    || (entry.active && entry.window.as_ref().is_none_or(|window| !window.ready()))
            })
    }
    pub(super) fn session_end_idle(&self) -> bool {
        !self.busy()
            && self
                .entries
                .iter()
                .filter(|entry| entry.active)
                .all(|entry| entry.window.as_ref().is_some_and(Window::session_end_idle))
    }
    pub(super) fn viewer_proofs(
        &self,
        documents: &[DocumentWindow],
        workspace: &Workspace,
        sessions: &[LiveSession],
    ) -> Vec<Arc<crate::live_relay_http::art_prepare::viewer::Proof>> {
        self.entries
            .iter()
            .filter(|entry| entry.active && current(entry, documents, workspace, sessions))
            .filter_map(|entry| entry.window.as_ref().and_then(Window::viewer_proof))
            .collect()
    }
    pub fn deadline(&self) -> Option<std::time::Instant> {
        self.entries
            .iter()
            .filter(|entry| entry.active)
            .filter_map(|entry| entry.window.as_ref().and_then(Window::deadline))
            .min()
    }
    pub fn start(
        &mut self,
        document: &DocumentWindow,
        workspace: &Workspace,
        art: String,
        lease: Arc<Lease>,
        authority: &mut Authority,
        config: Config,
    ) -> Result<(), &'static str> {
        // owner 仅在宿主 UI 线程调用；DocumentWindow/State 在本次借用期间保活。
        if !unsafe { document.is_alive() } || document.state.group_locked.get() {
            return Err("Art 宿主已关闭或锁定");
        }
        if !self.tasks.has_capacity() || self.entries.len() >= MAX_RECORDS {
            return Err("Art 正在执行或回收，请稍后再试");
        }
        self.entries
            .try_reserve(1)
            .map_err(|_| "无法保留 Art 状态")?;
        if self.workspace.is_none() {
            let workspace = super::document_state::new_unit_id()
                .map_err(|_| "无法创建 Art 工作区身份")?
                .to_string();
            self.tasks.reset_workspace(workspace.clone());
            self.workspace = Some(workspace);
        }
        let instance = super::document_state::new_unit_id()
            .map_err(|_| "无法创建 Art 实例身份")?
            .to_string();
        let unit = document.state.unit_id.clone();
        let inputs = dependencies::Stamp::capture(workspace, &unit)?;
        let request = description::Request {
            origin: config.origin().into(),
            workspace: self.workspace.clone().unwrap(),
            instance: instance.clone(),
            art,
        };
        let token = self
            .tasks
            .start_description(unit.clone(), request, lease, authority, config)
            .map_err(|_| "Art 未启动：请检查设备授权、签发者信任和配置")?;
        // 新请求已成功换代，旧窗口只能退休，不能再 cancel(unit) 误撤销新请求。
        for entry in self
            .entries
            .iter_mut()
            .filter(|entry| entry.active && entry.unit == unit)
        {
            entry.active = false;
            entry.images.cancel();
            if let Some(window) = &mut entry.window {
                let _ = window.stop();
            }
        }
        self.entries.push(Entry {
            unit,
            instance,
            token,
            stamp: Stamp::capture(&document.state),
            inputs,
            images: images::Images::default(),
            result_stamp: document.state.execution_results.borrow().output_stamp(),
            active: true,
            window: None,
        });
        Ok(())
    }
    pub fn poll(
        &mut self,
        documents: &[DocumentWindow],
        workspace: &mut Workspace,
        sessions: &[LiveSession],
        visible: bool,
    ) -> Option<String> {
        self.retire(documents, workspace, sessions);
        let mut status = self.prepare_windows(documents, workspace, sessions);
        for entry in &mut self.entries {
            let valid = entry.active && current(entry, documents, workspace, sessions);
            let unit = &entry.unit;
            let stamp = &entry.stamp;
            let inputs = &entry.inputs;
            let images = &entry.images;
            if let Some(window) = &mut entry.window {
                let host = || {
                    valid
                        && inputs.matches(workspace)
                        && images.current(documents, sessions, workspace)
                        && host_current(unit, stamp, documents, workspace)
                };
                if window.poll(host, visible).is_err() && entry.active {
                    entry.active = false;
                    entry.images.cancel();
                    self.tasks.cancel(unit);
                    status = Some("Art 视图已停止".into());
                    self.status_owner = None;
                }
                if window.finished() && entry.active {
                    entry.active = false;
                    entry.images.cancel();
                    self.tasks.cancel(unit);
                }
                if entry.active && host_current(unit, stamp, documents, workspace) {
                    if let Some(progress) = window.take_status() {
                        if status.is_none() {
                            status = Some(progress);
                            self.status_owner = Some(entry.token.clone());
                        }
                    }
                }
            }
        }
        // 当前文案也绑定请求代际；reset/换节点不能留下旧“执行中”。错误文案优先。
        if self.status_owner.as_ref().is_some_and(|token| {
            !self
                .entries
                .iter()
                .any(|entry| entry.active && &entry.token == token)
        }) {
            self.status_owner = None;
            status.get_or_insert_with(|| "Art 已停止".into());
        }
        self.entries.retain(|entry| {
            entry.active
                || entry.images.busy()
                || entry
                    .window
                    .as_ref()
                    .is_some_and(|window| !window.finished())
        });
        status
    }
    pub fn reset(&mut self) {
        self.workspace = None;
        self.tasks.reset_workspace(String::new());
        for entry in &mut self.entries {
            entry.active = false;
            entry.images.cancel();
            if let Some(window) = &mut entry.window {
                let _ = window.stop();
            }
        }
    }
    pub fn retire(
        &mut self,
        documents: &[DocumentWindow],
        workspace: &Workspace,
        sessions: &[LiveSession],
    ) {
        for entry in &mut self.entries {
            if entry.active && !current(entry, documents, workspace, sessions) {
                entry.active = false;
                entry.images.cancel();
                self.tasks.cancel(&entry.unit);
                if let Some(window) = &mut entry.window {
                    let _ = window.stop();
                }
            }
        }
    }
    pub fn shutdown(&mut self) {
        self.reset();
        self.tasks.shutdown();
        self.entries.clear(); // 应用退出：原 Lease 已全部撤销，最后 join 兜底不留 child。
    }
}
#[path = "art_local_results.rs"]
mod results;
#[cfg(test)]
#[path = "art_local_owner_tests.rs"]
mod tests;
fn host_current(
    unit: &UnitId,
    stamp: &Stamp,
    documents: &[DocumentWindow],
    workspace: &Workspace,
) -> bool {
    workspace.contains(unit)
        && documents.iter().any(|document| {
            &document.state.unit_id == unit
                && unsafe { document.is_alive() }
                && !document.state.group_locked.get()
                && stamp.matches(&document.state)
        })
}
fn current(
    entry: &Entry,
    documents: &[DocumentWindow],
    workspace: &Workspace,
    sessions: &[LiveSession],
) -> bool {
    entry.inputs.matches(workspace)
        && entry.images.current(documents, sessions, workspace)
        && host_current(&entry.unit, &entry.stamp, documents, workspace)
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.shutdown();
    }
}
