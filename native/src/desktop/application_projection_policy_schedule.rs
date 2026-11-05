//! 有界公平调度：NEW 与缺失记录的 accepted 分流，均复用 Sessions 单槽及原接收 owner。
use super::*;
use crate::{
    desktop::device_authorization_ui::projection::Request, projection_delivery::Invitation,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone, PartialEq, Eq)]
struct Key(String, u64, String, bool, String, Option<String>);
impl Key {
    fn matches(&self, entry: &Invitation) -> bool {
        self.0 == entry.envelope().projection_id()
            && self.1 == entry.revision()
            && self.2 == entry.digest()
            && self.3 == entry.accepted()
            && self.4 == entry.origin()
            && self.5.as_deref() == entry.peer_id()
    }
}
fn key(entry: &Invitation) -> Key {
    Key(
        entry.envelope().projection_id().to_owned(),
        entry.revision(),
        entry.digest().to_owned(),
        entry.accepted(),
        entry.origin().into(),
        entry.peer_id().map(str::to_owned),
    )
}
fn missing(
    entry: &Invitation,
    records: &[Arc<crate::projection_delivery::journal::Ticket>],
    units: &[crate::workspace_id::UnitId],
) -> bool {
    let unit_id = entry.receiver_unit_id();
    !units.iter().any(|unit| unit.as_str() == unit_id)
        && !records.iter().any(|ticket| {
            ticket.origin() == entry.origin()
                && ticket.invitation().peer_id() == entry.peer_id()
                && ticket.envelope().projection_id() == entry.envelope().projection_id()
        })
}
#[derive(Default)]
pub(in crate::desktop::application) struct Schedule {
    origin: Option<String>,
    attempted: Vec<Key>,
    last: Option<String>,
    next: Option<Instant>,
    pub(super) wake: Option<Instant>,
    failures: u32,
}
impl Schedule {
    #[cfg(test)]
    pub(in crate::desktop::application) fn defer_for_test(&mut self, origin: &str) {
        // 手动协议夹具独占其服务器脚本；只移动真实 deadline，不改变生产调度分支。
        self.origin = Some(origin.to_owned());
        self.next = Some(Instant::now() + Duration::from_secs(3600));
        self.wake = self.next;
    }
    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }
    pub(in crate::desktop::application) fn inbox(&mut self, entries: &[Arc<Invitation>]) {
        self.attempted
            .retain(|old| entries.iter().any(|entry| old.matches(entry)));
        self.failures = 0;
    }
    fn align_origin(&mut self, origin: &str) {
        if self.origin.as_deref() != Some(origin) {
            self.reset();
            self.origin = Some(origin.to_owned());
        }
    }
    pub(in crate::desktop::application) fn failure(&mut self, origin: &str, now: Instant) {
        self.align_origin(origin);
        self.failures = (self.failures + 1).min(4);
        self.next = now.checked_add(Duration::from_millis((3000 << self.failures).min(30_000)));
        self.wake = self.next;
    }
    pub(in crate::desktop::application) fn completed(&mut self, origin: &str, now: Instant) {
        self.align_origin(origin);
        // 间隔从真实完成算起；慢 inbox 不能立刻再占槽，须给既有关联的 750ms 同步留窗口。
        self.failures = 0;
        self.next = now.checked_add(Duration::from_secs(3));
        self.wake = self.next;
    }
    fn due(&mut self, origin: &str, now: Instant) -> bool {
        self.align_origin(origin);
        if let Some(next) = self.next.filter(|next| *next > now) {
            self.wake = Some(next);
            return false;
        }
        true
    }
    fn plan(
        &mut self,
        origin: &str,
        entries: &[Arc<Invitation>],
        policy: Policy,
        now: Instant,
    ) -> Option<Request> {
        if !self.due(origin, now) {
            return None;
        }
        self.next = now.checked_add(Duration::from_secs(3));
        self.wake = self.next; // 启动前失败也保有界唤醒，不依赖不存在的 worker 通知。
        let count = entries
            .len()
            .min(crate::projection_delivery::MAX_INVITATIONS);
        let start = entries
            .iter()
            .position(|entry| Some(entry.envelope().projection_id()) == self.last.as_deref())
            .map_or(0, |index| index + 1);
        for offset in 0..count {
            let entry = &entries[(start + offset) % count];
            if entry.origin() != origin
                || self.attempted.iter().any(|old| old.matches(entry))
                || (!entry.accepted() && policy == Policy::Confirm)
            {
                continue;
            }
            // 快照最多 64 项；满额时只查询，不淘汰旧凭证以反复重试同一失败邀请。
            if self.attempted.len() >= crate::projection_delivery::MAX_INVITATIONS {
                break;
            }
            self.attempted.push(key(entry));
            self.last = Some(entry.envelope().projection_id().to_owned());
            return Some(if entry.accepted() {
                Request::Reattach(entry.clone())
            } else if policy == Policy::Reject {
                Request::Reject(entry.clone())
            } else {
                Request::Accept(entry.clone())
            });
        }
        Some(Request::Refresh)
    }
}
impl Application {
    #[cfg(test)]
    pub(in crate::desktop::application) fn resume_projection_poll_for_test(&mut self) {
        if let super::super::profile::Owner::Ready { policy, .. } = &mut self.projection_profile {
            policy.schedule.reset();
        }
    }
    #[cfg(test)]
    pub(in crate::desktop::application) fn defer_projection_poll_for_test(&mut self, origin: &str) {
        if let super::super::profile::Owner::Ready { policy, .. } = &mut self.projection_profile {
            policy.schedule.defer_for_test(origin);
        }
    }
    pub(in crate::desktop::application) fn projection_policy_deadline(&self) -> Option<Instant> {
        match &self.projection_profile {
            super::super::profile::Owner::Ready { policy, .. } => policy.schedule.wake,
            _ => None,
        }
    }
    pub(in crate::desktop::application) unsafe fn process_projection_automatic(
        &mut self,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            crate::device_authorization_config::Config,
            crate::device_authorization_config::Error,
        >,
        now: Instant,
    ) {
        let super::super::profile::Owner::Ready { policy, .. } = &mut self.projection_profile
        else {
            return;
        };
        policy.schedule.wake = None;
        let view = self.authorization.borrow();
        let mode = view.consent.policy();
        let origin = view.origin.clone();
        let idle = !view.closed
            && !view.busy
            && !view.projection.busy
            && view.request.is_none()
            && !view.consent.saving
            && view.consent.participation().is_some()
            && !self.exit_requested.get()
            && !self.modals.blocked()
            && !self.projection_receive.busy()
            && !self.projection_edit.busy()
            && !self.remote_sessions.projection_pending()
            && self.save_job.is_none()
            && self.restore_job.is_none()
            && !self.checkpoint_is_exiting();
        if !idle {
            return;
        }
        let Some(origin) = origin else { return };
        if !crate::desktop::surface_windows::now_ms()
            .is_ok_and(|now| self.authorization_owner.token(&origin, now).is_some())
        {
            return;
        }
        if let super::super::profile::Owner::Ready { policy, .. } = &mut self.projection_profile {
            // 活跃窗口可频繁唤醒应用；deadline 未到不得准备 Scope/候选或分配 UnitId。
            if !policy.schedule.due(&origin, now) {
                return;
            }
        }
        // 先检查目标，再占候选；暂不能追加不耗费该邀请唯一自动尝试。
        drop(view);
        let existing = super::super::receive::Scope::capture_existing(self).ok();
        let scope = if mode == Policy::Auto {
            super::super::receive::Scope::capture(self).ok()
        } else {
            None
        };
        let mut view = self.authorization.borrow_mut();
        // 有 journal 的关联可能归其他已保存工作区；不得后台迁移、覆盖或排补偿。
        let entries: Vec<_> = view
            .projection
            .entries
            .iter()
            .filter(|entry| {
                (!entry.accepted()
                    && (mode == Policy::Reject || (mode == Policy::Auto && scope.is_some())))
                    || (entry.accepted()
                        && existing.is_some()
                        && missing(
                            entry,
                            self.projection_profile.entries().unwrap_or(&[]),
                            self.workspace.units(),
                        ))
            })
            .cloned()
            .collect();
        let super::super::profile::Owner::Ready { policy, .. } = &mut self.projection_profile
        else {
            return;
        };
        let Some(request) = policy.schedule.plan(&origin, &entries, mode, now) else {
            return;
        };
        let scope = if matches!(request, Request::Reattach(_)) {
            existing
        } else {
            scope
        };
        if super::super::start(
            &mut view,
            request,
            &mut self.authorization_owner,
            &mut self.remote_sessions,
            &self.projection_profile,
            config,
        ) {
            self.projection_receive.waiting(scope);
        } else if let super::super::profile::Owner::Ready { policy, .. } =
            &mut self.projection_profile
        {
            policy.schedule.failure(&origin, now);
        }
    }
}

#[cfg(test)]
#[path = "application_projection_policy_schedule_tests.rs"]
mod tests;
