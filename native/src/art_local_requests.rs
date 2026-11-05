//! 本地 Art 请求 owner：复用 graph_requests 代际，关联原租约，使旧下载/旧交付不能复活。
//! 一个 Registry 对应一个当前工作区；后台持 Pending，不借用 UI registry，不新建线程池。
use crate::{
    art_issuer_trust::{Authority, Authorization},
    device_authorization_lease::Lease,
    device_session_client::Client,
    graph_requests::{Request as Token, Requests},
    live_relay_http::art_prepare::{Prepared, Request},
    workspace_id::UnitId,
};
use std::sync::Arc;
#[path = "art_local_description.rs"]
pub mod description;
#[derive(Debug, PartialEq)]
pub enum Error {
    Workspace,
    Lease,
    Identity,
    Capacity,
    Stale,
    Trust,
    Download(crate::art_issuer_trust::Error),
    Configuration(crate::device_authorization_config::Error),
    Worker,
    Description(crate::live_relay_http::art_describe::Error),
}
struct Entry {
    unit: UnitId,
    token: Token,
    lease: Arc<Lease>,
    delivered: bool,
    running: bool,
}
pub struct Registry {
    workspace: String,
    requests: Requests,
    active: Vec<Entry>,
}
pub struct Pending {
    unit: UnitId,
    token: Token,
    request: Request,
    authorization: Authorization,
}
pub struct Completion {
    unit: UnitId,
    token: Token,
    result: Result<Prepared, Error>,
}
impl Registry {
    pub fn new(workspace: String) -> Self {
        Self {
            workspace,
            requests: Requests::default(),
            active: Vec::new(),
        }
    }
    /// request 的 workspace/instance/package 来自宿主；requestId 由此 owner 新生成，不接受脚本复用。
    /// 每次尝试使用独立 Lease，不能把尚在使用的网络取消槽借给另一个请求。
    pub fn begin(
        &mut self,
        unit: UnitId,
        mut request: Request,
        lease: Arc<Lease>,
        authority: &mut Authority,
    ) -> Result<Pending, Error> {
        if request.binding.workspace_id != self.workspace {
            return Err(Error::Workspace);
        }
        let (token, authorization) =
            self.register(&unit, &request.binding.origin, lease, authority)?;
        request.binding.request_id = token.id.to_string();
        Ok(Pending {
            unit,
            token,
            request,
            authorization,
        })
    }
    /// 两种入口共用代际和撤销事务；描述阶段不能绕过信任登记或另建请求 owner。
    fn register(
        &mut self,
        unit: &UnitId,
        origin: &str,
        lease: Arc<Lease>,
        authority: &mut Authority,
    ) -> Result<(Token, Authorization), Error> {
        if self.active.iter().any(|e| Arc::ptr_eq(&e.lease, &lease)) {
            return Err(Error::Lease);
        }
        self.requests
            .next_generation(unit)
            .map_err(|_| Error::Capacity)?;
        let id =
            unsafe { windows::Win32::System::Com::CoCreateGuid() }.map_err(|_| Error::Identity)?;
        let id = UnitId::parse(&format!("request:{id:?}")).ok_or(Error::Identity)?;
        let authorization = authority
            .authorize(origin, lease.clone())
            .map_err(|_| Error::Trust)?;
        self.active.try_reserve(1).map_err(|_| Error::Capacity)?;
        let token = self
            .requests
            .begin(unit.clone(), id)
            .map_err(|_| Error::Capacity)?;
        // 成功建立新代际后才撤销旧请求，准备失败不破坏原实例。
        if let Some(index) = self.active.iter().position(|e| &e.unit == unit) {
            self.active.remove(index).retire();
        }
        self.active.push(Entry {
            unit: unit.clone(),
            token: token.clone(),
            lease,
            delivered: false,
            running: false,
        });
        Ok((token, authorization))
    }
    fn claim(&mut self, unit: &UnitId, token: &Token) -> Result<(), Error> {
        if !self.requests.is_latest(unit, token) {
            return Err(Error::Stale);
        }
        let entry = self
            .active
            .iter_mut()
            .find(|e| &e.unit == unit && &e.token == token)
            .ok_or(Error::Stale)?;
        if entry.delivered || entry.lease.is_revoked() {
            return Err(Error::Stale);
        }
        entry.delivered = true;
        Ok(())
    }
    /// 接纳仅一次；成功交付不等于 Runtime 接管，未接管前退休必须撤销整个 Lease。
    pub fn accept(&mut self, completion: Completion) -> Result<Prepared, Error> {
        self.claim(&completion.unit, &completion.token)?;
        match completion.result {
            Ok(prepared) => Ok(prepared),
            Err(error) => {
                self.cancel(&completion.unit);
                Err(error)
            }
        }
    }
    pub fn cancel(&mut self, unit: &UnitId) {
        self.requests.invalidate(unit);
        if let Some(index) = self.active.iter().position(|e| &e.unit == unit) {
            self.active.remove(index).retire();
        }
    }
    pub fn clear(&mut self) {
        for entry in self.active.drain(..) {
            entry.retire();
        }
        self.requests.clear();
    }
    /// 删除节点释放代际预算；原 Lease 先撤销，迟到结果仍需完整 token 匹配。
    pub fn retain_units(&mut self, keep: impl Fn(&UnitId) -> bool) {
        self.active.retain(|entry| {
            if keep(&entry.unit) {
                true
            } else {
                entry.retire();
                false
            }
        });
        self.requests.retain_units(keep);
    }
    /// 仅在 Runtime 成功接管后登记；下载交付或窗口启动失败不拥有服务清理权。
    pub(crate) fn mark_running(&mut self, unit: &UnitId, token: &Token) {
        if let Some(entry) = self
            .active
            .iter_mut()
            .find(|entry| &entry.unit == unit && &entry.token == token && entry.delivered)
        {
            entry.running = true;
        }
    }
}
impl Entry {
    fn retire(&self) {
        if self.running {
            // 原 Runtime 停止执行、串行有界 cancel，退出时最终撤销认证。
            self.lease.stop_execution();
        } else {
            self.lease.revoke();
        }
    }
}
impl Drop for Registry {
    fn drop(&mut self) {
        self.clear();
    }
}
impl Pending {
    pub(crate) fn identity(&self) -> (UnitId, Token) {
        (self.unit.clone(), self.token.clone())
    }
    pub fn binding(&self) -> &crate::art_execution_grant::wire::Binding {
        &self.request.binding
    }
    pub fn fetch(self, client: &Client) -> Completion {
        let result = self
            .authorization
            .prepare(client, self.request)
            .map_err(Error::Download);
        Completion {
            unit: self.unit,
            token: self.token,
            result,
        }
    }
}
impl Completion {
    pub(crate) fn failed(unit: UnitId, token: Token, error: Error) -> Self {
        Self {
            unit,
            token,
            result: Err(error),
        }
    }
}
#[cfg(test)]
#[path = "art_local_requests_tests.rs"]
pub(crate) mod tests;
