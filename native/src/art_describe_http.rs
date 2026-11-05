//! 认证设备获取精确已审批包描述；它不是执行许可，prepare 仍须独立验签及复核权限。
//! origin/device 只取原 Client/Lease，工作区/实例由宿主填写，不接受响应覆盖这些身份。
use super::{now, Client, Control};
use crate::{
    art_execution_grant::{self as grant, wire, Expected},
    device_authorization_lease::Lease,
    live_relay_http::art_prepare::Request,
};
use serde::Deserialize;
use std::{sync::Arc, time::Duration};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Input,
    Wire,
    Lease,
    Http(super::Error),
    Contract(grant::Error),
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Response {
    protocol_version: String,
    descriptor: Package,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Package {
    art_id: String,
    art_version: String,
    package_digest: String,
    sdk_version: String,
    permissions: Vec<wire::Permission>,
    services: Vec<String>,
}
pub struct Descriptor {
    package: Package,
    origin: String,
    lease: Arc<Lease>,
}
impl Descriptor {
    /// 后台串行调用；不得与同一 Lease 的其他网络操作并行。
    pub fn fetch(client: &Client, lease: Arc<Lease>, art: &str) -> Result<Self, Error> {
        if art.is_empty() || art.len() > 257 {
            return Err(Error::Input);
        }
        lease
            .validate(client.origin(), now().map_err(Error::Http)?)
            .map_err(|_| Error::Lease)?;
        let body =
            serde_json::to_vec(&serde_json::json!({"artId":art})).map_err(|_| Error::Input)?;
        let bytes = lease
            .control(client)
            .map_err(|_| Error::Lease)?
            .art_description(&body)?;
        let value = Self::accept(&bytes, art, client.origin(), lease)?;
        value.validate_lease()?;
        Ok(value)
    }
    fn accept(bytes: &[u8], art: &str, origin: &str, lease: Arc<Lease>) -> Result<Self, Error> {
        if bytes.len() > 16 * 1024 {
            return Err(Error::Wire);
        }
        let response: Response = serde_json::from_slice(bytes).map_err(|_| Error::Wire)?;
        if response.protocol_version != "loom.art.local.describe.v1"
            || response.descriptor.art_id != art
        {
            return Err(Error::Input);
        }
        let value = Self {
            package: response.descriptor,
            origin: origin.into(),
            lease,
        };
        // 先验证包及范围；临时身份只用于结构验证，绝不发送或建立 Registry/Permit。
        value.make_request("validation", "validation", 1)?;
        Ok(value)
    }
    fn validate_lease(&self) -> Result<(), Error> {
        self.lease
            .validate(&self.origin, now().map_err(Error::Http)?)
            .map_err(|_| Error::Lease)
    }
    fn make_request(
        &self,
        workspace: &str,
        instance: &str,
        generation: u64,
    ) -> Result<Request, Error> {
        let p = &self.package;
        let request = Request {
            binding: wire::Binding {
                origin: self.origin.clone(),
                device_id: self.lease.device_id().into(),
                workspace_id: workspace.into(),
                instance_id: instance.into(),
                generation,
                request_id: "pending".into(), // Registry 在真正 begin 时生成新 ID。
                art_id: p.art_id.clone(),
                art_version: p.art_version.clone(),
                package_digest: p.package_digest.clone(),
                sdk_version: p.sdk_version.clone(),
            },
            permissions: p.permissions.clone(),
            services: p.services.clone(),
        };
        grant::validate_expected(&Expected {
            binding: &request.binding,
            permissions: &request.permissions,
            services: &request.services,
        })
        .map_err(Error::Contract)?;
        Ok(request)
    }
    /// 消费描述并移交原租约；审批可能已变化，不能据描述直接挂载或绕过 prepare。
    pub fn into_request(
        self,
        workspace: &str,
        instance: &str,
        generation: u64,
    ) -> Result<(Request, Arc<Lease>), Error> {
        self.validate_lease()?;
        let request = self.make_request(workspace, instance, generation)?;
        Ok((request, self.lease))
    }
}
impl Control<'_> {
    fn art_description(&self, body: &[u8]) -> Result<zeroize::Zeroizing<Vec<u8>>, Error> {
        self.send_limited(
            "POST",
            "/v1/art/local/describe",
            body,
            Duration::from_secs(10),
            16 * 1024,
        )
        .map_err(Error::Http)
    }
}
#[cfg(test)]
#[path = "art_describe_tests.rs"]
mod tests;
