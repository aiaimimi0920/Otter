//! 唯一接收消费者：原请求围栏→后台候选→隐藏追加→精确 checkpoint→可见呈现/回执。
use super::*;
use crate::{
    device_authorization_lease::Lease,
    projection_delivery_runtime::{Hold, Ready},
    session_source::Source,
};
#[path = "application_projection_receive_scope.rs"]
mod scope;
use scope::stale;
pub(in crate::desktop::application) use scope::Scope;
#[path = "application_projection_receive_display.rs"]
mod display;
#[path = "application_projection_receive_job.rs"]
mod job;
#[path = "application_projection_receive_lifecycle.rs"]
mod lifecycle;
#[path = "application_projection_present.rs"]
mod present;
#[path = "application_projection_receive_recovery.rs"]
mod recovery;
#[path = "application_projection_sync.rs"]
mod sync;

#[derive(Default)]
pub(in crate::desktop::application) struct Owner(Phase, sync::Schedule);
#[derive(Default)]
enum Phase {
    #[default]
    Idle,
    Waiting(Option<Scope>),
    Preparing(Box<Pending>),
    Recovering(Box<recovery::Pending>),
    Syncing(Box<sync::Pending>),
    Checkpoint(Box<Saved>),
    Reporting(Box<Reporting>),
}
struct Pending {
    scope: Scope,
    origin: String,
    current: Lease,
    job: job::Job,
}
struct Reporting {
    source: Source,
    current: Lease,
    visual: present::Visual,
    job: crate::projection_delivery_runtime::Job,
    recovery: Option<crate::projection_delivery::binding::Binding>,
}
struct Saved {
    source: Source,
    held: Hold,
    expected: crate::projection_delivery::binding::Binding,
    durable: bool,
    current: Lease,
    visual: present::Visual,
    workspace: crate::workspace::IdentityStamp,
    recovery: bool,
}
impl Owner {
    #[cfg(test)]
    pub(in crate::desktop::application) fn preparation_finished(&self) -> bool {
        matches!(&self.0, Phase::Preparing(pending) if pending.job.finished())
    }
    pub(in crate::desktop::application) fn busy(&self) -> bool {
        !matches!(self.0, Phase::Idle)
    }
    pub(in crate::desktop::application) fn preparing(&self) -> bool {
        // 持续 read 不修改 UI，可与用户保存/恢复准备并存；提交时仍核原来源/Document 围栏。
        matches!(
            self.0,
            Phase::Preparing(_) | Phase::Recovering(_) | Phase::Reporting(_)
        )
    }
    pub(in crate::desktop::application) fn waiting(&mut self, scope: Option<Scope>) {
        self.0 = Phase::Waiting(scope);
    }
    pub(super) fn finish_metadata(&mut self) {
        if matches!(self.0, Phase::Waiting(_)) {
            self.0 = Phase::Idle;
            self.1.defer_foreground(std::time::Instant::now());
        }
    }
    pub(in crate::desktop::application) fn deadline(&self, now: u64) -> Option<std::time::Instant> {
        self.phase_deadline(now)
            .into_iter()
            .chain(self.1.wake)
            .min()
    }
    fn phase_deadline(&self, now: u64) -> Option<std::time::Instant> {
        let expiry = match &self.0 {
            Phase::Preparing(pending) if !pending.job.stopped() => pending.job.expires_at_ms(),
            Phase::Recovering(pending) => pending.job.expires_at_ms()?,
            Phase::Syncing(pending) => pending.job.expires_at_ms()?,
            Phase::Checkpoint(saved) => saved.held.expires_at_ms(),
            Phase::Reporting(reporting) => {
                let held = reporting.job.held()?;
                if held.lease().is_revoked() {
                    return None; // 取消后只等原线程句柄，避免已过期 deadline 的零超时忙循环。
                }
                held.expires_at_ms()
            }
            _ => return None,
        };
        std::time::Instant::now()
            .checked_add(std::time::Duration::from_millis(expiry.saturating_sub(now)))
    }
    pub(in crate::desktop::application) fn wait_handle(
        &self,
    ) -> Option<std::os::windows::io::BorrowedHandle<'_>> {
        match &self.0 {
            Phase::Reporting(reporting) => reporting.job.wait_handle(),
            Phase::Recovering(pending) => pending.job.wait_handle(),
            Phase::Syncing(pending) => pending.job.wait_handle(),
            _ => None,
        }
    }
}
impl Application {
    fn receive_lease(&mut self, origin: &str) -> Result<Lease> {
        let view = self.authorization.borrow();
        if view.closed || view.origin.as_deref() != Some(origin) || self.exit_requested.get() {
            return Err(stale());
        }
        self.authorization_owner
            .lease(origin, job::now()?)
            .map_err(|_| stale())
    }
    fn receive_hold_valid(&self, held: &Hold, current: &Lease) -> bool {
        let view = self.authorization.borrow();
        // 保留的接收租约与原请求同 Grant；缓存替换会同步永久撤销两者，无需热路径重复分配 Lease。
        !view.closed
            && view.origin.as_deref() == Some(held.origin())
            && !self.exit_requested.get()
            && self.projection_profile.recorded(held.ticket())
            && job::now().is_ok_and(|now| held.validate(current, now).is_ok())
    }
    pub(in crate::desktop::application) fn receive_projection_ready(&mut self, ready: Ready) {
        let cleanup = ready.cleanup_ticket();
        if !matches!(self.projection_receive.0, Phase::Waiting(_)) {
            drop(ready);
            self.receive_cleanup(cleanup);
            self.receive_status(Err(stale()));
            return; // 不覆盖或同步 join 另一个在途 owner。
        }
        let phase = std::mem::take(&mut self.projection_receive.0);
        let result = (|| {
            let Phase::Waiting(Some(scope)) = phase else {
                return Err(stale());
            };
            if !unsafe { scope.matches(self) } {
                return Err(stale());
            }
            let origin = ready.origin().to_owned();
            let lease = self.receive_lease(&origin)?;
            // 原 profile 的同一 Store/Record 是必需条件，不以 UUID 或相等磁盘内容代替。
            if !self.projection_profile.recorded(ready.ticket()) {
                return Err(stale());
            }
            let job = job::Job::start(
                ready,
                &lease,
                &scope,
                self.workspace.units().to_vec(),
                self.execution_wake.callback(),
            )?;
            self.projection_receive.0 = Phase::Preparing(Box::new(Pending {
                scope,
                origin,
                current: lease,
                job,
            }));
            Ok("正在准备接收投送…")
        })();
        if result.is_err() {
            self.receive_cleanup(cleanup);
        }
        self.receive_status(result);
    }
    pub(in crate::desktop::application) fn poll_projection_receive(
        &mut self,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            crate::device_authorization_config::Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        // 领取状态后复核，避免本任务的 preparing 标记把自身 Guard/追加/checkpoint 阻塞。
        let phase = std::mem::take(&mut self.projection_receive.0);
        let result = match phase {
            Phase::Preparing(mut pending) => {
                if !pending.job.valid(&pending.origin) || !unsafe { pending.scope.matches(self) } {
                    pending.job.stop();
                }
                let Some(result) = pending.job.poll() else {
                    self.projection_receive.0 = Phase::Preparing(pending);
                    return;
                };
                // 只在回收后取快照；成功 poll 已移动 Hold，不能只从 Job 取票据。
                let cleanup = match &result {
                    Ok(prepared) => prepared.held.cleanup_ticket(),
                    Err(_) => pending.job.cleanup_ticket(),
                };
                let result = match result {
                    Ok(prepared) if !pending.job.stopped() => unsafe {
                        self.publish_projection_receive(pending.scope, prepared, pending.current)
                    },
                    _ => Err(stale()),
                };
                drop(pending.job); // 即使成功结果因 stopped 被丢弃，也先释放原 Hold 后再移交。
                if result.is_err() {
                    self.receive_cleanup(cleanup);
                }
                result
            }
            Phase::Checkpoint(mut saved) => unsafe {
                if self.projection_checkpoint_inclusion(saved.held.ticket())
                    == Some(&saved.expected)
                {
                    saved.stabilize();
                }
                let cleanup = saved.held.cleanup_ticket();
                let result = self.display_projection_saved(saved, config);
                if result.is_err() {
                    self.receive_cleanup(cleanup);
                }
                result
            },
            Phase::Reporting(reporting) => unsafe { self.poll_projection_report(reporting) },
            Phase::Recovering(pending) => unsafe { self.poll_projection_recovery(pending, config) },
            other => {
                self.projection_receive.0 = other;
                return;
            }
        };
        self.receive_status(result);
    }
    unsafe fn publish_projection_receive(
        &mut self,
        scope: Scope,
        prepared: job::Prepared,
        current: Lease,
    ) -> Result<&'static str> {
        use crate::desktop::session_binding::Bound;
        use std::sync::atomic::AtomicBool;
        if !scope.matches(self) || !self.receive_hold_valid(&prepared.held, &current) {
            return Err(stale());
        }
        let cancel = AtomicBool::new(false);
        let catalog = self.catalog.borrow().loaded.clone();
        let states = Bound::prepare_for(prepared.vector, catalog.clone(), &cancel)
            .map_err(|failure| failure.1)?
            .into_parts_for(catalog.as_ref(), &cancel)
            .map_err(|_| stale())?
            .prepare_states(&cancel)?;
        let windows = self.prepare_restored_batch(states, &cancel, |placement| {
            scope.space.physical(placement.rect).map_err(|_| stale())
        })?;
        if !scope.matches(self) || !self.receive_hold_valid(&prepared.held, &current) {
            return Err(stale());
        }
        let append = self.prepare_append(windows, scope.source, prepared.retained, scope.space)?;
        self.publish_append(append, false)?;
        // 共同发布仍独占 Windows；接管后从 Application 的原 Document owner 取得呈现凭证。
        let document = self
            .documents
            .iter()
            .find(|doc| doc.state.unit_id.as_str() == prepared.held.ticket().unit())
            .ok_or_else(stale)?;
        let visual = present::Visual::capture(document)?;
        self.projection_receive.0 = Phase::Checkpoint(Box::new(Saved {
            source: self.restored_session.as_ref().unwrap().original.clone(),
            held: prepared.held,
            expected: prepared.expected,
            durable: false,
            current,
            visual,
            workspace: self.workspace.identity_stamp(),
            recovery: false,
        }));
        Ok("正在保存接收投送…")
    }
    fn receive_status(&mut self, result: Result<&str>) {
        #[cfg(test)]
        if let Err(error) = &result {
            eprintln!("projection-receive-failure: {error}");
        }
        self.projection_receive
            .1
            .defer_foreground(std::time::Instant::now());
        let mut view = self.authorization.borrow_mut();
        // Syncing 只有串行占用，不是前台 UI busy；拒绝显式 Display 不能污染其有效性。
        view.projection.busy = self.remote_sessions.projection_pending()
            || (self.projection_receive.busy()
                && !matches!(self.projection_receive.0, Phase::Syncing(_)));
        let status = result.unwrap_or("接收未完成；本地内容和未结意图保留");
        if view.projection.status != status {
            view.projection.status.clear();
            view.projection.status.push_str(status);
        }
    }
}
