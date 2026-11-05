//! 有截止的单动作收敛读取；只 GET、不重放 POST，等待可由租约取消直接唤醒。
use super::*;
use crate::{
    surface_action_ack::{Ack, Status},
    surface_click_event::ClickEvent,
    surface_convergence::{Snapshot, Tracker},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConvergenceError {
    Identity,
    Stopped,
    Deadline,
    ConfirmationRequired,
    Stream(StreamError),
    State(crate::surface_convergence::Error),
}
pub struct ConvergenceOutcome {
    pub ack: Ack,
    pub result: Result<Snapshot, ConvergenceError>,
}
impl Control<'_> {
    /// 调用方只能缩短 25 秒总预算；成功也不是 UI 交付许可，消费时仍须复核租约。
    pub fn converge_surface(
        &self,
        event: &ClickEvent,
        ack: &Ack,
        deadline: Instant,
    ) -> ConvergenceOutcome {
        self.converge_mode(event, ack, deadline, false)
    }
    /// 决定已经 HTTP 确认后只观察原动作，不重新 POST，也不再次自动批准。
    pub fn converge_decided_surface(
        &self,
        event: &ClickEvent,
        ack: &Ack,
        decision: &crate::surface_confirmation::Decision,
        deadline: Instant,
    ) -> ConvergenceOutcome {
        if !decision.confirmed() || !decision.matches(event, ack) {
            return ConvergenceOutcome {
                ack: ack.clone(),
                result: Err(ConvergenceError::Identity),
            };
        }
        self.converge_mode(event, ack, deadline, true)
    }
    fn converge_mode(
        &self,
        event: &ClickEvent,
        ack: &Ack,
        deadline: Instant,
        decided: bool,
    ) -> ConvergenceOutcome {
        let deadline = deadline.min(Instant::now() + Duration::from_secs(25));
        if event.binding().origin() != self.client.origin()
            || event.binding().device() != self.session.device_id()
        {
            return ConvergenceOutcome {
                ack: ack.clone(),
                result: Err(ConvergenceError::Identity),
            };
        }
        let mut tracker = match Tracker::new(event, ack) {
            Ok(tracker) => tracker,
            Err(error) => {
                return ConvergenceOutcome {
                    ack: ack.clone(),
                    result: Err(ConvergenceError::State(error)),
                }
            }
        };
        let result = self.read_convergence(&mut tracker, deadline, decided);
        ConvergenceOutcome {
            ack: tracker.ack().clone(),
            result,
        }
    }
    fn convergence_gate(&self, deadline: Instant) -> Result<(), ConvergenceError> {
        if self.cancel.is_stopped() {
            return Err(ConvergenceError::Stopped);
        }
        if Instant::now() >= deadline {
            return Err(ConvergenceError::Deadline);
        }
        let now = now().map_err(|error| ConvergenceError::Stream(StreamError::Transport(error)))?;
        self.session
            .token_for(self.client.origin(), now)
            .map_err(|_| ConvergenceError::Stream(StreamError::Transport(Error::Credential)))?;
        Ok(())
    }
    fn read_convergence(
        &self,
        tracker: &mut Tracker<'_>,
        deadline: Instant,
        decided: bool,
    ) -> Result<Snapshot, ConvergenceError> {
        loop {
            self.convergence_gate(deadline)?;
            if !decided && tracker.ack().status() == Status::AwaitingConfirmation {
                return Err(ConvergenceError::ConfirmationRequired);
            }
            let was_success = tracker.ack().succeeded();
            let batch = self.surface_stream_until(tracker.after(), deadline);
            self.convergence_gate(deadline)?;
            let batch = batch.map_err(ConvergenceError::Stream)?;
            tracker.apply(&batch).map_err(ConvergenceError::State)?;
            // 状态机只保留必要状态；等待下一次读取前释放可能达到 8 MiB 的批次。
            drop(batch);
            self.convergence_gate(deadline)?;
            if let Some(snapshot) = tracker.take_snapshot() {
                return Ok(snapshot);
            }
            if !decided && tracker.ack().status() == Status::AwaitingConfirmation {
                return Err(ConvergenceError::ConfirmationRequired);
            }
            // 新观察成功后立即刷新；其余读取有界等待，避免空流或不完整刷新形成忙循环。
            if !was_success && tracker.ack().succeeded() {
                continue;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if !self.cancel.wait(Duration::from_millis(200).min(remaining)) {
                return Err(ConvergenceError::Stopped);
            }
        }
    }
}
