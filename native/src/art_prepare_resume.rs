//! 当前 v3 包的在线准备与离线保存；身份缺失或请求失败直接结束，不退旧协议。
use super::*;
use crate::{
    art_execution_grant::resume::Ticket, art_offline_store::Slot, art_source_cache::Store,
};
impl Prepared {
    pub(crate) fn fetch_resumable(
        client: &Client,
        lease: Arc<Lease>,
        request: Request,
        pin: Pin,
        cache: &Store,
    ) -> Result<Self, Error> {
        let loaded = crate::device_identity_store::Store::new(cache.profile())
            .and_then(|s| s.load_existing())
            .map_err(|_| Error::Input)?;
        let identity = loaded.identity();
        if identity.device_id() != Some(lease.device_id())
            || request.binding.origin != client.origin()
            || request.binding.device_id != lease.device_id()
        {
            return Err(Error::Input);
        }
        grant::validate_expected(&request.expected()).map_err(Error::Grant)?;
        let time = now().map_err(Error::Http)?;
        lease
            .validate(client.origin(), time)
            .map_err(|_| Error::Lease)?;
        let device_key = identity.public_key().map_err(|_| Error::Input)?;
        let slot = Slot::open(
            cache.profile(),
            client.origin(),
            &request.binding.art_id,
            &pin,
            &device_key,
        )
        .map_err(|_| Error::Input)?;
        let stopped = || lease.execution_stopped();
        // 先减权，失败不得继续联网后保留一份可被未来误用的旧离线凭证。
        slot.invalidate(time, &stopped).map_err(|_| Error::Input)?;
        drop(slot);
        let source_key = cache::key(&request, &pin);
        let cached = cache.load(&source_key).ok().flatten();
        let body = serde_json::json!({"binding":request.binding,
            "permissions":request.permissions,"services":request.services,"knownSourceDigest":cached.as_ref().map(|c|c.digest.clone()).unwrap_or_else(||"0".repeat(64))});
        let bytes = package::request::fetch(client, &lease, body)?;
        let response: package::Response =
            serde_json::from_slice(&bytes).map_err(|_| Error::Wire)?;
        if response.protocol_version != grant::package::visibility::PREPARE {
            return Err(Error::Wire);
        }
        let source = match response.source {
            serde_json::Value::Null => cached.ok_or(Error::Wire)?.source,
            serde_json::Value::String(s) if s.len() <= 4 * MAX_SOURCE.div_ceil(3) => {
                STANDARD.decode(s).map_err(|_| Error::Wire)?
            }
            _ => return Err(Error::Wire),
        };
        let envelope = serde_json::to_vec(&response.grant).map_err(|_| Error::Wire)?;
        let mut prepared = Self::accept_parts(envelope, source, lease, request, pin)?;
        prepared.client = Some(client.clone());
        let package_offline =
            package::accept(&mut prepared, &response.package, cache, &device_key)?;
        let stopped = || {
            prepared.lease.execution_stopped()
                || prepared
                    .lease
                    .validate(client.origin(), now().unwrap_or(u64::MAX))
                    .is_err()
        };
        if stopped() {
            return Err(Error::Lease);
        }
        let complete = cache.put(&source_key, &prepared.source, &stopped).is_ok()
            && prepared.assets.iter().all(|asset| {
                crate::art_package_cache::put(
                    cache,
                    &source_key,
                    &asset.resource,
                    &asset.bytes,
                    &stopped,
                )
                .is_ok()
            });
        if package_offline {
            let resume_envelope = response.package;
            let bytes = serde_json::to_vec(&resume_envelope).map_err(|_| Error::Wire)?;
            let ticket = Ticket::verify(&bytes, prepared.pin.issuer()).map_err(Error::Grant)?;
            let original: wire::Grant = serde_json::from_slice(
                &STANDARD
                    .decode(&response.grant.payload)
                    .map_err(|_| Error::Wire)?,
            )
            .map_err(|_| Error::Wire)?;
            if ticket.device_public_key != device_key || ticket.grant != original {
                return Err(Error::Input);
            }
            ticket
                .permit(
                    prepared.request.expected(),
                    &prepared.source,
                    now().map_err(Error::Http)?,
                )
                .map_err(Error::Grant)?;
            // 缓存故障不影响已获得的在线许可；源码先提交，离线凭证随后原子发布。
            if complete {
                if let Ok(slot) = Slot::open(
                    cache.profile(),
                    client.origin(),
                    &prepared.request.binding.art_id,
                    &prepared.pin,
                    &device_key,
                ) {
                    let _ = slot.save(resume_envelope, now().map_err(Error::Http)?, &stopped);
                }
            }
        }
        if stopped() {
            return Err(Error::Lease);
        }
        Ok(prepared)
    }
    pub(crate) fn resumed(
        envelope: Vec<u8>,
        source: Vec<u8>,
        lease: Arc<Lease>,
        request: Request,
        pin: Pin,
        assets: Vec<crate::art_package_content::Asset>,
    ) -> Result<Self, Error> {
        let time = now().map_err(Error::Http)?;
        let ticket = Ticket::verify(&envelope, pin.issuer()).map_err(Error::Grant)?;
        let permit = ticket
            .permit(request.expected(), &source, time)
            .map_err(Error::Grant)?;
        lease
            .accept_local(&ticket, time)
            .map_err(|_| Error::Lease)?;
        Ok(Self {
            request,
            pin,
            envelope,
            source,
            lease,
            permit: Arc::new(Mutex::new(permit)),
            client: None,
            resumed: true,
            assets,
            ports: ticket.ports,
            visibility: ticket.visibility,
        })
    }
    pub(crate) fn source_key(request: &Request, pin: &Pin) -> [u8; 32] {
        cache::key(request, pin)
    }
}
