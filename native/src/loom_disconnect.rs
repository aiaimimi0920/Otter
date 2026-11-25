//! 主动断开使用一次性的原会话凭据；任务有界且退出时回收，不轮换设备身份。
use super::{Error, Task};
use crate::desktop::loom_execution_signal::Notify;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ring::rand::{SecureRandom, SystemRandom};
use std::time::{Duration, Instant};

pub(super) fn start(
    config: crate::device_authorization_config::Config,
    token: zeroize::Zeroizing<String>,
    wake: Notify,
) -> Result<Task<()>, Error> {
    Task::spawn("otter-disconnect", wake, move |stop| {
        let client = config
            .prepare_client(stop)
            .map_err(Error::DisconnectConfig)?;
        // Device authentication requires a fresh 32-character base64url nonce.
        let mut nonce = [0; 24];
        SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| Error::Worker)?;
        let nonce = URL_SAFE_NO_PAD.encode(nonce);
        let (status, _) = client
            .exchange(
                crate::device_session_client::Request {
                    method: "POST",
                    path: "/v1/otter/connections/disconnect",
                    body: b"{}",
                    device: Some((&token, &nonce)),
                    limit: 4096,
                },
                stop.flag(),
                &mut |h| stop.install(h),
                Instant::now() + Duration::from_secs(5),
            )
            .map_err(Error::DisconnectExchange)?;
        if status == 200 || status == 401 {
            Ok(())
        } else {
            Err(Error::Response)
        }
    })
    .map_err(|_| Error::Worker)
}
