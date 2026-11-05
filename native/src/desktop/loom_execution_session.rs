//! 同步后台 Art 会话组合；不创建线程/窗口。交付前处理资源释放，失败返回租约 owner。
use super::loom_delivery::Delivery;
use crate::{
    graph_requests::Request, loom_art_exchange, loom_art_response::Event, loom_bridge_client,
    loom_bridge_identity::Identity, loom_bridge_stream::Interrupt, workspace_id::UnitId,
};
use serde_json::Value;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

pub enum Update {
    Progress(f64),
    Preview { revision: u64, delivery: Delivery },
}
pub struct Final {
    pub revision: u64,
    pub delivery: Delivery,
    pub candidates: Option<Value>,
}
pub enum Failure {
    Cancelled,
    Connect(loom_bridge_client::Error),
    Interrupt,
    Exchange(loom_art_exchange::Error),
    Remote(String),
    InvalidLeases(crate::loom_output_leases::Error),
    PendingRelease(Box<Delivery>),
    Shader(super::shader_fetch_render::Error),
}
pub struct Session<'a> {
    pub identity: &'a Identity,
    pub unit: &'a UnitId,
    pub request: &'a Request,
    pub cancel: &'a AtomicBool,
    pub timeout: Duration,
}
impl Session<'_> {
    /// connected 将只读中断能力交给任务 owner；UI 不得直接调用本阻塞函数。
    /// emit 返回 false 停止会话。请求注册/旧请求取消在调用前完成，远端取消由任务 owner 负责。
    pub fn run(
        &self,
        message: String,
        connected: impl FnOnce(Interrupt),
        mut emit: impl FnMut(Update) -> bool,
    ) -> Result<Final, Failure> {
        if self.cancel.load(Ordering::Acquire) {
            return Err(Failure::Cancelled);
        }
        let socket =
            loom_bridge_client::connect(self.identity, self.timeout).map_err(Failure::Connect)?;
        let interrupt = socket
            .get_ref()
            .sock
            .interrupt_handle()
            .map_err(|_| Failure::Interrupt)?;
        connected(interrupt);
        if self.cancel.load(Ordering::Acquire) {
            return Err(Failure::Cancelled);
        }
        let mut failure = None;
        let mut preview_revision = 0;
        let result = loom_art_exchange::run_cancellable(
            socket,
            message,
            self.unit,
            self.request,
            self.timeout,
            Some(self.cancel),
            |event| match event {
                Event::Progress(value) => {
                    !self.cancel.load(Ordering::Acquire) && emit(Update::Progress(value))
                }
                Event::Preview { revision, value } => {
                    let stale = revision <= preview_revision;
                    let outputs = serde_json::json!({"preview":value});
                    match self.prepare(&outputs, None, stale) {
                        Ok(delivery) => {
                            if self.cancel.load(Ordering::Acquire) {
                                return false;
                            }
                            if stale {
                                return true;
                            }
                            preview_revision = revision;
                            emit(Update::Preview { revision, delivery })
                        }
                        Err(error) => {
                            failure = Some(error);
                            false
                        }
                    }
                }
                _ => true,
            },
        );
        if let Some(error) = failure {
            return Err(error);
        }
        let terminal = result.map_err(|error| {
            if self.cancel.load(Ordering::Acquire) || error == loom_art_exchange::Error::Cancelled {
                Failure::Cancelled
            } else {
                Failure::Exchange(error)
            }
        })?;
        match terminal {
            Event::Result {
                revision,
                outputs,
                candidates,
            } => {
                let delivery = self.prepare(&outputs, candidates.as_ref(), false)?;
                if self.cancel.load(Ordering::Acquire) {
                    return Err(Failure::Cancelled);
                }
                Ok(Final {
                    revision,
                    delivery,
                    candidates,
                })
            }
            Event::Failure(message) => Err(Failure::Remote(message)),
            _ => Err(Failure::Exchange(loom_art_exchange::Error::Closed)),
        }
    }
    fn prepare(
        &self,
        outputs: &Value,
        candidates: Option<&Value>,
        skip: bool,
    ) -> Result<Delivery, Failure> {
        // 旧预览仍需提取并释放租约，但不做昂贵像素解码。
        let cancelled = AtomicBool::new(true);
        let mut delivery = Delivery::prepare(
            self.unit.clone(),
            self.request.clone(),
            outputs,
            candidates,
            if skip { &cancelled } else { self.cancel },
        )
        .map_err(Failure::InvalidLeases)?;
        if delivery.release_authenticated(self.identity).is_err() {
            return Err(Failure::PendingRelease(Box::new(delivery)));
        }
        Ok(delivery)
    }
}
