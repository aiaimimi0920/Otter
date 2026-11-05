//! 稳定关联更新复用原Job/Active/四槽；不登记新邀请补偿，撤权只停止本任务而不取消持久关联。
use super::*;
use crate::projection_delivery::sender::{binding::Binding, Frame, Request as SenderRequest};
pub(crate) type Preparation =
    Box<dyn FnOnce(&Lease) -> Result<Option<Frame>, Error> + Send + 'static>;
pub(crate) struct Request {
    pub request: SenderRequest,
    pub expected: Binding,
    pub source: crate::session_source::Source,
    pub prepare: Preparation,
}
impl Job {
    pub(crate) fn start_sender_update(
        config: Config,
        lease: Lease,
        request: Request,
    ) -> io::Result<Self> {
        Self::spawn_admitted(
            Connection::Configured(config),
            Arc::new(lease),
            Work::SenderUpdate(Box::new(request)),
            Some(Slot::acquire()?),
            None,
        )
    }
}
impl Request {
    pub(super) fn run(
        self,
        control: &crate::live_relay_http::Control<'_>,
        task: &Arc<Lease>,
    ) -> Result<Completed, Error> {
        // expected来自canonical，必须先验所属intent；不可借另一个目标的baseline请求网络。
        if self.expected.stopped() || !self.expected.matches_intent(self.request.intent()) {
            return Err(Error::Sender(
                crate::projection_delivery::sender::Error::Changed,
            ));
        }
        Binding::admit_update(&self.source, &self.expected).map_err(Error::Sender)?;
        let active = self
            .request
            .activate(task.clone(), false)
            .map_err(Error::Sender)?;
        let mut response = control
            .read_sender_update(&active, &self.expected)
            .map_err(Error::Http)?;
        if response.editing().is_some() {
            return Err(Error::Sender(
                crate::projection_delivery::sender::Error::Changed,
            ));
        }
        if let Some(frame) = (self.prepare)(task)? {
            if let Some(update) = crate::projection_delivery::sender::update::Update::prepare(
                frame,
                &response,
                task.preparation_cancel(),
            )
            .map_err(Error::Sender)?
            {
                response = control
                    .update_sender(&active, &self.expected, &update)
                    .map_err(Error::Http)?;
            }
        }
        active.check().map_err(Error::Sender)?;
        let candidate = if response.revision() == self.expected.revision()
            && response.digest() == self.expected.digest()
            && response.receiver_device().is_some() == self.expected.linked()
        {
            None
        } else {
            Some(
                Binding::prepare(
                    &self.source,
                    self.request.intent(),
                    &response,
                    self.expected.name(),
                    task.preparation_cancel(),
                )
                .map_err(Error::Sender)?,
            )
        };
        Ok(Completed::Updated(Box::new(sent::Connected {
            active,
            response,
            candidate,
        })))
    }
}
