//! 自动单向 receiver PNG 同步；原 accept/checkpoint/report 状态优先，不再建网络 owner。
use super::*;
use crate::{
    device_authorization_config::Config,
    projection_delivery::{binding::Binding, journal::Ticket},
    projection_delivery_runtime::{
        sync::{Outcome, Request, Update},
        Job, Output,
    },
};
use std::{sync::Arc, time::Instant};
#[path = "application_projection_sync_schedule.rs"]
mod schedule;
pub(super) use schedule::Schedule;
#[path = "application_projection_sync_publish.rs"]
mod publish;

pub(super) struct Pending {
    source: Source,
    workspace: crate::workspace::IdentityStamp,
    ticket: Arc<Ticket>,
    expected: Binding,
    guard: restore_batch::guard::Document,
    current: Lease,
    pub(super) job: Job,
}
impl Application {
    pub(in crate::desktop::application) fn projection_sync_index(
        &self,
    ) -> Option<&crate::projection_delivery::binding::checkpoint::Index> {
        let record = self.restored_session.as_ref()?;
        record
            .restored_projections
            .as_ref()
            .filter(|index| index.is_for(&record.original))
            .or_else(|| {
                self.workspace_checkpoint
                    .as_ref()?
                    .projection_baseline()
                    .filter(|(_, source)| source.same_storage(&record.original))
                    .map(|(index, _)| index)
            })
    }
    fn projection_sync_idle(&self) -> bool {
        let view = self.authorization.borrow();
        !view.closed
            && view.origin.as_deref().is_some_and(|origin| {
                job::now().is_ok_and(|now| self.authorization_owner.token(origin, now).is_some())
            })
            && !view.busy
            && !view.projection.busy
            && !self.exit_requested.get()
            && !self.checkpoint_is_exiting()
            && self.save_job.is_none()
            && self.restore_job.is_none()
            && !self.remote_sessions.projection_pending()
            && !self.projection_edit.busy()
            && restore_batch::guard::is_idle(self)
    }
    unsafe fn sync_pending_current(&self, pending: &Pending) -> bool {
        self.projection_sync_idle()
            && pending.workspace.matches(&self.workspace)
            && self.receive_source_current(&pending.source, pending.ticket.unit())
            && self.projection_profile.recorded(&pending.ticket)
            && self.authorization.borrow().origin.as_deref() == Some(pending.ticket.origin())
            && pending
                .current
                .validate(pending.ticket.origin(), job::now().unwrap_or(u64::MAX))
                .is_ok()
            && self
                .projection_sync_index()
                .and_then(|index| index.for_ticket(&pending.ticket))
                == Some(&pending.expected)
            && self
                .documents
                .iter()
                .find(|doc| doc.state.unit_id.as_str() == pending.ticket.unit())
                .is_some_and(|doc| pending.guard.matches(doc))
    }
    pub(in crate::desktop::application) unsafe fn process_projection_sync(
        &mut self,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
        now: Instant,
    ) {
        self.projection_receive.1.wake = None;
        let phase = std::mem::take(&mut self.projection_receive.0);
        if let Phase::Syncing(mut pending) = phase {
            let valid = self.sync_pending_current(&pending);
            if !valid {
                pending.job.request_stop();
            }
            let Some(result) = pending.job.try_join() else {
                self.projection_receive.0 = Phase::Syncing(pending);
                return;
            };
            let success = if valid {
                match result {
                    Ok(Output::Synchronized(outcome)) => match *outcome {
                        Outcome::Unchanged => true,
                        Outcome::Updated(update) => {
                            self.publish_projection_sync(&pending, *update).is_ok()
                        }
                    },
                    _ => false,
                }
            } else {
                false
            };
            self.projection_receive
                .1
                .settle(&pending.ticket, now, success);
            if !success {
                self.authorization.borrow_mut().projection.status =
                    "投送同步暂缓，保留当前内容".into();
            }
            let unit = success
                .then(|| crate::workspace_id::UnitId::parse(pending.ticket.unit()))
                .flatten();
            drop(pending); // 回收后才释放同 intent 许可，不产生 cleanup。
            if let Some(unit) = unit {
                self.automatic_projection_edit(unit, config);
            }
        } else {
            self.projection_receive.0 = phase;
        }
        if self.projection_receive.busy() || !self.projection_sync_idle() {
            return;
        }
        let origin = self.authorization.borrow().origin.clone();
        let candidates =
            self.projection_profile
                .entries()
                .unwrap_or(&[])
                .iter()
                .filter_map(|ticket| {
                    if ticket.cleanup_pending()
                        || Some(ticket.origin()) != origin.as_deref()
                        || !self
                            .workspace
                            .units()
                            .iter()
                            .any(|unit| unit.as_str() == ticket.unit())
                    {
                        return None;
                    }
                    let binding = self.projection_sync_index()?.for_ticket(ticket)?.clone();
                    // 首次自动准入只信真实 canonical 包含；后续连续任务可沿本 owner 的同意图推进。
                    if self.projection_checkpoint_inclusion(ticket) != Some(&binding)
                        && !self.projection_receive.1.entries.iter().any(|entry| {
                            entry.ticket.same_record(ticket) && entry.binding == binding
                        })
                    {
                        return None;
                    }
                    Some((ticket.clone(), binding))
                })
                .collect();
        self.projection_receive.1.reconcile(candidates, now);
        if let Some(index) = self.projection_receive.1.due(now) {
            let entry = &self.projection_receive.1.entries[index];
            let ticket = entry.ticket.clone();
            let expected = entry.binding.clone();
            let result = self.start_projection_sync(ticket.clone(), expected, config);
            if result.is_err() {
                self.projection_receive.1.settle(&ticket, now, false);
            }
        }
        if !self.projection_receive.busy() {
            self.projection_receive.1.wake = self
                .projection_receive
                .1
                .entries
                .iter()
                .map(|entry| entry.due)
                .min();
        }
    }
    unsafe fn start_projection_sync(
        &mut self,
        ticket: Arc<Ticket>,
        expected: Binding,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) -> Result<()> {
        let doc = self
            .documents
            .iter()
            .find(|doc| doc.state.unit_id.as_str() == ticket.unit())
            .ok_or_else(stale)?;
        let guard = restore_batch::guard::Document::capture(doc)?;
        let canvas = doc.state.canvas.borrow();
        // 不清本地破坏性编辑/历史；未实现的重基类型必须保最后图像并退避。
        let replaceable = canvas.editor.source_replaceable()
            && canvas.minified.is_none()
            && canvas
                .minified_source
                .as_ref()
                .is_none_or(|source| source.matches(&canvas.editor));
        let original = workspace_checkpoint::projection_proof::source(&canvas).cloned();
        let layers = canvas.editor.layers().ok_or_else(stale)?;
        let frame = canvas
            .editor
            .full_logical_frame()
            .unwrap_or(crate::crop_geometry::Rect {
                x: 0.0,
                y: 0.0,
                width: layers.size.0 as f64,
                height: layers.size.1 as f64,
            });
        let size = layers.size;
        let editing = doc
            .state
            .projection_edit
            .borrow()
            .as_ref()
            .map(|owner| owner.capture(&canvas.editor).document().clone());
        drop(canvas);
        let source = self
            .restored_session
            .as_ref()
            .ok_or_else(stale)?
            .original
            .clone();
        let origin = ticket.origin();
        let config = config(origin).map_err(|_| stale())?;
        let current = self.receive_lease(origin)?;
        let lease = self.receive_lease(origin)?;
        if current.device_id() != ticket.device() || !current.same_grant(&lease) {
            return Err(stale());
        }
        lease.notify_revocation(self.execution_wake.callback());
        let job = Job::start_sync(
            config,
            lease,
            Request {
                ticket: ticket.retry(),
                expected: expected.clone(),
                source: source.clone(),
                size,
                frame,
                original,
                replaceable,
                editing,
            },
        )
        .map_err(|_| stale())?;
        self.projection_receive.0 = Phase::Syncing(Box::new(Pending {
            source,
            workspace: self.workspace.identity_stamp(),
            ticket,
            expected,
            guard,
            current,
            job,
        }));
        Ok(())
    }
}
