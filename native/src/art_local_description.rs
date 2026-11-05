//! 描述和 prepare 在同一后台链串行执行；登记先于网络，替换/撤信任可中断描述阶段。
use super::*;
use crate::live_relay_http::art_describe::Descriptor;

/// 只含宿主上下文；未知的包版本、digest 和权限不能用占位元数据伪造。
pub struct Request {
    pub origin: String,
    pub workspace: String,
    pub instance: String,
    pub art: String,
}
pub struct PendingDescription {
    unit: UnitId,
    token: Token,
    request: Request,
    lease: Arc<Lease>,
    authorization: Authorization,
}
impl Registry {
    pub fn begin_description(
        &mut self,
        unit: UnitId,
        request: Request,
        lease: Arc<Lease>,
        authority: &mut Authority,
    ) -> Result<PendingDescription, Error> {
        if request.workspace != self.workspace {
            return Err(Error::Workspace);
        }
        // 这里只作宿主输入有界检查；网络描述和完整 grant 仍分别经过严格合同校验。
        if request.art.is_empty()
            || request.art.len() > 257
            || request.instance.is_empty()
            || request.instance.len() > 128
            || request.workspace.is_empty()
            || request.workspace.len() > 128
        {
            return Err(Error::Identity);
        }
        let generation = self
            .requests
            .next_generation(&unit)
            .map_err(|_| Error::Capacity)?;
        if generation > 9_007_199_254_740_991 {
            return Err(Error::Capacity);
        }
        let (token, authorization) =
            self.register(&unit, &request.origin, lease.clone(), authority)?;
        Ok(PendingDescription {
            unit,
            token,
            request,
            lease,
            authorization,
        })
    }
}
impl PendingDescription {
    pub(crate) fn fetch_offline(self, profile: &std::path::Path) -> Completion {
        let result = self
            .authorization
            .resume(profile, &self.request, &self.token)
            .map_err(Error::Download);
        Completion {
            unit: self.unit,
            token: self.token,
            result,
        }
    }
    pub(crate) fn identity(&self) -> (UnitId, Token) {
        (self.unit.clone(), self.token.clone())
    }
    pub fn fetch(self, client: &Client) -> Completion {
        self.fetch_inner(client, None)
    }
    pub(crate) fn fetch_cached(
        self,
        client: &Client,
        cache: &crate::art_source_cache::Store,
    ) -> Completion {
        self.fetch_inner(client, Some(cache))
    }
    fn fetch_inner(
        self,
        client: &Client,
        cache: Option<&crate::art_source_cache::Store>,
    ) -> Completion {
        let result = (|| {
            if client.origin() != self.request.origin {
                return Err(Error::Identity);
            }
            if let Some(cache) = cache {
                self.authorization
                    .invalidate_offline(cache.profile(), &self.request.art)
                    .map_err(Error::Download)?;
            }
            let descriptor = Descriptor::fetch(client, self.lease, &self.request.art)
                .map_err(Error::Description)?;
            let (mut request, _) = descriptor
                .into_request(
                    &self.request.workspace,
                    &self.request.instance,
                    self.token.generation,
                )
                .map_err(Error::Description)?;
            // token 在描述之前由原 Registry 分配；不采纳任何响应提供的请求身份。
            request.binding.request_id = self.token.id.to_string();
            match cache {
                Some(cache) => self.authorization.prepare_cached(client, request, cache),
                None => self.authorization.prepare(client, request),
            }
            .map_err(Error::Download)
        })();
        Completion {
            unit: self.unit,
            token: self.token,
            result,
        }
    }
}
#[cfg(test)]
#[path = "art_local_description_tests.rs"]
mod tests;
