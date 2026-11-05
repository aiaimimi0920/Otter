//! Loom 工作流控制请求编码；借用快照大字段，不执行发送或推断连接权限。
use super::{check, Error};
use std::{
    io::Write,
    sync::atomic::{AtomicBool, Ordering},
};

pub(super) fn validate_request_id(id: &str) -> Result<(), Error> {
    if id.is_empty() || id.len() > 256 || id.chars().any(char::is_control) {
        Err(Error::RequestIdentity)
    } else {
        Ok(())
    }
}
pub(super) fn request<T: serde::Serialize>(
    workflow: &str,
    snapshot: &T,
    request_id: &str,
    limit: usize,
    cancel: &AtomicBool,
) -> Result<Vec<u8>, Error> {
    check(cancel)?;
    validate_request_id(request_id)?;
    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Params<'a, T> {
        request_id: &'a str,
        workflow_id: &'a str,
        snapshot: &'a T,
    }
    #[derive(serde::Serialize)]
    struct Request<'a, T> {
        method: &'static str,
        params: Params<'a, T>,
    }
    encode(
        &Request {
            method: "loom.hook.workflow.sync",
            params: Params {
                request_id,
                workflow_id: workflow,
                snapshot,
            },
        },
        limit,
        cancel,
    )
}
pub(super) fn encode<T: serde::Serialize>(
    value: &T,
    limit: usize,
    cancel: &AtomicBool,
) -> Result<Vec<u8>, Error> {
    struct Bounded<'a> {
        bytes: Vec<u8>,
        limit: usize,
        cancel: &'a AtomicBool,
    }
    impl Write for Bounded<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.cancel.load(Ordering::Acquire)
                || bytes.len() > self.limit.saturating_sub(self.bytes.len())
            {
                return Err(std::io::Error::other(
                    "workflow request cancelled or over budget",
                ));
            }
            let needed = self.bytes.len() + bytes.len();
            if needed > self.bytes.capacity() {
                // 有界几何增长，避免 serde 的短片段写入每次都重新分配/复制大 payload。
                let capacity = needed
                    .max(self.bytes.capacity().saturating_mul(2).max(256))
                    .min(self.limit);
                self.bytes
                    .try_reserve_exact(capacity - self.bytes.len())
                    .map_err(std::io::Error::other)?;
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    check(cancel)?;
    let mut writer = Bounded {
        bytes: Vec::new(),
        limit: limit.min(crate::session_document::MAX_SESSION_BYTES),
        cancel,
    };
    let encoded = serde_json::to_writer(&mut writer, value);
    check(cancel)?;
    encoded.map_err(|_| Error::Budget)?;
    Ok(writer.bytes)
}

#[cfg(test)]
#[path = "session_workflow_wire_tests.rs"]
mod tests;
