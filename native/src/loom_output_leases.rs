//! 每次 formal 交付的远端租约 owner；保留执行身份，释放失败不得静默丢掉待释放列表。
use crate::{
    graph_requests::Request,
    loom_art_control::{self, Outcome},
    workspace_id::UnitId,
};
use serde_json::Value;

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidOutputs,
    InvalidHandle,
    Capacity,
    Allocation,
    Busy,
    Empty,
    Encoding,
}
pub struct Leases {
    unit: UnitId,
    execution: Request,
    handles: Vec<String>,
    attempt: Option<UnitId>,
}
impl Leases {
    /// 扫描正式输出的顶层资源，不把任意 value JSON 内的同名字段当成可释放句柄。
    pub fn outputs(unit: UnitId, execution: Request, outputs: &Value) -> Result<Self, Error> {
        let map = outputs.as_object().ok_or(Error::InvalidOutputs)?;
        if map.len() > 128 {
            return Err(Error::Capacity);
        }
        Self::collect(unit, execution, map.values())
    }
    pub fn preview(unit: UnitId, execution: Request, value: &Value) -> Result<Self, Error> {
        Self::collect(unit, execution, std::iter::once(value))
    }
    fn collect<'a>(
        unit: UnitId,
        execution: Request,
        values: impl Iterator<Item = &'a Value>,
    ) -> Result<Self, Error> {
        let mut handles = Vec::new();
        for value in values {
            if value["kind"].as_str() != Some("shared_memory") {
                continue;
            }
            let handle = value["handle"]
                .as_str()
                .filter(|name| {
                    name.starts_with("Loom_Buffer_")
                        && name.len() > 12
                        && name.len() <= 256
                        && !name.chars().any(char::is_control)
                })
                .ok_or(Error::InvalidHandle)?;
            if handles.iter().any(|previous| previous == handle) {
                continue;
            }
            if handles.len() >= 128 {
                return Err(Error::Capacity);
            }
            handles.try_reserve(1).map_err(|_| Error::Allocation)?;
            handles.push(handle.to_owned());
        }
        Ok(Self {
            unit,
            execution,
            handles,
            attempt: None,
        })
    }
    pub fn is_empty(&self) -> bool {
        self.handles.is_empty()
    }
    pub fn pending_count(&self) -> usize {
        self.handles.len()
    }
    /// 编码成功后才登记尝试。调用方为每次尝试提供全新 ID；失败回执不能覆盖后一次尝试。
    pub fn begin_release(&mut self, release_id: UnitId) -> Result<String, Error> {
        if self.attempt.is_some() {
            return Err(Error::Busy);
        }
        if self.is_empty() {
            return Err(Error::Empty);
        }
        let handles: Vec<&str> = self.handles.iter().map(String::as_str).collect();
        let message =
            loom_art_control::release_message(&self.unit, &self.execution, &release_id, &handles)
                .map_err(|_| Error::Encoding)?;
        self.attempt = Some(release_id);
        Ok(message)
    }
    /// 只有当前尝试的成功回执能清空租约。None 表示传输失败，可再次尝试；不伪造成功。
    pub fn complete_release(&mut self, release_id: &UnitId, outcome: Option<Outcome>) -> bool {
        if self.attempt.as_ref() != Some(release_id) {
            return false;
        }
        self.attempt = None;
        if outcome == Some(Outcome::Succeeded) {
            self.handles.clear();
        }
        true
    }
}
// 不在 Drop 中阻塞网络或创建后台线程。执行 owner 退出前必须显式处理 pending_count，
// 失败租约应保留在有界重试 owner 中；当前类型本身不保证远端释放已经发生。

#[cfg(test)]
#[path = "loom_output_leases_tests.rs"]
mod tests;
