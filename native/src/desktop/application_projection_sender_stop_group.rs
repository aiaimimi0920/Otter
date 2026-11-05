//! 组停止只拥有最多八项原Request及来源围栏；持久减权/patch/unlink/ack仍由原stop单槽执行。
use super::*;
use crate::desktop::device_authorization_ui::sender::groups::StopSelection;
use crate::session_source::Source;
#[path = "application_projection_sender_stop_group_flow.rs"]
mod flow;

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Queued,
    Starting,
    Pending,
    Stopped,
    Failed,
    Cancelled,
}
struct Entry {
    binding: Binding,
    request: Option<Request>,
    state: State,
}
pub(super) struct Batch {
    selection: StopSelection,
    workspace: crate::workspace::IdentityStamp,
    scope: String,
    canonical: Source,
    original: Source,
    entries: Vec<Entry>,
    valid: bool,
    dirty: bool,
}
impl Batch {
    fn capture(app: &Application, selection: StopSelection) -> Option<Self> {
        let view = app.authorization.borrow();
        let current = app.workspace_checkpoint.as_ref()?.sender_source()?;
        if !app.sender_stop_available()
            || app.projection_sender.stop.busy
            || app.projection_sender.stop_group.is_some()
            || !current.same_storage(&selection.source)
            || !view.targets.directory.as_ref().is_some_and(|directory| {
                std::sync::Arc::ptr_eq(directory, &selection.group.directory)
            })
            || !crate::desktop::device_authorization_ui::sender::groups::stop_enabled(
                &view,
                &selection.group.unit,
                selection.group.index,
            )
        {
            return None;
        }
        let group = selection
            .group
            .directory
            .groups()?
            .get(selection.group.index)?;
        let scope = app.projection_sender_scope()?.to_string();
        let requests = app.projection_sender.profile.entries()?;
        let entries = app
            .workspace_checkpoint
            .as_ref()?
            .sender_index()?
            .entries()
            .iter()
            .filter(|binding| {
                binding.unit() == selection.group.unit.as_str()
                    && binding.scope() == scope
                    && binding.envelope().origin() == selection.group.directory.origin()
                    && group.target_ids().iter().any(|id| id == binding.target())
            })
            .map(|binding| Entry {
                binding: binding.clone(),
                request: requests
                    .iter()
                    .find(|request| binding.matches_intent(request.intent()))
                    .cloned(),
                state: State::Queued,
            })
            .collect::<Vec<_>>();
        if entries.is_empty()
            || entries.len() > crate::projection_delivery::sender::binding::MAX_PER_UNIT
        {
            return None;
        }
        Some(Self {
            workspace: app.workspace.identity_stamp(),
            scope,
            canonical: current.clone(),
            original: app.restored_session.as_ref()?.original.clone(),
            selection,
            entries,
            valid: true,
            dirty: true,
        })
    }
    unsafe fn matches(&mut self, app: &Application) -> bool {
        let view = app.authorization.borrow();
        let current = app
            .workspace_checkpoint
            .as_ref()
            .and_then(|owner| owner.sender_source());
        let valid = self.valid
            && self.workspace.matches(&app.workspace)
            && !view.closed
            && view.origin.as_deref() == Some(self.selection.group.directory.origin())
            && view.targets.directory.as_ref().is_some_and(|directory| {
                std::sync::Arc::ptr_eq(directory, &self.selection.group.directory)
            })
            && app.restore_job.is_none()
            && app
                .restored_session
                .as_ref()
                .and_then(|record| record.projection_scope)
                .is_some_and(|scope| scope.to_string() == self.scope)
            && app
                .documents
                .iter()
                .any(|doc| doc.is_alive() && doc.state.unit_id == self.selection.group.unit)
            && app
                .restored_session
                .as_ref()
                .is_some_and(|record| record.original.same_storage(&self.original))
            && current.is_some_and(|source| {
                source.same_storage(&self.canonical) || source.same_storage(&self.original)
            });
        if valid {
            self.canonical = current.unwrap().clone();
        }
        valid
    }
    // 仅原stop Context已重新核验的patch可推进围栏；外部同字节原文不能重抓成新基准。
    pub(super) fn patched(&mut self, canonical: &Source, original: &Source, next: &Source) {
        self.valid &= original.same_storage(&self.original)
            && (canonical.same_storage(&self.canonical) || canonical.same_storage(&self.original));
        if self.valid {
            self.canonical = canonical.clone();
            self.original = next.clone();
        }
    }
    // canonical编码有独立allocation；仅真实Checkpoint提交回执可以登记它，不能由轮询重抓。
    pub(super) fn committed(
        &mut self,
        index: Option<&Index>,
        canonical: &Source,
        original: &Source,
        workspace: &crate::workspace::Workspace,
    ) {
        self.valid &= self.workspace.matches(workspace)
            && original.same_storage(&self.original)
            && index.is_some_and(|index| index.is_for(canonical));
        if self.valid {
            self.canonical = canonical.clone();
        }
    }
    fn cancel(&mut self) {
        for entry in &mut self.entries {
            if entry.state == State::Queued {
                entry.state = State::Cancelled;
            }
        }
        self.dirty = true;
    }
    fn report(&mut self, app: &Application) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        let count = |state| {
            self.entries
                .iter()
                .filter(|entry| entry.state == state)
                .count()
        };
        let group = &self.selection.group.directory.groups().unwrap()[self.selection.group.index];
        let mut view = app.authorization.borrow_mut();
        view.sender.group_stopping = true;
        view.sender.group_unit = Some(self.selection.group.unit.clone());
        view.sender.group_results = vec![format!(
            "停止组 {}：{} 已确认 · {} 待确认 · {} 失败 · {} 未受理",
            group.name(),
            count(State::Stopped),
            count(State::Starting) + count(State::Pending),
            count(State::Failed),
            count(State::Queued) + count(State::Cancelled),
        )];
        view.sender
            .group_results
            .extend(self.entries.iter().map(|entry| {
                let state = match entry.state {
                    State::Queued => "等待受理",
                    State::Starting => "正在持久受理；取消不撤销已发布意图",
                    State::Pending => "已持久受理，等待解绑/保存/确认",
                    State::Stopped => "停止已确认",
                    State::Failed => "未受理：持久减权失败或缺少原意图",
                    State::Cancelled => "未受理/已取消",
                };
                format!(
                    "{}：{state}",
                    if entry.binding.name().is_empty() {
                        entry.binding.target()
                    } else {
                        entry.binding.name()
                    }
                )
            }));
    }
}
