//! 首次授权的有界批准等待；续签继续使用 Client::issue_once，不隐式进入此循环。
use crate::{
    device_identity::Identity,
    device_request_cancel::Cancel,
    device_session_attempt::{self, Session},
    device_session_client::{self, Client},
};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Stopped,
    Timeout,
    Request(device_session_client::Error),
}

/// 仅后台执行，最多等待九十秒；请求与等待间隔共用总期限，不启动定时线程。
pub fn wait(client: &Client, identity: &Identity, cancel: &Cancel) -> Result<Session, Error> {
    wait_until(
        client,
        identity,
        cancel,
        Instant::now() + Duration::from_secs(90),
    )
}
pub(super) fn wait_until(
    client: &Client,
    identity: &Identity,
    cancel: &Cancel,
    deadline: Instant,
) -> Result<Session, Error> {
    loop {
        if cancel.is_stopped() {
            return Err(Error::Stopped);
        }
        if Instant::now() >= deadline {
            return Err(Error::Timeout);
        }
        let result = client.issue_until(
            identity,
            cancel.flag(),
            |handle| cancel.install(handle),
            deadline,
        );
        cancel.clear();
        if cancel.is_stopped() {
            return Err(Error::Stopped);
        }
        if Instant::now() >= deadline {
            return Err(Error::Timeout);
        }
        match result {
            Ok(session) => return Ok(session),
            Err(device_session_client::Error::Protocol(
                device_session_attempt::Error::PendingApproval,
            )) => {}
            Err(error) => return Err(Error::Request(error)),
        }
        let interval =
            Duration::from_millis(250).min(deadline.saturating_duration_since(Instant::now()));
        if !cancel.wait(interval) {
            return Err(Error::Stopped);
        }
    }
}
