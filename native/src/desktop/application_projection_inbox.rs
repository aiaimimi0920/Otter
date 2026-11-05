//! 应用结算点的收件菜单消费者；先处理切换/撤权再收结果，不持 UI 借用跨线程或磁盘。
use super::*;
use crate::desktop::{
    composition::relay::sessions::Sessions,
    device_authorization_ui::{projection as ui, View},
};
use crate::{
    device_authorization_config::Config,
    device_authorization_owner::Owner,
    projection_delivery_runtime::{Error as JobError, Output},
};
use std::sync::Arc;
#[path = "application_projection_cleanup.rs"]
pub(super) mod cleanup;
#[path = "application_projection_policy.rs"]
pub(super) mod policy;
#[path = "application_projection_profile.rs"]
pub(super) mod profile;
#[path = "application_projection_receive.rs"]
pub(super) mod receive;
#[path = "application_projection_targets.rs"]
pub(super) mod targets;

pub(super) fn reset(
    remote: &mut Sessions,
    view: &mut View,
    receive: &mut receive::Owner,
    cleanup: &mut cleanup::Owner,
) {
    receive.stop(cleanup);
    remote.cancel_projection();
    ui::reset(view);
    crate::desktop::device_authorization_ui::targets::reset(view);
}
pub(super) fn start(
    view: &mut View,
    request: ui::Request,
    owner: &mut Owner,
    remote: &mut Sessions,
    profile: &profile::Owner,
    config: &mut impl FnMut(
        &str,
    )
        -> std::result::Result<Config, crate::device_authorization_config::Error>,
) -> bool {
    if view.closed
        || view.busy
        || view.projection.busy
        || view.targets.busy
        || remote.projection_pending()
    {
        view.projection.status = "投送收件任务不可用".into();
        return false; // 不覆盖仍在运行任务的 busy，也不清除其候选。
    }
    let refresh = matches!(request, ui::Request::Refresh);
    if refresh {
        view.projection.entries.clear();
    }
    let result = (|| {
        if !matches!(
            request,
            ui::Request::Compensate(_) | ui::Request::Display(_)
        ) {
            view.projection.participation =
                Some(view.consent.participation().ok_or("本机接收已暂停")?);
        }
        if matches!(request, ui::Request::Accept(_)) && view.consent.stamp().is_none() {
            return Err("当前接收策略不允许新投送".into());
        }
        let origin = view
            .origin
            .as_deref()
            .ok_or_else(|| "尚未选择服务".to_owned())?;
        if let ui::Request::Reject(entry) | ui::Request::Accept(entry) = &request {
            if entry.accepted()
                || entry.origin() != origin
                || !view
                    .projection
                    .entries
                    .iter()
                    .any(|current| Arc::ptr_eq(current, entry))
            {
                return Err("投送选择已失效，请刷新收件箱".into());
            }
        }
        if let ui::Request::Reattach(entry) = &request {
            if !entry.accepted()
                || entry.origin() != origin
                || !view
                    .projection
                    .entries
                    .iter()
                    .any(|current| Arc::ptr_eq(current, entry))
            {
                return Err("投送恢复选择已失效，请刷新收件箱".into());
            }
        }
        if let ui::Request::Compensate(entry) = &request {
            if entry.origin() != origin
                || !profile.contains(entry)
                || !view
                    .projection
                    .recovery
                    .entries
                    .iter()
                    .any(|current| Arc::ptr_eq(current, entry))
            {
                return Err("未结投送选择已失效".into());
            }
        }
        let now =
            crate::desktop::surface_windows::now_ms().map_err(|_| "系统时间无效".to_owned())?;
        let lease = owner
            .lease(origin, now)
            .map_err(|e| format!("请先获取有效授权：{e:?}"))?;
        let config = config(origin).map_err(|e| format!("配置无效：{e:?}"))?;
        match request {
            ui::Request::Display(_) => return Err("呈现恢复必须经过当前工作区核验".into()),
            ui::Request::Refresh => remote.query_projection_inbox(config, lease),
            ui::Request::Reject(entry) => remote.reject_projection(config, lease, entry),
            ui::Request::Compensate(entry) => remote.compensate_projection(config, lease, entry),
            ui::Request::Accept(entry) => {
                let profile::Owner::Ready { store, .. } = profile else {
                    return Err("接收存储尚未就绪".into());
                };
                remote.accept_projection(config, lease, entry, store.clone())
            }
            ui::Request::Reattach(entry) => {
                let profile::Owner::Ready { store, .. } = profile else {
                    return Err("接收存储尚未就绪".into());
                };
                remote.reattach_projection(config, lease, entry, store.clone())
            }
        }
        .map_err(|e| format!("投送任务未启动：{e}"))
    })();
    let started = result.is_ok();
    view.projection.busy = started;
    view.projection.status = match result {
        Ok(()) => if refresh {
            "正在查询投送收件箱…"
        } else {
            "正在处理投送…"
        }
        .into(),
        Err(error) => error,
    };
    started
}
impl Application {
    /// 只查询实际发布/规范启动恢复的索引；Unit 存在与相同原文只是当前作用域围栏。
    pub(super) fn projection_checkpoint_inclusion(
        &self,
        ticket: &crate::projection_delivery::journal::Ticket,
    ) -> Option<&crate::projection_delivery::binding::Binding> {
        if self.restore_job.is_some() {
            return None;
        }
        let owner = self.workspace_checkpoint.as_ref()?;
        let (index, source) = owner.projection_baseline()?;
        if !self
            .workspace
            .units()
            .iter()
            .any(|unit| unit.as_str() == ticket.unit())
            || self
                .restored_session
                .as_ref()
                .is_none_or(|record| !record.original.same_storage(source))
        {
            return None;
        }
        index.for_ticket(ticket)
    }
    pub(super) fn reset_projection_inbox(&mut self) {
        self.stop_projection_sender();
        reset(
            &mut self.remote_sessions,
            &mut self.authorization.borrow_mut(),
            &mut self.projection_receive,
            &mut self.projection_cleanup,
        );
    }
    pub(super) fn poll_projection_inbox(&mut self) {
        let pending = self.remote_sessions.projection_pending();
        let targets_pending = self.remote_sessions.projection_targets_pending();
        let event = self.remote_sessions.poll_projection();
        if pending && !targets_pending && !self.remote_sessions.projection_pending() {
            self.projection_profile.refresh();
        }
        self.collect_projection_abandoned(); // 必须早于 closed/origin/discard 的 UI 过滤。
        self.collect_projection_sender();
        let Some(event) = event else {
            return;
        };
        if event.sender {
            unsafe {
                self.finish_projection_sender(event.result, event.sender_cleanup);
            }
            return;
        }
        if event.targets {
            targets::finish(
                &mut self.authorization.borrow_mut(),
                &event.origin,
                event.result,
            );
            return;
        }
        if let Ok(Output::Received(ready)) = event.result {
            self.receive_projection_ready(*ready);
            return;
        }
        self.projection_receive.finish_metadata();
        let mut view = self.authorization.borrow_mut();
        if view.closed || view.origin.as_deref() != Some(event.origin.as_str()) {
            return;
        }
        view.projection.busy = false;
        if matches!(event.result, Ok(Output::Inbox(_) | Output::Rejected))
            && !view
                .projection
                .participation
                .take()
                .is_some_and(|stamp| view.consent.participates(&stamp))
        {
            return; // 暂停/恢复或失败写入都不能复活旧收件快照，不触碰补偿或已保存的持续同步。
        }
        if event.result.is_ok() {
            if let profile::Owner::Ready { policy, .. } = &mut self.projection_profile {
                policy
                    .schedule
                    .completed(&event.origin, std::time::Instant::now());
            }
        }
        match event.result {
            Ok(Output::Targets(_)) => unreachable!("targets are consumed by the directory owner"),
            Ok(Output::Inbox(inbox)) => {
                let offline = inbox.has_offline_peers();
                view.projection.entries = inbox.into_entries().into_iter().map(Arc::new).collect();
                if let profile::Owner::Ready { policy, .. } = &mut self.projection_profile {
                    policy.schedule.inbox(&view.projection.entries);
                }
                view.projection.status = format!(
                    "投送收件箱：{} 项{}",
                    view.projection.entries.len(),
                    if offline {
                        " · 离线来源未查询"
                    } else {
                        ""
                    }
                );
            }
            Ok(Output::Rejected) => {
                if let Some(entry) = event.rejected {
                    view.projection
                        .entries
                        .retain(|current| !Arc::ptr_eq(current, &entry));
                }
                view.projection.status = "已拒绝投送".into();
            }
            Ok(Output::Received(_)) => {
                unreachable!("received owner consumed before borrowing view")
            }
            Ok(Output::Compensated) => {
                view.projection.status = "投送解绑已确认".into();
            }
            Ok(Output::Displayed) => unreachable!("displayed is consumed by the receive owner"),
            Ok(Output::Synchronized(_)) => unreachable!("sync is consumed by the receive owner"),
            Ok(Output::Edited(_) | Output::EditAcknowledged) => {
                unreachable!("edit results require the collaboration owner")
            }
            Ok(Output::Sent(_) | Output::SenderUpdated(_) | Output::SenderUnlinked) => {
                unreachable!("sender results require their own consumer")
            }
            Err(JobError::Stopped) => {
                view.projection.entries.clear();
                view.projection.status = "投送操作已取消，请重新获取授权".into();
            }
            Err(error) => {
                if let profile::Owner::Ready { policy, .. } = &mut self.projection_profile {
                    policy
                        .schedule
                        .failure(&event.origin, std::time::Instant::now());
                }
                view.projection.status = format!("投送操作失败：{error:?}");
            }
        }
    }
    pub(super) fn collect_projection_abandoned(&mut self) {
        if let Some(ticket) = self.remote_sessions.take_projection_abandoned() {
            self.projection_cleanup.abandon(ticket);
        }
    }
}
