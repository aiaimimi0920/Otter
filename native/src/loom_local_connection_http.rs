//! Local management channel for discovery manifests: fixed loopback, no proxies/redirects, bounded time/size, and no remote addresses.
use super::Error;
use crate::{device_request_cancel::Cancel, loom_media_target::Target};
use serde_json::Value;
use std::{io::Read, net::IpAddr, time::Duration};
use zeroize::Zeroizing;

pub(super) struct Local {
    origin: String,
    token: Zeroizing<String>,
    agent: ureq::Agent,
}
impl Local {
    pub(super) fn new(target: Target) -> Result<Self, Error> {
        let (origin, token) = target.local_control().map_err(|_| Error::LocalOnly)?;
        let mut url = url::Url::parse(origin).map_err(|_| Error::LocalOnly)?;
        // Pin localhost to loopback so DNS cannot take local-administrator credentials off-machine.
        if url.host_str() == Some("localhost") {
            url.set_host(Some("127.0.0.1"))
                .map_err(|_| Error::LocalOnly)?;
        }
        if !url.host_str().is_some_and(|host| {
            host.trim_matches(['[', ']'])
                .parse::<IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
        }) || token.is_empty()
            || token.len() > 8192
            || token.chars().any(char::is_control)
        {
            return Err(Error::LocalOnly);
        }
        let agent = ureq::Agent::config_builder()
            .proxy(None)
            .max_redirects(0)
            .http_status_as_error(false)
            .max_idle_connections(2)
            .max_idle_connections_per_host(2)
            .max_idle_age(Duration::from_secs(15))
            .max_response_header_size(16 * 1024)
            .timeout_global(Some(Duration::from_secs(2)))
            .build()
            .into();
        Ok(Self {
            origin: url.origin().ascii_serialization(),
            token: Zeroizing::new(token.into()),
            agent,
        })
    }
    pub(super) fn request(
        &self,
        path: &str,
        body: Option<Value>,
        cancel: &Cancel,
    ) -> Result<Value, Error> {
        if cancel.is_stopped() {
            return Err(Error::Stopped);
        }
        let url = format!("{}{path}", self.origin);
        let authorization = Zeroizing::new(format!("Bearer {}", &*self.token));
        let response = match body {
            Some(body) => self
                .agent
                .post(&url)
                .header("Authorization", &*authorization)
                .header("Content-Type", "application/json")
                .send(body.to_string()),
            None => self
                .agent
                .get(&url)
                .header("Authorization", &*authorization)
                .call(),
        };
        let mut response = response.map_err(|_| Error::LocalHttp)?;
        if !response.status().is_success() {
            return Err(Error::LocalHttp);
        }
        let mut bytes = Zeroizing::new(Vec::new());
        response
            .body_mut()
            .as_reader()
            .take(256 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::LocalHttp)?;
        if cancel.is_stopped() {
            return Err(Error::Stopped);
        }
        if bytes.len() > 256 * 1024 {
            return Err(Error::Response);
        }
        serde_json::from_slice(&bytes).map_err(|_| Error::Response)
    }
}
