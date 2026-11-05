//! 传统收件任务复用原远程集合容量/等待/退出；取消只标记，完成后才 join 并归还槽。
use super::*;
use crate::{
    projection_delivery::{InboxPolicy, Invitation},
    projection_delivery_runtime::{Error, Job, Operation, Output},
};
pub(super) struct Pending {
    job: Job,
    rejected: Option<Arc<Invitation>>,
    discarded: bool,
    targets: bool,
    sender: bool,
    sender_cleanup: bool,
}
impl Pending {
    pub(super) fn handle(&self) -> Option<BorrowedHandle<'_>> {
        self.job.wait_handle()
    }
}
pub struct Event {
    pub origin: String,
    pub rejected: Option<Arc<Invitation>>,
    pub result: Result<Output, Error>,
    pub targets: bool,
    pub sender: bool,
    pub sender_cleanup: bool,
}
impl Sessions {
    pub(crate) fn update_projection(
        &mut self,
        config: Config,
        lease: Lease,
        request: crate::projection_delivery_runtime::sender_update::Request,
    ) -> io::Result<()> {
        self.admit_projection()?;
        self.projection_job = Some(Pending {
            job: Job::start_sender_update(config, lease, request)?,
            rejected: None,
            discarded: false,
            targets: false,
            sender: true,
            sender_cleanup: false,
        });
        Ok(())
    }
    pub(crate) fn send_projection(
        &mut self,
        config: Config,
        lease: Lease,
        prepare: crate::projection_delivery_runtime::SenderPreparation,
    ) -> io::Result<()> {
        self.admit_projection()?;
        self.projection_job = Some(Pending {
            job: Job::start_sender(config, lease, prepare)?,
            rejected: None,
            discarded: false,
            targets: false,
            sender: true,
            sender_cleanup: false,
        });
        Ok(())
    }
    pub(crate) fn cancel_sender(
        &mut self,
        config: Config,
        lease: Lease,
        request: crate::projection_delivery::sender::Request,
    ) -> io::Result<()> {
        self.start_projection(
            config,
            lease,
            None,
            Operation::CancelSend(Box::new(request)),
        )
    }
    pub fn query_projection_targets(&mut self, config: Config, lease: Lease) -> io::Result<()> {
        self.start_projection(config, lease, None, Operation::Targets)
    }
    pub fn query_projection_inbox(&mut self, config: Config, lease: Lease) -> io::Result<()> {
        self.start_projection(config, lease, None, Operation::Inbox(InboxPolicy::Confirm))
    }
    pub fn reject_projection(
        &mut self,
        config: Config,
        lease: Lease,
        entry: Arc<Invitation>,
    ) -> io::Result<()> {
        let operation = Operation::Reject(Box::new((*entry).clone()));
        self.start_projection(config, lease, Some(entry), operation)
    }
    pub fn compensate_projection(
        &mut self,
        config: Config,
        lease: Lease,
        entry: Arc<crate::projection_delivery::journal::Ticket>,
    ) -> io::Result<()> {
        self.start_projection(
            config,
            lease,
            None,
            Operation::Compensate(Box::new(entry.retry())),
        )
    }
    pub fn accept_projection(
        &mut self,
        config: Config,
        lease: Lease,
        invitation: Arc<Invitation>,
        journal: crate::projection_delivery::journal::Store,
    ) -> io::Result<()> {
        self.start_projection(
            config,
            lease,
            None,
            Operation::Accept {
                invitation: Box::new((*invitation).clone()),
                journal,
            },
        )
    }
    pub fn reattach_projection(
        &mut self,
        config: Config,
        lease: Lease,
        invitation: Arc<Invitation>,
        journal: crate::projection_delivery::journal::Store,
    ) -> io::Result<()> {
        self.start_projection(
            config,
            lease,
            None,
            Operation::Reattach {
                invitation: Box::new((*invitation).clone()),
                journal,
            },
        )
    }
    fn start_projection(
        &mut self,
        config: Config,
        lease: Lease,
        rejected: Option<Arc<Invitation>>,
        operation: Operation,
    ) -> io::Result<()> {
        self.admit_projection()?;
        self.projection_job = Some(Pending {
            targets: matches!(operation, Operation::Targets),
            sender: matches!(operation, Operation::Send(_) | Operation::CancelSend(_)),
            sender_cleanup: matches!(operation, Operation::CancelSend(_)),
            job: Job::start(config, lease, operation)?,
            rejected,
            discarded: false,
        });
        Ok(())
    }
    fn admit_projection(&self) -> io::Result<()> {
        if self.closed {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        if self.projection_job.is_some()
            || self.projection_abandoned.is_some()
            || self.projection_sender_abandoned.is_some()
            || self.occupied() >= MAX_SESSIONS
        {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        Ok(())
    }
    pub fn projection_pending(&self) -> bool {
        self.projection_job.is_some()
    }
    pub(crate) fn projection_targets_pending(&self) -> bool {
        self.projection_job
            .as_ref()
            .is_some_and(|pending| pending.targets)
    }
    pub fn cancel_projection(&mut self) {
        if let Some(pending) = &mut self.projection_job {
            pending.discarded = true;
            pending.job.request_stop();
        }
    }
    pub fn poll_projection(&mut self) -> Option<Event> {
        let result = self.projection_job.as_mut()?.job.try_join()?;
        let mut pending = self.projection_job.take()?;
        self.projection_abandoned = pending.job.take_abandoned();
        self.projection_sender_abandoned = pending.job.take_sender_request();
        if self.closed || pending.discarded {
            return None;
        }
        Some(Event {
            origin: pending.job.origin().to_owned(),
            rejected: pending.rejected,
            result,
            targets: pending.targets,
            sender: pending.sender,
            sender_cleanup: pending.sender_cleanup,
        })
    }
    pub(crate) fn take_projection_abandoned(
        &mut self,
    ) -> Option<crate::projection_delivery::journal::Ticket> {
        self.projection_abandoned.take()
    }
    pub(crate) fn take_projection_sender(
        &mut self,
    ) -> Option<crate::projection_delivery::sender::Request> {
        self.projection_sender_abandoned.take()
    }
    /// 最后 Unit 已销毁，不能让尚未回收的 Accept 越过空检查点；只在终态同步回收。
    pub(crate) fn finish_projection(&mut self) {
        if let Some(mut pending) = self.projection_job.take() {
            pending.job.stop_and_join();
            self.projection_abandoned = pending.job.take_abandoned();
            self.projection_sender_abandoned = pending.job.take_sender_request();
        }
    }
}
