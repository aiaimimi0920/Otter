//! 原Live控制worker和UI之间的独立显式授权单槽；配置/Stop/回执共用原线程。
use super::ticket::{Authorization, Ticket};
use super::*;
use crate::live_relay_viewer_trigger_queue::Phase;
use std::sync::{Arc, Mutex};
#[derive(Clone)]
pub(crate) struct Host {
    pub binding: Binding,
    pub identity: Identity,
}
#[derive(Clone)]
pub(crate) struct Registration {
    pub scope: Scope,
    pub condition: Condition,
    gate: Arc<Authorization>,
    confirmed: bool,
}
#[derive(Clone)]
pub(crate) enum Command {
    Configure(Registration),
    Stop(Registration),
}
#[derive(Default)]
struct State {
    host: Option<Host>,
    registration: Option<Registration>,
    command: Option<Command>,
    phase: Phase,
    ticket: Option<Ticket>,
    delivered: bool,
    ack_sent: bool,
    closed: bool,
    paused: bool,
}
#[derive(Default)]
pub(crate) struct Queue(Mutex<State>);
impl Queue {
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub fn configure_host(&self, host: Host) {
        let mut s = self.state();
        if !s.closed && s.host.is_none() {
            s.host = Some(host);
        }
    }
    pub fn host(&self) -> Option<Host> {
        let s = self.state();
        (!s.closed).then(|| s.host.clone()).flatten()
    }
    pub fn phase(&self) -> Phase {
        self.state().phase
    }
    pub fn registration(&self) -> Option<(Scope, Condition, bool)> {
        self.state().registration.as_ref().map(|r| {
            (
                r.scope.clone(),
                r.condition.clone(),
                r.confirmed && r.gate.active(),
            )
        })
    }
    pub fn submit(&self, condition: Condition, authorization: String) -> bool {
        if condition.validate().is_err() || condition.revision > MAX_SEQUENCE {
            return false;
        }
        let mut s = self.state();
        let Some(host) = &s.host else { return false };
        if s.closed
            || s.paused
            || s.ticket.is_some()
            || matches!(s.phase, Phase::Pending | Phase::Unknown)
        {
            return false;
        }
        let revision = s
            .registration
            .as_ref()
            .map_or(Some(1), |r| r.condition.revision.checked_add(1));
        if revision != Some(condition.revision)
            || s.registration
                .as_ref()
                .is_some_and(|r| r.scope.authorization_id == authorization)
        {
            return false;
        }
        let scope = Scope {
            session_id: host.identity.session().into(),
            epoch: host.identity.epoch(),
            binding_id: format!("local-art:{}", host.binding.instance_id),
            authorization_id: authorization,
        };
        if !scope.validate() {
            return false;
        }
        if let Some(r) = &s.registration {
            r.gate.stop();
        }
        let r = Registration {
            scope,
            condition,
            gate: Authorization::new(),
            confirmed: false,
        };
        s.command = Some(Command::Configure(r.clone()));
        s.registration = Some(r);
        s.phase = Phase::Pending;
        true
    }
    /// 立即取消本地执行，HTTP仅负责随后减权；不等待网络才封住迟到结果。
    pub fn stop(&self) -> bool {
        let mut s = self.state();
        if s.closed {
            return false;
        }
        let Some(r) = s.registration.clone() else {
            return false;
        };
        r.gate.stop();
        s.ticket = None;
        s.delivered = false;
        s.ack_sent = false;
        s.command = Some(Command::Stop(r));
        s.phase = Phase::Pending;
        true
    }
    pub fn take_command(&self) -> Option<Command> {
        self.state().command.take()
    }
    pub fn command_pending(&self) -> bool {
        self.state().command.is_some()
    }
    pub fn confirm(&self, r: &Registration, enabled: bool) {
        let mut s = self.state();
        if s.closed
            || s.registration
                .as_ref()
                .is_none_or(|current| !Arc::ptr_eq(&current.gate, &r.gate))
        {
            return;
        }
        if enabled && !r.gate.active() {
            return;
        } // Stop在HTTP期间到达，旧成功不能恢复授权。
        if let Some(current) = &mut s.registration {
            current.confirmed = enabled;
        }
        s.phase = Phase::Confirmed;
    }
    pub fn reject(&self, unknown: bool) {
        let mut s = self.state();
        if let Some(r) = &s.registration {
            r.gate.stop();
        }
        s.ticket = None;
        s.command = None;
        s.phase = if unknown {
            Phase::Unknown
        } else {
            Phase::Rejected
        };
    }
    /// 媒体/控制失联后须新用户授权；重连不恢复旧注册。
    pub fn retire(&self) {
        let mut s = self.state();
        Self::retire_locked(&mut s);
    }
    fn retire_locked(s: &mut State) {
        if let Some(r) = &s.registration {
            r.gate.stop();
        }
        s.ticket = None;
        s.command = None;
        s.delivered = false;
        s.ack_sent = false;
        if s.registration.is_some() && s.phase != Phase::Unknown {
            s.phase = Phase::Rejected;
        }
    }
    pub fn pause(&self, paused: bool) {
        let mut s = self.state();
        s.paused = paused;
        if paused {
            Self::retire_locked(&mut s);
        }
    }
    pub fn close(&self) {
        let mut s = self.state();
        Self::retire_locked(&mut s);
        s.closed = true;
        s.host = None;
    }
    pub fn poll_registration(&self) -> Option<Registration> {
        let s = self.state();
        s.registration
            .as_ref()
            .filter(|r| {
                !s.closed
                    && !s.paused
                    && s.command.is_none()
                    && r.confirmed
                    && r.gate.active()
                    && s.ticket.is_none()
            })
            .cloned()
    }
    pub fn publish(&self, registration: &Registration, delivery: Delivery) -> bool {
        let mut s = self.state();
        let Some(r) = &s.registration else {
            return false;
        };
        if s.closed
            || s.paused
            || !r.confirmed
            || !r.gate.active()
            || !Arc::ptr_eq(&r.gate, &registration.gate)
            || s.ticket.is_some()
        {
            return false;
        }
        let Some(host) = &s.host else { return false };
        if delivery.condition_revision != r.condition.revision
            || delivery.observation.observation_id != r.condition.observation_id
            || delivery.source_device_id != host.identity.source_device()
        {
            return false;
        }
        let event = Event {
            session_id: r.scope.session_id.clone(),
            epoch: r.scope.epoch,
            delivery,
        };
        let Some(ticket) = Ticket::new(host.binding.clone(), r.gate.clone(), event) else {
            r.gate.stop();
            return false;
        };
        s.ticket = Some(ticket);
        s.delivered = false;
        s.ack_sent = false;
        true
    }
    pub fn take_delivery(&self) -> Option<Ticket> {
        let mut s = self.state();
        if s.closed || s.delivered {
            return None;
        }
        let ticket = s.ticket.as_ref()?.clone();
        if !ticket.current(ticket.binding()) {
            return None;
        }
        s.delivered = true;
        Some(ticket)
    }
    pub fn take_receipt(&self) -> Option<(Registration, Ticket, Receipt)> {
        let mut s = self.state();
        if s.closed || s.ack_sent {
            return None;
        }
        let ticket = s.ticket.as_ref()?.clone();
        if !ticket.current(ticket.binding()) {
            return None;
        }
        let receipt = ticket.receipt()?;
        let registration = s.registration.clone()?;
        s.ack_sent = true;
        Some((registration, ticket, receipt))
    }
    pub fn acknowledged(&self) {
        let mut s = self.state();
        s.ticket = None;
        s.delivered = false;
        s.ack_sent = false;
    }
    pub fn expire(&self) {
        let mut s = self.state();
        if s.ticket.as_ref().is_some_and(|t| !t.current(t.binding())) {
            Self::retire_locked(&mut s);
        }
    }
}

#[cfg(test)]
#[path = "art_live_trigger_queue_tests.rs"]
mod tests;
