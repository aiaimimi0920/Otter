//! 单次点击冒泡批次：先完整预检，后台顺序提交，十秒总预算，不自动跨确认继续。
use super::*;
use crate::{
    surface_action_ack::{Ack, Status},
    surface_click_event::{ClickEvent, MAX_EVENT_BYTES},
    surface_lifecycle_owner::Owner,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchError {
    Limit,
    Identity,
    Attempted,
    Allocation,
}
#[derive(Debug, PartialEq, Eq)]
pub enum BatchStop {
    Complete,
    Confirmation,
    Action,
    Submit(SubmitError),
    Deadline,
    Panicked,
}
pub struct BatchSubmission {
    /// 与输入的连续前缀逐项对应；提交错误项没有 ACK，原事件仍保留 attempted 事实。
    pub acknowledgements: Vec<Ack>,
    pub stop: BatchStop,
}
impl Control<'_> {
    /// 不更新场景或 owner；调用方保留整批原事件，并负责后续 ACK 收敛与窗口交付。
    pub fn submit_surface_click_batch(
        &self,
        owner: &Owner,
        current_revision: u64,
        events: &mut [ClickEvent],
    ) -> Result<BatchSubmission, BatchError> {
        validate_batch(owner, current_revision, events)?;
        let mut acknowledgements = Vec::new();
        acknowledgements
            .try_reserve_exact(events.len())
            .map_err(|_| BatchError::Allocation)?;
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let mut stop = BatchStop::Complete;
        for event in events {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                stop = BatchStop::Deadline;
                break;
            }
            // 后续单项异常不能丢失此前已经收到的 ACK；当前事件 attempted 仍保留。
            let submitted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.submit_surface_click_with_timeout(owner, current_revision, event, remaining)
            }));
            let submitted = match submitted {
                Ok(value) => value,
                Err(_) => {
                    stop = BatchStop::Panicked;
                    break;
                }
            };
            match submitted {
                Ok(ack) => {
                    stop = if ack.accepted() && ack.status() == Status::AwaitingConfirmation {
                        BatchStop::Confirmation
                    } else if ack.accepted()
                        && matches!(
                            ack.status(),
                            Status::Accepted | Status::Queued | Status::Running | Status::Succeeded
                        )
                    {
                        BatchStop::Complete
                    } else {
                        BatchStop::Action
                    };
                    acknowledgements.push(ack);
                    if stop != BatchStop::Complete {
                        break;
                    }
                }
                Err(error) => {
                    stop = BatchStop::Submit(error);
                    break;
                }
            }
        }
        Ok(BatchSubmission {
            acknowledgements,
            stop,
        })
    }
}
pub(crate) fn validate_batch(
    owner: &Owner,
    revision: u64,
    events: &[ClickEvent],
) -> Result<(), BatchError> {
    validate_batch_identity(owner, revision, events)?;
    if events.iter().any(ClickEvent::attempted) {
        return Err(BatchError::Attempted);
    }
    Ok(())
}
pub(crate) fn validate_batch_identity(
    owner: &Owner,
    revision: u64,
    events: &[ClickEvent],
) -> Result<(), BatchError> {
    if events.len() > crate::surface_scene::MAX_DEPTH {
        return Err(BatchError::Limit);
    }
    let mut bytes = 0usize;
    for (index, event) in events.iter().enumerate() {
        if event.binding() != owner.binding()
            || event.revision() != revision
            || events[..index].iter().any(|prior| prior.id() == event.id())
        {
            return Err(BatchError::Identity);
        }
        bytes = bytes
            .checked_add(event.bytes().len())
            .ok_or(BatchError::Limit)?;
        if bytes > MAX_EVENT_BYTES {
            return Err(BatchError::Limit);
        }
    }
    Ok(())
}
