//! 显式触发配置意图；不执行条件，不包含凭据，Surface身份固定为本次真实观看宿主。
use crate::live_relay_trigger_registration::{Binding, Condition, Error, Registration, Target};
use serde::{Deserialize, Serialize};
pub(crate) const MAX_TEXT: usize = 16 * 1024;
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Command {
    pub binding_id: String,
    pub enabled: bool,
    pub condition: Condition,
    pub target: Target,
}
#[cfg(test)]
#[path = "live_relay_trigger_command_tests.rs"]
mod tests;
impl Command {
    pub fn parse(text: &str) -> Result<Self, Error> {
        if text.len() > MAX_TEXT {
            return Err(Error::Limit);
        }
        let command: Self = serde_json::from_str(text).map_err(|_| Error::Json)?;
        command.validate()?;
        Ok(command)
    }
    pub fn validate(&self) -> Result<(), Error> {
        self.registration("viewer:validation").validate()
    }
    fn registration(&self, device: &str) -> Registration {
        Registration {
            binding: Binding {
                binding_id: self.binding_id.clone(),
                observation_id: self.condition.observation_id.clone(),
                condition_revision: self.condition.revision,
                authorized_by: device.into(),
                enabled: self.enabled,
            },
            condition: self.condition.clone(),
            target: self.target.clone(),
            armed: true,
            last_match: false,
            pending_offline_match: false,
        }
    }
    pub fn matches(&self, actual: &Registration, device: &str) -> bool {
        let expected = self.registration(device);
        actual.binding == expected.binding
            && actual.condition == expected.condition
            && actual.target == expected.target
    }
    /// 更新与禁用均创建新revision；不能借禁用承诺取消已reserved的Surface动作。
    pub fn update(registration: &Registration, disable: bool) -> Option<Self> {
        let mut condition = registration.condition.clone();
        condition.revision = condition.revision.checked_add(1)?;
        Some(Self {
            binding_id: registration.binding.binding_id.clone(),
            enabled: !disable && registration.binding.enabled,
            condition,
            target: registration.target.clone(),
        })
    }
}
