//! 显式包准备的受控资源下载；全部在原worker串行，逐资源前后复核原Permit/Lease和统一截止。
use super::*;
use crate::{
    art_execution_grant::package::{Resource, RESOURCE},
    art_package_content::Asset,
    art_source_cache::Store,
};
#[path = "art_prepare_package_request.rs"]
pub(super) mod request;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Response {
    pub protocol_version: String,
    pub grant: wire::Envelope,
    pub source: serde_json::Value,
    pub package: wire::Envelope,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Reply {
    protocol_version: String,
    grant_id: String,
    resource_id: String,
    digest: String,
    bytes: String,
}
fn current(prepared: &Prepared) -> Result<(), Error> {
    let time = now().map_err(Error::Http)?;
    prepared
        .lease
        .validate(&prepared.request.binding.origin, time)
        .map_err(|_| Error::Lease)?;
    prepared
        .permit
        .lock()
        .map_err(|_| Error::Lease)?
        .authorize(
            &prepared.request.binding,
            time,
            true,
            prepared.lease.execution_stopped(),
        )
        .map_err(Error::Grant)?;
    Ok(())
}
pub(super) fn accept(
    prepared: &mut Prepared,
    envelope: &wire::Envelope,
    cache: &Store,
    device_key: &str,
) -> Result<bool, Error> {
    let bytes = serde_json::to_vec(envelope).map_err(|_| Error::Wire)?;
    let proof = grant::package::visibility::Package::verify(&bytes, prepared.pin.issuer())
        .map_err(Error::Grant)?;
    let package = proof.package;
    let original: wire::Envelope =
        serde_json::from_slice(&prepared.envelope).map_err(|_| Error::Wire)?;
    let original: wire::Grant =
        serde_json::from_slice(&STANDARD.decode(original.payload).map_err(|_| Error::Wire)?)
            .map_err(|_| Error::Wire)?;
    if package.grant != original
        || (!package.device_public_key.is_empty() && package.device_public_key != device_key)
    {
        return Err(Error::Input);
    }
    let key = cache::key(&prepared.request, &prepared.pin);
    let deadline = prepared
        .deadline(true)
        .1
        .min(std::time::Instant::now() + Duration::from_secs(10));
    for resource in package.resources {
        current(prepared)?;
        let bytes = match crate::art_package_cache::load(cache, &key, &resource)
            .ok()
            .flatten()
        {
            Some(bytes) => bytes,
            None => fetch(prepared, &original.grant_id, &resource, deadline)?,
        };
        let asset = Asset::accept(resource, bytes).map_err(|_| Error::Wire)?;
        current(prepared)?;
        prepared.assets.push(asset);
    }
    crate::art_package_content::merge(
        signed::runtime::Content::empty(serde_json::json!({"state":{}})),
        &prepared.assets,
    )
    .map_err(|_| Error::Limit)?;
    current(prepared)?;
    prepared.ports = Some(proof.ports);
    prepared.visibility = Some(proof.visibility);
    Ok(!package.device_public_key.is_empty())
}
fn fetch(
    prepared: &Prepared,
    grant_id: &str,
    resource: &Resource,
    deadline: std::time::Instant,
) -> Result<Vec<u8>, Error> {
    let envelope: wire::Envelope =
        serde_json::from_slice(&prepared.envelope).map_err(|_| Error::Wire)?;
    let body = serde_json::to_vec(
        &serde_json::json!({"protocolVersion":RESOURCE,"grant":envelope,"resourceId":resource.id}),
    )
    .map_err(|_| Error::Wire)?;
    if body.len() > 32 * 1024 {
        return Err(Error::Limit);
    }
    let client = prepared.client.as_ref().ok_or(Error::Input)?;
    let response = prepared
        .lease
        .control(client)
        .map_err(|_| Error::Lease)?
        .send_until(
            "POST",
            "/v1/art/local/resource",
            &body,
            deadline,
            4 * (resource.size as usize).div_ceil(3) + 1024,
        )
        .map_err(Error::Http)?;
    let reply: Reply = serde_json::from_slice(&response).map_err(|_| Error::Wire)?;
    if reply.protocol_version != RESOURCE
        || reply.grant_id != grant_id
        || reply.resource_id != resource.id
        || reply.digest != resource.digest
        || reply.bytes.len() > 4 * (resource.size as usize).div_ceil(3)
    {
        return Err(Error::Wire);
    }
    STANDARD.decode(reply.bytes).map_err(|_| Error::Wire)
}
