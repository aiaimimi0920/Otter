//! 同步升级的原生产品只请求当前 v3；没有发现协商、旧路由探测或失败重放。
use super::*;
fn encode(mut request: serde_json::Value) -> Result<Vec<u8>, Error> {
    request["protocolVersion"] = grant::package::visibility::PREPARE.into();
    let body = serde_json::to_vec(&request).map_err(|_| Error::Input)?;
    if body.len() > 16 * 1024 {
        return Err(Error::Limit);
    }
    Ok(body)
}
pub(in crate::live_relay_http::art_prepare) fn fetch(
    client: &Client,
    lease: &Lease,
    request: serde_json::Value,
) -> Result<zeroize::Zeroizing<Vec<u8>>, Error> {
    let body = encode(request)?;
    lease
        .control(client)
        .map_err(|_| Error::Lease)?
        .send_limited(
            "POST",
            grant::package::visibility::ROUTE,
            &body,
            Duration::from_secs(10),
            MAX_RESPONSE + wire::MAX_ENVELOPE,
        )
        .map_err(Error::Http)
}
#[cfg(test)]
#[path = "art_prepare_package_discovery_tests.rs"]
mod tests;
