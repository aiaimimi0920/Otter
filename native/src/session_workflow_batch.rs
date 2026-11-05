//! 惰性控制请求批次：借用快照，最多保留一个编码消息；不连接网络或推断发送授权。
use super::{check, components, Error, Snapshot, WORKFLOW_ID};
use crate::byte_budget::{Budget, Lease};
use ring::rand::{SecureRandom, SystemRandom};
use std::sync::{atomic::AtomicBool, Arc};
#[path = "session_workflow_dispatch.rs"]
pub mod dispatch;
#[path = "session_workflow_exchange.rs"]
pub mod exchange;

pub struct Request {
    bytes: Vec<u8>,
    id: String,
    workflow: String,
    // 消息先释放，预算后释放；不提供脱离租约转移 bytes 的接口。
    _lease: Lease,
}
impl Request {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn workflow_id(&self) -> &str {
        &self.workflow
    }
}
pub struct Batch<'a> {
    snapshot: &'a Snapshot,
    components: Vec<components::Component<'a>>,
    cursor: usize,
    pending: Option<Request>,
}
impl<'a> Batch<'a> {
    pub fn prepare(
        snapshot: &'a Snapshot,
        saved: &[u8],
        cancel: &AtomicBool,
    ) -> Result<Self, Error> {
        Ok(Self {
            snapshot,
            components: components::prepare(snapshot, saved, cancel)?,
            cursor: 0,
            pending: None,
        })
    }
    pub fn remaining(&self) -> usize {
        self.components.len() + 1 - self.cursor
    }
    /// 只生成当前消息。重试借用同一字节/ID，不能把网络超时当成远端未提交。
    /// 上层必须在每次实际发送前复核保存 receipt、连接身份与授权。
    pub fn current(
        &mut self,
        budget: &Arc<Budget>,
        limit: usize,
        cancel: &AtomicBool,
    ) -> Result<Option<&Request>, Error> {
        check(cancel)?;
        if self.pending.as_ref().is_some_and(|p| p.bytes.len() > limit) {
            return Err(Error::Budget);
        }
        if self.remaining() == 0 {
            return Ok(None);
        }
        if self.pending.is_none() {
            // 编码前准入最坏容量，禁止先分配大消息再追认预算。
            let limit = limit.min(crate::session_document::MAX_SESSION_BYTES);
            let mut lease = budget.reserve(limit).ok_or(Error::Budget)?;
            let mut random = [0u8; 16];
            SystemRandom::new()
                .fill(&mut random)
                .map_err(|_| Error::RequestIdentity)?;
            let id = format!(
                "workflow-sync:{}",
                uuid::Builder::from_random_bytes(random).into_uuid()
            );
            let (bytes, workflow) = if self.cursor == 0 {
                (
                    self.snapshot.encode_request(&id, limit, cancel)?,
                    WORKFLOW_ID,
                )
            } else {
                let component = &self.components[self.cursor - 1];
                (
                    component.encode_request(&id, limit, cancel)?,
                    component.workflow_id(),
                )
            };
            if !lease.grow_to(bytes.capacity()) {
                return Err(Error::Budget);
            }
            check(cancel)?;
            self.pending = Some(Request {
                bytes,
                id,
                workflow: workflow.to_owned(),
                _lease: lease,
            });
        }
        Ok(self.pending.as_ref())
    }
    /// 仅消费调用方已验证的成功回执。失败/超时不得调用，且不更新来源元数据。
    pub fn acknowledge(&mut self, request_id: &str) -> bool {
        if self.pending.as_ref().is_none_or(|p| p.id != request_id) {
            return false;
        }
        self.pending = None;
        self.cursor += 1;
        true
    }
}

#[cfg(test)]
#[path = "session_workflow_batch_tests.rs"]
mod tests;
