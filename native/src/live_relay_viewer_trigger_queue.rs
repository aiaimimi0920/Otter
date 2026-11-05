//! 原控制线程消费的单槽意图；stop封住迟到回执，不新增线程、等待句柄或无界消息。
use crate::live_relay_trigger_command::Command;
use std::sync::Mutex;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Phase {
    #[default]
    Idle,
    Pending,
    Confirmed,
    Rejected,
    Unknown,
}
#[derive(Default)]
struct State {
    binding: Option<(String, String)>,
    command: Option<Command>,
    phase: Phase,
    stopped: bool,
    paused: bool,
}
#[derive(Default)]
pub(crate) struct Queue(Mutex<State>);
impl Queue {
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub fn configure(&self, binding: Option<(String, String)>) {
        let mut s = self.state();
        if !s.stopped {
            s.binding = binding;
        }
    }
    pub fn binding(&self) -> Option<(String, String)> {
        let s = self.state();
        (!s.stopped && !s.paused)
            .then(|| s.binding.clone())
            .flatten()
    }
    pub fn available(&self) -> bool {
        let s = self.state();
        !s.stopped && !s.paused && s.binding.is_some()
    }
    pub fn phase(&self) -> Phase {
        self.state().phase
    }
    pub fn submit(&self, command: Command) -> bool {
        if command.validate().is_err() {
            return false;
        }
        let mut s = self.state();
        if s.stopped
            || s.paused
            || matches!(s.phase, Phase::Pending | Phase::Unknown)
            || s.binding.as_ref().is_none_or(|(surface, attachment)| {
                command.target.surface_instance_id != *surface
                    || command.target.surface_attachment_id != *attachment
            })
        {
            return false;
        }
        s.command = Some(command);
        s.phase = Phase::Pending;
        true
    }
    pub fn take(&self) -> Option<Command> {
        let mut s = self.state();
        if s.paused {
            None
        } else {
            s.command.take()
        }
    }
    /// 控制读取失联时取消尚未发送的配置意图；已发送的未知结果仍保Unknown终态。
    pub fn pause(&self, paused: bool) {
        let mut s = self.state();
        s.paused = paused;
        if paused && s.command.take().is_some() && s.phase == Phase::Pending {
            s.phase = Phase::Rejected;
        }
    }
    pub fn complete(&self, phase: Phase) -> bool {
        let mut s = self.state();
        if s.stopped || s.phase != Phase::Pending {
            return false;
        }
        s.phase = phase;
        true
    }
    pub fn reject(&self) {
        let mut s = self.state();
        if !s.stopped && !matches!(s.phase, Phase::Pending | Phase::Unknown) {
            s.phase = Phase::Rejected;
        }
    }
    pub fn stop(&self) {
        let mut s = self.state();
        s.stopped = true;
        s.binding = None;
        s.command = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paused_queue_rejects_new_work_and_discards_only_unsent_intents() {
        let queue = Queue::default();
        queue.configure(Some(("s".into(), "a".into())));
        let command=Command::parse(r#"{"bindingId":"b","enabled":true,"target":{"surfaceInstanceId":"s","surfaceAttachmentId":"a","surfaceNodeId":"n","surfaceEvent":"click","surfaceAction":"run"},"condition":{"conditionId":"c","revision":1,"observationId":"o","operator":"equals","operand":true,"stableForMs":0,"risingEdge":false,"rearm":false,"minimumConfidence":"exact"}}"#).unwrap();
        assert!(queue.submit(command.clone()));
        queue.pause(true);
        assert_eq!(queue.phase(), Phase::Rejected);
        assert!(!queue.available());
        assert!(!queue.submit(command.clone()));
        assert!(queue.take().is_none());
        queue.pause(false);
        assert!(queue.available());
        assert!(queue.submit(command));
        queue.take().unwrap();
        queue.complete(Phase::Unknown);
        queue.pause(true);
        queue.pause(false);
        assert_eq!(queue.phase(), Phase::Unknown);
        queue.stop();
        queue.pause(false);
        assert!(!queue.available());
    }
}
