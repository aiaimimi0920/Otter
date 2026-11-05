//! 主干临时缓存合同；0 禁用对应软策略，资源/扫描硬边界独立保留。
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Policy {
    pub temp_cache_max_bytes: u64,
    pub temp_cache_retention_days: u32,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            temp_cache_max_bytes: 256 * 1024 * 1024,
            temp_cache_retention_days: 7,
        }
    }
}
impl Policy {
    pub fn validate(self) -> Result<Self, &'static str> {
        if self.temp_cache_max_bytes != 0
            && !(32 * 1024 * 1024..=16 * 1024 * 1024 * 1024).contains(&self.temp_cache_max_bytes)
        {
            return Err("Temporary cache maximum must be zero or between 32 MiB and 16 GiB");
        }
        if self.temp_cache_retention_days > 3650 {
            return Err("Temporary cache retention cannot exceed 3650 days");
        }
        Ok(self)
    }
}
#[cfg(test)]
#[path = "cache_policy_tests.rs"]
mod tests;
