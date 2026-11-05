//! 单次正式交付 owner：先记录远端租约再解码；本地结果失败不会丢掉释放责任。
use super::loom_output_batch::{self, Batch};
use crate::{
    graph_requests::Request,
    loom_art_control::{self, Outcome},
    loom_output_leases::{self, Leases},
    workspace_id::UnitId,
};
use serde_json::Value;
use std::sync::atomic::AtomicBool;

pub struct Delivery {
    unit: UnitId,
    request: Request,
    result: Result<Batch, loom_output_batch::Error>,
    leases: Leases,
}
#[derive(Debug, PartialEq)]
pub enum ReleaseError {
    IdentityGeneration,
    Lease(loom_output_leases::Error),
    Transport(loom_art_control::Error),
    Rejected(Outcome),
}
#[derive(Debug, PartialEq)]
pub enum TakeError {
    PendingRelease,
    Decode(loom_output_batch::Error),
}
impl Delivery {
    /// 本地结果只移交一次；未确认释放的租约及结果仍完整归本 owner，失败不消费。
    /// 解码失败也只在释放完成后移交错误；此方法不访问网络、不复制图片。
    pub fn take_released(&mut self) -> Result<Batch, TakeError> {
        if !self.leases.is_empty() {
            return Err(TakeError::PendingRelease);
        }
        std::mem::replace(&mut self.result, Err(loom_output_batch::Error::Consumed))
            .map_err(TakeError::Decode)
    }
    /// 已认证远端取码的交付保留真实身份；仅在原结果已单次取走且租约清空后安装本地派生批次。
    /// 不创建 Delivery/lease，也不修改远端释放状态。调用方必须在后台完成全部准备后再安装。
    pub(super) fn install_derived(&mut self, batch: Batch) -> Result<(), TakeError> {
        if !self.leases.is_empty() {
            return Err(TakeError::PendingRelease);
        }
        if !matches!(self.result, Err(loom_output_batch::Error::Consumed)) {
            return Err(TakeError::Decode(loom_output_batch::Error::Consumed));
        }
        self.result = Ok(batch);
        Ok(())
    }
    /// 使用执行时固定的发现身份，而不是重新发现可能已经重启/换实例的 Loom。
    /// 仅在后台调用：TLS 连接最多三秒，控制交换最多五秒；失败后可显式重试。
    pub fn release_authenticated(
        &mut self,
        identity: &crate::loom_bridge_identity::Identity,
    ) -> Result<(), ReleaseError> {
        if self.leases.is_empty() {
            return Ok(());
        }
        let id =
            super::document_state::new_unit_id().map_err(|_| ReleaseError::IdentityGeneration)?;
        let release_id =
            UnitId::parse(&format!("release:{id}")).ok_or(ReleaseError::IdentityGeneration)?;
        self.release_with(release_id, |message, request_id| {
            let socket =
                crate::loom_bridge_client::connect(identity, std::time::Duration::from_secs(5))
                    .map_err(|_| loom_art_control::Error::Closed)?;
            loom_art_control::run(socket, message, request_id)
        })
    }
    /// 原始响应仍由调用方持有。lease 校验失败时不解码，调用者必须处理不受支持响应。
    pub fn prepare(
        unit: UnitId,
        request: Request,
        outputs: &Value,
        candidates: Option<&Value>,
        cancel: &AtomicBool,
    ) -> Result<Self, loom_output_leases::Error> {
        let leases = Leases::outputs(unit.clone(), request.clone(), outputs)?;
        let result = loom_output_batch::decode(outputs, candidates, cancel);
        Ok(Self {
            unit,
            request,
            result,
            leases,
        })
    }
    pub fn unit(&self) -> &UnitId {
        &self.unit
    }
    pub fn request(&self) -> &Request {
        &self.request
    }
    pub fn result(&self) -> Result<&Batch, &loom_output_batch::Error> {
        self.result.as_ref()
    }
    pub fn pending_release_count(&self) -> usize {
        self.leases.pending_count()
    }
    /// 后台调用，transport 应使用已认证控制通道。失败时保留本地结果及远端租约。
    /// 新 ID 由执行 owner 提供；当前方法不重试、不创建线程，也不在 Drop 中联网。
    pub fn release_with(
        &mut self,
        release_id: UnitId,
        transport: impl FnOnce(String, &UnitId) -> Result<Outcome, loom_art_control::Error>,
    ) -> Result<(), ReleaseError> {
        if self.leases.is_empty() {
            return Ok(());
        }
        let message = self
            .leases
            .begin_release(release_id.clone())
            .map_err(ReleaseError::Lease)?;
        match transport(message, &release_id) {
            Ok(outcome) => {
                let success = outcome == Outcome::Succeeded;
                self.leases
                    .complete_release(&release_id, Some(outcome.clone()));
                if success {
                    Ok(())
                } else {
                    Err(ReleaseError::Rejected(outcome))
                }
            }
            Err(error) => {
                self.leases.complete_release(&release_id, None);
                Err(ReleaseError::Transport(error))
            }
        }
    }
}

#[cfg(test)]
#[path = "loom_delivery_tests.rs"]
mod tests;
