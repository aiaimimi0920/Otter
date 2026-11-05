//! 一次设备过滤流读取以取得确认提示；不发批准请求、不重放动作、不持续轮询。
use super::*;
use crate::{
    surface_action_ack::{Ack, Status},
    surface_click_event::ClickEvent,
    surface_confirmation::Request as Confirmation,
    surface_convergence::Tracker,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmationError {
    Deadline,
    Identity,
    State,
    Stopped,
    Missing,
    Stream(StreamError),
    Transport(Error),
    Convergence(crate::surface_convergence::Error),
    Request(crate::surface_confirmation::Error),
}
pub struct ConfirmationOutcome {
    pub ack: Ack,
    pub result: Result<Confirmation, ConfirmationError>,
}
impl Control<'_> {
    pub fn surface_confirmation(&self, event: &ClickEvent, ack: &Ack) -> ConfirmationOutcome {
        self.surface_confirmation_until(event, ack, Instant::now() + Duration::from_secs(3))
    }
    /// 批次观察只能缩短读取预算，不能为每条提示重新延长整批截止。
    pub fn surface_confirmation_until(
        &self,
        event: &ClickEvent,
        ack: &Ack,
        deadline: Instant,
    ) -> ConfirmationOutcome {
        let deadline = deadline.min(Instant::now() + Duration::from_secs(3));
        let error = if self.cancel.is_stopped() {
            Some(ConfirmationError::Stopped)
        } else if Instant::now() >= deadline {
            Some(ConfirmationError::Deadline)
        } else if event.binding().origin() != self.client.origin()
            || event.binding().device() != self.session.device_id()
            || !ack.matches_event(event)
        {
            Some(ConfirmationError::Identity)
        } else if !event.attempted()
            || !ack.accepted()
            || ack.status() != Status::AwaitingConfirmation
        {
            Some(ConfirmationError::State)
        } else {
            None
        };
        if let Some(error) = error {
            return ConfirmationOutcome {
                ack: ack.clone(),
                result: Err(error),
            };
        }
        let mut tracker = match Tracker::new(event, ack) {
            Ok(tracker) => tracker,
            Err(error) => {
                return ConfirmationOutcome {
                    ack: ack.clone(),
                    result: Err(ConfirmationError::Convergence(error)),
                }
            }
        };
        let result = (|| {
            let batch = self.surface_stream_until(0, deadline);
            if self.cancel.is_stopped() {
                return Err(ConfirmationError::Stopped);
            }
            if Instant::now() >= deadline {
                return Err(ConfirmationError::Deadline);
            }
            tracker
                .apply(&batch.map_err(ConfirmationError::Stream)?)
                .map_err(ConfirmationError::Convergence)?;
            let request = tracker
                .take_confirmation()
                .ok_or(ConfirmationError::Missing)?;
            let now = now().map_err(ConfirmationError::Transport)?;
            self.session
                .token_for(self.client.origin(), now)
                .map_err(|_| ConfirmationError::Transport(Error::Credential))?;
            if self.cancel.is_stopped() {
                return Err(ConfirmationError::Stopped);
            }
            request
                .validate(event, tracker.ack(), now)
                .map_err(ConfirmationError::Request)?;
            if Instant::now() >= deadline {
                return Err(ConfirmationError::Deadline);
            }
            Ok(request)
        })();
        ConfirmationOutcome {
            ack: tracker.ack().clone(),
            result,
        }
    }
}
