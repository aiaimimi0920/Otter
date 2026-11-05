//! 同意持久化归原 profile owner；成功 commit 才提升接受权，失败不改变已提交策略。
use super::super::*;
use crate::{
    desktop::{
        device_authorization_ui::{policy::Consent, Request},
        loom_execution_signal::Notify,
    },
    projection_receive_policy::{Change, Policy},
};
use std::{path::Path, sync::atomic::AtomicBool};
#[path = "application_projection_policy_file.rs"]
mod file;
#[path = "application_projection_policy_job.rs"]
mod job;
#[path = "application_projection_policy_schedule.rs"]
mod schedule;
pub(super) use schedule::Schedule;

#[derive(Default)]
pub(in crate::desktop::application) struct Owner {
    file: Option<file::File>,
    initial: bool,
    job: Option<job::Job>,
    pub(super) schedule: Schedule,
}

impl Owner {
    pub(super) fn load(root: &Path, cancel: &AtomicBool) -> Self {
        Self {
            file: Some(file::File::load(root, cancel)),
            initial: true,
            ..Default::default()
        }
    }
    pub(super) fn poll(&mut self, consent: &mut Consent) {
        if self.initial {
            self.initial = false;
            let file = self.file.as_ref().expect("loaded policy");
            consent.install(file.policy());
            consent.install_paused(file.paused());
            consent.status = if file.writable() {
                ""
            } else {
                "接收策略不可读，已拒绝新投送"
            };
        }
        let Some(result) = self.job.as_mut().and_then(job::Job::poll) else {
            return;
        };
        self.job.take();
        consent.finish_save();
        match result.and_then(file::Prepared::commit) {
            Ok((next, retained)) => {
                consent.install(next.policy());
                consent.install_paused(next.paused());
                self.file = Some(next);
                self.schedule.reset();
                consent.status = if retained {
                    "接收策略已保存，旧恢复文件保留"
                } else {
                    "接收策略已保存"
                };
            }
            Err(_) => consent.status = "保存接收策略失败",
        }
    }
    fn request(&mut self, change: impl Into<Change>, consent: &mut Consent, notify: Notify) {
        if self.job.is_some() || consent.saving {
            return;
        }
        let change = change.into();
        if match change {
            Change::Policy(policy) => consent.policy() == policy,
            Change::Paused(paused) => consent.paused() == paused,
        } {
            return;
        }
        let Some(file) = self.file.as_ref().filter(|file| file.writable()) else {
            consent.status = "保存接收策略失败";
            return;
        };
        match job::Job::start(file.clone(), change, notify) {
            Ok(job) => {
                self.job = Some(job);
                match change {
                    Change::Policy(_) => consent.begin_save(),
                    Change::Paused(_) => consent.begin_pause(),
                }
                self.schedule.reset();
            }
            Err(_) => consent.status = "保存接收策略失败",
        }
    }
}
impl Application {
    pub(in crate::desktop::application) fn process_projection_policy(&mut self) {
        let mut view = self.authorization.borrow_mut();
        let owner = match &mut self.projection_profile {
            super::profile::Owner::Ready { policy, .. } => Some(policy),
            _ => None,
        };
        let mut owner = owner;
        if view.closed || self.exit_requested.get() {
            if let Some(owner) = owner {
                owner.job.take();
            }
            view.consent.finish_save();
            return;
        }
        if let Some(owner) = owner.as_mut() {
            owner.poll(&mut view.consent);
        }
        let change = match &view.request {
            Some(Request::ReceivePolicy(policy)) => Some(Change::Policy(*policy)),
            Some(Request::ReceivePause(paused)) => Some(Change::Paused(*paused)),
            _ => None,
        };
        if let Some(change) = change {
            view.request.take();
            if view.closed || self.exit_requested.get() || self.modals.blocked() {
                return;
            }
            if let Some(owner) = owner {
                owner.request(change, &mut view.consent, self.execution_wake.callback());
                if matches!(change, Change::Paused(_)) && view.consent.saving {
                    // 立即清空旧菜单/凭证；网络仍由原单槽回收，失败或恢复也不复活旧快照。
                    view.projection.entries.clear();
                    view.projection.participation = None;
                }
            } else {
                view.consent.status = "保存接收策略失败";
            }
        }
    }
}

#[cfg(test)]
#[path = "application_projection_policy_tests.rs"]
mod tests;
