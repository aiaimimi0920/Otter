//! 已验签 Prepared 的串行服务交换；仅后台调用，不自动重试，不建立新凭据或延长许可。
//! Call 借用原 Prepared/Permit/Lease，网络等待不进入 JS child 的执行预算。
use super::{now, wire as grant_wire, Client, Prepared};
use crate::art_execution_grant;
use base64::Engine as _;
use ring::rand::SecureRandom as _;
use serde::Serialize;
use serde_json::Value;
use std::time::{Duration, Instant};
#[path = "art_service_dispatcher.rs"]
pub(crate) mod dispatcher;
#[path = "art_service_image_output.rs"]
mod image_output;
#[path = "art_service_images.rs"]
mod images;
pub(crate) const MAX_IMAGE_REQUEST: usize = images::MAX_REQUEST;
#[path = "art_service_confirmation.rs"]
pub(crate) mod confirmation;
#[path = "art_service_confirmation_wire.rs"]
mod confirmation_wire;
#[path = "art_service_progress.rs"]
pub(crate) mod progress;
#[path = "art_service_wire.rs"]
mod wire;
pub use wire::{Owner, Reply, Status};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Random,
    Input,
    Limit,
    Wire,
    Identity,
    Sequence,
    Stopped,
    Time,
    Lease,
    Grant(art_execution_grant::Error),
    Http(crate::live_relay_http::Error),
}

pub struct Call<'a> {
    authority: Authority<'a>,
    client: &'a Client,
    owner: Owner,
    input: Value,
    images: Option<images::Uploads>,
    image_output: bool,
    output_image: Option<crate::surface_javascript::ImageResource>,
    timeout_ms: u64,
    deadline: Instant,
    prior: Option<Reply>,
    submitted: bool,
    stopped: bool,
    confirmable: bool,
    challenge: Option<confirmation_wire::Challenge>,
    decision: Option<confirmation_wire::Decision>,
}
// 只借原授权，不持 MutexGuard 跨网络；停止回调可安全重入同一个 Permit。
struct Authority<'a> {
    permit: &'a std::sync::Mutex<art_execution_grant::Permit>,
    lease: &'a crate::device_authorization_lease::Lease,
    binding: &'a grant_wire::Binding,
    envelope: &'a [u8],
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Request<'a> {
    protocol_version: &'static str,
    owner: &'a Owner,
    grant: grant_wire::Envelope,
    #[serde(skip_serializing_if = "Option::is_none")]
    input: Option<&'a Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout_ms: Option<u64>,
}
impl<'a> Call<'a> {
    /// SDK 只提供服务名和 JSON；callId、完整绑定与许可只能来自宿主。
    pub fn new(
        prepared: &'a mut Prepared,
        client: &'a Client,
        service: &str,
        input: Value,
        timeout_ms: u64,
    ) -> Result<Self, Error> {
        Self::begin(
            Authority {
                permit: &prepared.permit,
                lease: &prepared.lease,
                binding: &prepared.request.binding,
                envelope: &prepared.envelope,
            },
            client,
            service,
            input,
            timeout_ms,
        )
    }
    fn begin(
        authority: Authority<'a>,
        client: &'a Client,
        service: &str,
        input: Value,
        timeout_ms: u64,
    ) -> Result<Self, Error> {
        if timeout_ms == 0 || timeout_ms > 60_000 || client.origin() != authority.binding.origin {
            return Err(Error::Input);
        }
        if serde_json::to_vec(&input).map_err(|_| Error::Wire)?.len() > wire::MAX_INPUT {
            return Err(Error::Limit);
        }
        let time = now().map_err(Error::Http)?;
        authority
            .lease
            .validate(client.origin(), time)
            .map_err(|_| Error::Lease)?;
        let mut permit = authority.permit.lock().map_err(|_| Error::Stopped)?;
        let grant = permit
            .authorize(authority.binding, time, true, authority.lease.is_revoked())
            .map_err(Error::Grant)?;
        if !grant
            .permissions
            .contains(&grant_wire::Permission::LoomService)
            || !grant.services.iter().any(|value| value == service)
        {
            return Err(Error::Input);
        }
        let mut nonce = [0u8; 24];
        ring::rand::SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| Error::Random)?;
        let owner = Owner {
            binding: grant.binding.clone(),
            grant_id: grant.grant_id.clone(),
            call_id: format!(
                "call:{}",
                base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(nonce)
            ),
            service: service.into(),
        };
        let (wall, monotonic) = permit.deadline(true);
        let wall = wall.min(authority.lease.expires_at_ms());
        drop(permit);
        let timeout_ms = timeout_ms.min(wall.saturating_sub(time));
        let deadline = (Instant::now() + Duration::from_millis(timeout_ms)).min(monotonic);
        if timeout_ms == 0 || Instant::now() >= deadline {
            return Err(Error::Time);
        }
        Ok(Self {
            authority,
            client,
            owner,
            input,
            images: None,
            image_output: false,
            output_image: None,
            timeout_ms,
            deadline,
            prior: None,
            submitted: false,
            stopped: false,
            confirmable: false,
            challenge: None,
            decision: None,
        })
    }
    pub fn owner(&self) -> &Owner {
        &self.owner
    }
    fn authorize(&mut self, cancel: bool, abandoned: bool) -> Result<(), Error> {
        if self.stopped
            && !(cancel
                && (self.authority.lease.execution_stopped() || abandoned)
                && !self.authority.lease.is_revoked())
        {
            return Err(Error::Stopped);
        }
        if Instant::now() >= self.deadline {
            return Err(Error::Time);
        }
        let time = now().map_err(Error::Http)?;
        self.authority
            .lease
            .validate(self.client.origin(), time)
            .map_err(|_| Error::Lease)?;
        if !cancel && self.authority.lease.execution_stopped() {
            return Err(Error::Stopped);
        }
        let mut permit = self.authority.permit.lock().map_err(|_| Error::Stopped)?;
        if cancel {
            permit
                .authorize_cancel(&self.owner.binding, time)
                .map_err(Error::Grant)?;
        } else {
            permit
                .authorize(
                    &self.owner.binding,
                    time,
                    true,
                    self.authority.lease.is_revoked(),
                )
                .map_err(Error::Grant)?;
        }
        Ok(())
    }
    pub fn submit(&mut self) -> Result<Reply, Error> {
        if self.submitted {
            return Err(Error::Sequence);
        }
        self.submitted = true; // Even an ambiguous transport failure must not silently replay submission.
        let path = if self.confirmable {
            "/v1/art/local/service/submit-confirmable"
        } else if self.image_output {
            "/v1/art/local/service/submit-image-output"
        } else if self.images.is_some() {
            "/v1/art/local/service/submit-images"
        } else {
            "/v1/art/local/service/submit"
        };
        self.exchange(path, true)
    }
    pub fn poll(&mut self) -> Result<Reply, Error> {
        if !self.submitted {
            return Err(Error::Sequence);
        }
        self.exchange(
            if self.confirmable {
                "/v1/art/local/service/poll-confirmable"
            } else if self.image_output {
                "/v1/art/local/service/poll-image-output"
            } else {
                "/v1/art/local/service/poll"
            },
            false,
        )
    }
    pub fn cancel(&mut self) -> Result<Reply, Error> {
        if !self.submitted {
            return Err(Error::Sequence);
        }
        self.exchange("/v1/art/local/service/cancel", false)
    }
    /// 消费失败链，只尽力取消原call一次；不恢复submit/poll，也不延长调用原期限。
    fn abandon(mut self, deadline: Instant) -> Result<Reply, Error> {
        if !self.submitted {
            return Err(Error::Sequence);
        }
        self.deadline = self
            .deadline
            .min(deadline)
            .min(Instant::now() + Duration::from_secs(1));
        self.exchange_inner("/v1/art/local/service/cancel", false, true)
    }
    fn exchange(&mut self, path: &'static str, submit: bool) -> Result<Reply, Error> {
        let result = self.exchange_inner(path, submit, false);
        if result.is_err() {
            self.stopped = true;
        }
        result
    }
    fn exchange_inner(
        &mut self,
        path: &'static str,
        submit: bool,
        abandoned: bool,
    ) -> Result<Reply, Error> {
        self.authorize(path == "/v1/art/local/service/cancel", abandoned)?;
        if let Some(prior) = self.prior.as_ref().filter(|p| {
            p.status.terminal() && !(self.image_output && path == "/v1/art/local/service/cancel")
        }) {
            return Ok(prior.clone());
        }
        let request = Request {
            protocol_version: wire::PROTOCOL,
            owner: &self.owner,
            grant: serde_json::from_slice(self.authority.envelope).map_err(|_| Error::Wire)?,
            input: submit.then_some(&self.input),
            timeout_ms: submit.then_some(self.timeout_ms),
        };
        let confirmation_reply = self.confirmable && path != "/v1/art/local/service/cancel";
        let bytes = if confirmation_reply {
            self.encode_confirmation(&request, submit, path.ends_with("/confirm"))?
        } else {
            match self.images.as_ref().filter(|_| submit) {
                Some(images) if self.image_output => {
                    images.encode_for(&request, image_output::PROTOCOL)?
                }
                Some(images) => images.encode(&request)?,
                None if submit && self.image_output => image_output::encode(&request)?,
                None => serde_json::to_vec(&request).map_err(|_| Error::Wire)?,
            }
        };
        let maximum = if submit && self.images.is_some() {
            images::MAX_REQUEST
        } else {
            96 * 1024
        };
        if bytes.len() > maximum {
            return Err(Error::Limit);
        }
        let control = if path == "/v1/art/local/service/cancel" {
            self.authority.lease.control(self.client)
        } else {
            self.authority.lease.execution_control(self.client)
        }
        .map_err(|_| Error::Lease)?;
        let image_reply = self.image_output && path != "/v1/art/local/service/cancel";
        let maximum_reply = if confirmation_reply && self.image_output {
            confirmation_wire::MAX_REPLY
        } else if confirmation_reply {
            wire::MAX_REPLY + 4096
        } else if image_reply {
            image_output::MAX_REPLY
        } else {
            wire::MAX_REPLY
        };
        let _budget = if image_reply {
            Some(crate::pixels::reserve(4 * maximum_reply).ok_or(Error::Limit)?)
        } else {
            None
        };
        let response = control
            .send_until("POST", path, &bytes, self.deadline, maximum_reply)
            .map_err(Error::Http)?;
        self.authorize(path == "/v1/art/local/service/cancel", abandoned)?;
        let reply = if confirmation_reply {
            self.parse_confirmation(&response)?
        } else if image_reply {
            let (reply, image) = image_output::parse(&response, &self.owner, self.prior.as_ref())?;
            self.output_image = image;
            reply
        } else {
            Reply::parse(&response, &self.owner, self.prior.as_ref())?
        };
        self.prior = Some(reply.clone());
        Ok(reply)
    }
}
