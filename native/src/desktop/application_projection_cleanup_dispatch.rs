//! 从原 checkpoint/工作区身份取得删除资格；持久标记完成后才用当前原设备授权减权。
use super::*;
use crate::device_authorization_config::Config;

impl Application {
    pub(in crate::desktop::application) unsafe fn collect_deleted_projections(&mut self) {
        if self.exit_requested.get() || self.checkpoint_is_exiting() {
            return; // 普通退出不是删除；整体 workspace 交换也不经过 closed 列表。
        }
        if matches!(self.projection_profile, profile::Owner::Disabled) {
            return;
        }
        for doc in self.documents.iter().filter(|doc| !doc.is_alive()) {
            if let Some(binding) = self.workspace_checkpoint.as_ref().and_then(|owner| {
                owner.projection_deleted_binding(&self.workspace, doc.state.unit_id.as_str())
            }) {
                if !self.projection_cleanup.deleted.contains(binding) {
                    self.projection_cleanup.deleted.push(binding.clone());
                }
            }
        }
    }
    pub(in crate::desktop::application) fn process_projection_cleanup(
        &mut self,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        let now = Instant::now();
        self.projection_cleanup.resolve(&self.projection_profile);
        let owner = &mut self.projection_cleanup;
        owner.wake = None;
        if owner.mark.as_ref().is_some_and(mark::Job::finished) {
            match owner.mark.take().unwrap().finish() {
                Ok(ticket) => {
                    drop(ticket);
                    owner.queued.remove(0);
                    owner.mark_due = None;
                    owner.mark_failures = 0;
                    self.projection_profile.refresh();
                }
                Err(error) => {
                    owner.fail_mark(now);
                    self.authorization.borrow_mut().projection.status =
                        format!("未结投送意图尚未保存：{error:?}");
                }
            }
        }
        if owner.network.is_some() && !self.remote_sessions.projection_pending() {
            let ticket = owner.network.take().unwrap();
            if self.projection_profile.recorded(&ticket) {
                owner.retry_network(ticket, now);
            }
        }
        owner.prune(&self.projection_profile);
        if owner.mark.is_none() && !owner.queued.is_empty() {
            if owner.mark_due.is_none_or(|due| due <= now) {
                match mark::Job::start(owner.queued[0].retry(), self.execution_wake.callback()) {
                    Ok(job) => owner.mark = Some(job),
                    Err(_) => owner.fail_mark(now),
                }
            }
            owner.wake = owner.mark.is_none().then_some(owner.mark_due).flatten();
        }
        if owner.pending_local()
            || owner.network.is_some()
            || self.remote_sessions.projection_pending()
            || self.projection_receive.busy()
            || self.modals.blocked()
            || self.exit_requested.get()
            || self.restore_job.is_some()
        {
            return;
        }
        let view = self.authorization.borrow();
        if view.closed || view.busy || view.projection.busy {
            return;
        }
        let Some(origin) = view.origin.as_deref() else {
            return;
        };
        let Some(entries) = self.projection_profile.entries() else {
            return;
        };
        if !entries
            .iter()
            .any(|ticket| ticket.cleanup_pending() && ticket.origin() == origin)
        {
            return;
        }
        let Ok(now_ms) = crate::desktop::surface_windows::now_ms() else {
            return;
        };
        let Ok(lease) = self.authorization_owner.lease(origin, now_ms) else {
            return;
        };
        let mut selected = None;
        for ticket in entries.iter().filter(|ticket| {
            ticket.cleanup_pending()
                && ticket.origin() == origin
                && ticket.device() == lease.device_id()
        }) {
            if let Some(due) = owner.due(ticket).filter(|due| *due > now) {
                owner.wake = Some(owner.wake.map_or(due, |old| old.min(due)));
            } else {
                selected = Some(ticket.clone());
                break;
            }
        }
        let Some(ticket) = selected else { return };
        let prepared = config(origin);
        drop(view);
        let started = prepared.ok().is_some_and(|config| {
            self.remote_sessions
                .compensate_projection(config, lease, ticket.clone())
                .is_ok()
        });
        if started {
            owner.network = Some(ticket);
            owner.wake = None;
            let mut view = self.authorization.borrow_mut();
            view.projection.busy = true;
            view.projection.status = "正在清理未结投送…".into();
        } else {
            owner.retry_network(ticket.clone(), now);
            owner.wake = owner.due(&ticket);
        }
    }
}
