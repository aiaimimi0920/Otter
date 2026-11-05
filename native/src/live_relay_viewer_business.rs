//! 观看业务状态事务：先验证整批观察/审计再替换，不执行 source 输入或 Surface 动作。
use crate::{
    live_relay_observation::{self as observation, Capabilities, Observations},
    live_relay_trigger_audit::{self as audit, Audits},
    live_relay_trigger_registration::{self as registration, Registration},
    live_relay_viewer_identity::ControlEvent,
};
use std::sync::Arc;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Interaction,
    Observation(observation::Error),
    Audit(audit::Error),
    Registration(registration::Error),
}
#[derive(Clone)]
pub struct Business {
    display_identity: Arc<()>,
    interaction: crate::live_relay_http::input::Capabilities,
    capabilities: Capabilities,
    observations: Observations,
    audits: Audits,
    registrations: Arc<[Registration]>,
}
impl Business {
    /// 必须来自同一已验证 attach 响应；目录响应清空了业务数据，不能代替 attach。
    pub fn from_snapshot(bytes: &[u8]) -> Result<Self, Error> {
        Ok(Self {
            display_identity: Arc::new(()),
            interaction: crate::live_relay_http::input::Capabilities::parse(bytes)
                .map_err(|_| Error::Interaction)?,
            capabilities: Capabilities::from_snapshot(bytes).map_err(Error::Observation)?,
            observations: Observations::from_snapshot(bytes).map_err(Error::Observation)?,
            audits: Audits::from_snapshot(bytes).map_err(Error::Audit)?,
            registrations: registration::from_snapshot(bytes)
                .map_err(Error::Registration)?
                .into(),
        })
    }
    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }
    pub(crate) fn interaction(&self) -> crate::live_relay_http::input::Capabilities {
        self.interaction
    }
    pub fn observations(&self) -> &Observations {
        &self.observations
    }
    pub fn audits(&self) -> &Audits {
        &self.audits
    }
    pub fn registrations(&self) -> &[Registration] {
        &self.registrations
    }
    pub(crate) fn replace_registrations(&mut self, registrations: Vec<Registration>) {
        if self.registrations.as_ref() != registrations.as_slice() {
            self.registrations = registrations.into();
            self.display_identity = Arc::new(());
        }
    }
    /// 输入/会话心跳不使只读语义投影失效；身份仅在已提交的语义事务中换代。
    pub(crate) fn same_display(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.display_identity, &other.display_identity)
    }
    /// 调用方须先校验 envelope session/epoch/连续 sequence；成功后才提交控制游标。
    pub fn apply(&mut self, events: &[ControlEvent]) -> Result<(), Error> {
        let mut next = self.clone();
        next.observations
            .apply(
                events
                    .iter()
                    .filter(|e| e.kind() == "observation")
                    .map(|e| e.payload().as_bytes()),
            )
            .map_err(Error::Observation)?;
        next.audits
            .apply(
                events
                    .iter()
                    .filter(|e| e.kind() == "trigger_event")
                    .map(|e| e.payload().as_bytes()),
            )
            .map_err(Error::Audit)?;
        if events
            .iter()
            .any(|e| matches!(e.kind(), "observation" | "trigger_event"))
        {
            next.display_identity = Arc::new(());
        }
        // session_state 由控制 owner 处理；input_event 仅 source 执行。
        // 其余消息沿用主干 viewer 的无副作用策略，仍由外层推进可靠游标。
        *self = next;
        Ok(())
    }
}
#[cfg(test)]
#[path = "live_relay_viewer_business_tests.rs"]
mod tests;
