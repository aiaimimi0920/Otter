//! 原生扩展会话、授权和命令交换；沿用本机 TLS/Hook 身份，结果副作用仍由文档 owner 审核提交。
use crate::{
    extension_when::Context,
    loom_bridge_client::BridgeSocket,
    loom_bridge_identity::Identity,
    loom_bridge_stream::Interrupt,
    loom_extension_snapshot::{self as catalog, Snapshot},
};
use ring::rand::{SecureRandom, SystemRandom};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
#[path = "loom_extension_invoke.rs"]
mod invoke;
#[path = "loom_extension_resources.rs"]
mod resources;
#[path = "loom_extension_result.rs"]
mod result;
#[cfg(test)]
#[path = "loom_extension_test_peer.rs"]
pub(crate) mod test_peer;
#[cfg(test)]
#[path = "loom_extension_session_tests.rs"]
mod tests;
#[path = "loom_extension_wire.rs"]
mod wire;
pub use resources::{Access, Attachment, Image, Prepared};
pub use result::{Effect, EffectKind, Outcome};

#[derive(Debug)]
pub enum Error {
    Cancelled,
    Timeout,
    Closed,
    Wire,
    Limit,
    Random,
    Unsupported,
    Unavailable,
    AuthorityChanged,
    Denied,
    Remote { code: String, message: String },
    Hook(crate::loom_handshake_exchange::Error),
    Snapshot(catalog::Error),
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    unit_id: String,
    revision: u64,
}
impl Target {
    pub fn new(unit: &str, revision: u64) -> Result<Self, Error> {
        if !catalog::text(unit, 384) || revision > catalog::MAX_INTEGER {
            return Err(Error::Wire);
        }
        Ok(Self {
            unit_id: unit.into(),
            revision,
        })
    }
}
pub struct Session {
    socket: Option<BridgeSocket>,
    id: String,
    snapshot: Option<Arc<Snapshot>>,
    ocr_text: bool,
}
fn request_id() -> Result<String, Error> {
    let mut bytes = [0; 16];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| Error::Random)?;
    Ok(format!(
        "extension:{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}
impl Session {
    /// 只供后台 owner 调用；本函数不读取发现文件、不创建线程，不接受 UI 任意 URL。
    pub fn open(identity: &Identity, cancel: &AtomicBool) -> Result<Self, Error> {
        if cancel.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        let (hook, mut socket) = crate::loom_handshake_exchange::open(identity, cancel).map_err(
            |error| match error {
                crate::loom_handshake_exchange::Error::Cancelled => Error::Cancelled,
                error => Error::Hook(error),
            },
        )?;
        socket.set_config(|config| {
            config.max_message_size = Some(wire::MAX_BYTES);
            config.max_frame_size = Some(wire::MAX_BYTES);
        });
        let id = request_id()?;
        let request = serde_json::json!({"method":"loom.extension.handshake", "params":{
            "requestId":id, "hookSessionId":hook.session_id(), "protocol":catalog::PROTOCOL,
            "apiVersion":catalog::API, "requiredFeatures":["contribution.snapshot", "command.invoke", "resource.authorization.v1"],
            "optionalFeatures":["shortcut.registry", "menu.registry", "notice.effects", "ocr-text.v1"] }});
        let response = wire::exchange(
            &mut socket,
            &mut None,
            &id,
            request,
            Duration::from_secs(10),
            cancel,
        )?;
        let response = wire::handshake(&response)?;
        Ok(Self {
            socket: Some(socket),
            id: response.0,
            snapshot: Some(Arc::new(response.1)),
            ocr_text: response.2,
        })
    }
    pub fn snapshot(&self) -> Option<&Snapshot> {
        self.socket.as_ref()?;
        self.snapshot.as_deref()
    }
    pub fn supports_ocr_text(&self) -> bool {
        self.ocr_text
    }
    /// 只读目录投影，不签发资源许可；会话 owner 必须在终止时撤销其 UI 引用。
    pub fn snapshot_handle(&self) -> Option<Arc<Snapshot>> {
        self.socket.as_ref()?;
        self.snapshot.clone()
    }
    /// 后台订阅阻塞等待真正的目录换代；不发轮询 RPC，取消沿现有 Interrupt 唤醒。
    pub fn wait_update(&mut self, cancel: &AtomicBool) -> Result<(), Error> {
        let result = wire::wait_update(
            self.socket.as_mut().ok_or(Error::Closed)?,
            &mut self.snapshot,
            cancel,
        );
        if result.is_err() {
            self.stop();
        }
        result
    }
    pub fn interrupt(&self) -> Result<Interrupt, Error> {
        self.socket
            .as_ref()
            .ok_or(Error::Closed)?
            .get_ref()
            .sock
            .interrupt_handle()
            .map_err(|_| Error::Closed)
    }
    /// 真实远端 checkOnly 校验，不签发资源 lease；执行和附件 CAS 仍需各自事务。
    pub fn authorize(
        &mut self,
        command: &str,
        context: &Context<'_>,
        target: &Target,
        cancel: &AtomicBool,
    ) -> Result<(), Error> {
        if cancel.load(Ordering::Acquire) {
            self.stop();
            return Err(Error::Cancelled);
        }
        let snapshot = self.snapshot().ok_or(Error::Closed)?;
        let authority = snapshot
            .command(command, context)
            .ok_or(Error::Unavailable)?;
        let generation = snapshot.generation();
        let id = request_id()?;
        let request = serde_json::json!({"method":"loom.extension.command.authorize", "params":{
            "requestId":id, "sessionId":self.id, "pluginId":authority.plugin(), "commandId":authority.id(),
            "snapshotGeneration":generation, "target":target, "checkOnly":true }});
        let result = wire::exchange(
            self.socket.as_mut().ok_or(Error::Closed)?,
            &mut self.snapshot,
            &id,
            request,
            Duration::from_secs(5),
            cancel,
        );
        // 目录换代后连成功响应也不代表旧按钮的权限仍有效，不重放这个请求。
        let result = result.and_then(|data| {
            if data["authorized"] != true {
                return Err(Error::Denied);
            }
            Ok(())
        });
        self.settle(generation, result)
    }
    fn settle<T>(&mut self, generation: u64, result: Result<T, Error>) -> Result<T, Error> {
        let result = if self
            .snapshot
            .as_ref()
            .is_some_and(|s| s.generation() != generation)
        {
            Err(Error::AuthorityChanged)
        } else {
            result
        };
        if result.is_err() {
            self.stop();
        }
        result
    }
    fn stop(&mut self) {
        // 不保留失败会话的旧 authority 或空闲连接；由同一 owner 关闭，无后台重试。
        self.snapshot = None;
        self.socket = None;
    }
}

fn remaining(
    socket: &mut BridgeSocket,
    deadline: Instant,
    cap: Duration,
    cancel: &AtomicBool,
) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        return Err(Error::Cancelled);
    }
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(Error::Timeout);
    }
    socket
        .get_mut()
        .sock
        .operation_deadline(remaining.min(cap))
        .map_err(|_| transport_error(cancel))
}

// Interrupt 可在调用前的取消检查与实际 I/O 之间生效；错误分类必须再次观察取消 owner。
fn transport_error(cancel: &AtomicBool) -> Error {
    if cancel.load(Ordering::Acquire) {
        Error::Cancelled
    } else {
        Error::Closed
    }
}
