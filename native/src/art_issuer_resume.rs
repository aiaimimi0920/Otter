//! 离线启动只从当前profile已有设备身份、受信pin与独立签名恢复凭证取精确包；不访问网络。
use super::*;
use crate::{
    art_execution_grant::{resume::Ticket, wire},
    art_local_requests::description,
    graph_requests::Request as Token,
};
use std::path::Path;
fn now() -> Result<u64, Error> {
    u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| Error::Trust)?
            .as_millis(),
    )
    .map_err(|_| Error::Trust)
}
impl Authorization {
    pub(crate) fn invalidate_offline(&self, profile: &Path, art: &str) -> Result<(), Error> {
        let Ok(store) = crate::device_identity_store::Store::new(profile) else {
            return Err(Error::Trust);
        };
        let loaded = match store.load_existing() {
            Ok(loaded) => loaded,
            // 旧在线调用者可能没有持久身份；锁冲突/损坏不能冒充“不存在”跳过减权。
            Err(crate::device_identity_store::Error::Missing) => return Ok(()),
            Err(_) => return Err(Error::Trust),
        };
        if loaded.identity().device_id() != Some(self.lease.device_id()) {
            return Err(Error::Trust);
        }
        let key = loaded.identity().public_key().map_err(|_| Error::Trust)?;
        let slot =
            crate::art_offline_store::Slot::open(profile, &self.origin, art, &self.pin, &key)
                .map_err(|_| Error::Trust)?;
        slot.invalidate(now()?, &|| self.lease.execution_stopped())
            .map_err(|_| Error::Trust)
    }
    pub(crate) fn resume(
        self,
        profile: &Path,
        context: &description::Request,
        token: &Token,
    ) -> Result<Prepared, Error> {
        if !self.lease.is_local() || self.origin != context.origin || self.lease.execution_stopped()
        {
            return Err(Error::Trust);
        }
        let store = crate::device_identity_store::Store::new(profile).map_err(|_| Error::Trust)?;
        let loaded = store.load_existing().map_err(|_| Error::Trust)?;
        let key = loaded.identity().public_key().map_err(|_| Error::Trust)?;
        let now = now()?;
        let slot = crate::art_offline_store::Slot::open(
            profile,
            &self.origin,
            &context.art,
            &self.pin,
            &key,
        )
        .map_err(|_| Error::Trust)?;
        let bytes = slot
            .take(now, &|| self.lease.execution_stopped())
            .map_err(|_| Error::Trust)?;
        drop(slot);
        let ticket = Ticket::verify(
            &bytes,
            crate::art_execution_grant::Issuer {
                key_id: &self.pin.key_id,
                public_key: &self.pin.public_key,
            },
        )
        .map_err(|_| Error::Trust)?;
        let old = &ticket.grant.binding;
        if ticket.grant.issued_at_ms
            <= crate::art_offline_store::Slot::cutoff(profile, &self.origin)
                .map_err(|_| Error::Trust)?
        {
            return Err(Error::Trust);
        }
        if ticket.device_public_key != key
            || Some(old.device_id.as_str()) != loaded.identity().device_id()
            || old.origin != context.origin
            || old.art_id != context.art
        {
            return Err(Error::Trust);
        }
        let request = Request {
            binding: wire::Binding {
                origin: context.origin.clone(),
                request_id: token.id.to_string(),
                device_id: old.device_id.clone(),
                workspace_id: context.workspace.clone(),
                instance_id: context.instance.clone(),
                generation: token.generation,
                art_id: old.art_id.clone(),
                art_version: old.art_version.clone(),
                package_digest: old.package_digest.clone(),
                sdk_version: old.sdk_version.clone(),
            },
            permissions: ticket.grant.permissions.clone(),
            services: ticket.grant.services.clone(),
        };
        let cache = crate::art_source_cache::Store::new(profile);
        let source = cache
            .load(&Prepared::source_key(&request, &self.pin))
            .map_err(|_| Error::Trust)?
            .ok_or(Error::Trust)?
            .source;
        let source_key = Prepared::source_key(&request, &self.pin);
        let mut assets = Vec::new();
        for resource in ticket.resources {
            if self.lease.execution_stopped() {
                return Err(Error::Trust);
            }
            let bytes = crate::art_package_cache::load(&cache, &source_key, &resource)
                .map_err(|_| Error::Trust)?
                .ok_or(Error::Trust)?;
            assets.push(
                crate::art_package_content::Asset::accept(resource, bytes)
                    .map_err(|_| Error::Trust)?,
            );
        }
        Prepared::resumed(bytes, source, self.lease, request, self.pin, assets)
            .map_err(Error::Prepare)
    }
}
