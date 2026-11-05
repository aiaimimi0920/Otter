//! 有界本地 Art 下载任务：原 registry 拥有代际/租约，清理中的线程继续占槽。
//! UI 仅移交配置快照，DNS/TLS/源码验签在 worker；不自动建立信任或创建窗口。
use super::loom_execution_signal::{CompletionSignal, Notify};
use crate::{
    art_execution_grant::wire::Binding,
    art_issuer_trust::Authority,
    art_local_requests::{Completion, Error, Pending, Registry},
    device_authorization_config::Config,
    device_authorization_lease::Lease,
    graph_requests::Request as Token,
    live_relay_http::art_prepare::{Prepared, Request},
    workspace_id::UnitId,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

const MAX_JOBS: usize = 2;
struct Job {
    unit: UnitId,
    token: Token,
    lease: Arc<Lease>,
    ready: Arc<AtomicBool>,
    worker: JoinHandle<Completion>,
}
pub struct Report {
    pub unit: UnitId,
    pub result: Result<Prepared, Error>,
}
pub struct Tasks {
    registry: Registry,
    jobs: Vec<Job>,
    notify: Notify,
}
impl Tasks {
    /// notify 必须短且不阻塞，不同步等待 UI；生产调用方使用 Application Wake 的投递回调。
    pub fn new(workspace: String, notify: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self {
            registry: Registry::new(workspace),
            jobs: Vec::new(),
            notify,
        }
    }
    /// 已结束但未领取的线程同样占槽，最大只保留两份有界源码结果。
    pub fn busy(&self) -> bool {
        !self.jobs.is_empty()
    }
    pub fn has_capacity(&self) -> bool {
        self.jobs.len() < MAX_JOBS
    }
    /// 从设备描述到源码验签共用一个 worker 和原 Lease；UI 不先同步获取描述。
    pub fn start_description(
        &mut self,
        unit: UnitId,
        request: crate::art_local_requests::description::Request,
        lease: Arc<Lease>,
        authority: &mut Authority,
        config: Config,
    ) -> Result<Token, Error> {
        if config.origin() != request.origin {
            return Err(Error::Configuration(
                crate::device_authorization_config::Error::Origin,
            ));
        }
        let worker_lease = lease.clone();
        self.start_description_with(unit, request, lease, authority, move |pending| {
            let cache = crate::art_source_cache::Store::new(config.identity_root());
            if worker_lease.is_local() {
                return Ok(pending.fetch_offline(config.identity_root()));
            }
            let client = worker_lease
                .prepare_client(&config)
                .map_err(Error::Configuration)?;
            Ok(pending.fetch_cached(&client, &cache))
        })
    }
    pub(crate) fn start_description_with(
        &mut self,
        unit: UnitId,
        request: crate::art_local_requests::description::Request,
        lease: Arc<Lease>,
        authority: &mut Authority,
        run: impl FnOnce(
                crate::art_local_requests::description::PendingDescription,
            ) -> Result<Completion, Error>
            + Send
            + 'static,
    ) -> Result<Token, Error> {
        self.reserve_job()?;
        let pending = self
            .registry
            .begin_description(unit, request, lease.clone(), authority)?;
        let (unit, token) = pending.identity();
        self.spawn(unit, token.clone(), lease, move || run(pending))?;
        Ok(token)
    }
    pub fn start(
        &mut self,
        unit: UnitId,
        request: Request,
        lease: Arc<Lease>,
        authority: &mut Authority,
        config: Config,
    ) -> Result<Binding, Error> {
        if config.origin() != request.binding.origin {
            return Err(Error::Configuration(
                crate::device_authorization_config::Error::Origin,
            ));
        }
        let worker_lease = lease.clone();
        self.start_with(unit, request, lease, authority, move |pending| {
            let client = worker_lease
                .prepare_client(&config)
                .map_err(Error::Configuration)?;
            Ok(pending.fetch(&client))
        })
    }
    #[cfg(test)]
    pub(crate) fn start_client(
        &mut self,
        unit: UnitId,
        request: Request,
        lease: Arc<Lease>,
        authority: &mut Authority,
        client: crate::device_session_client::Client,
    ) -> Result<Binding, Error> {
        self.start_with(unit, request, lease, authority, move |pending| {
            Ok(pending.fetch(&client))
        })
    }
    fn start_with(
        &mut self,
        unit: UnitId,
        request: Request,
        lease: Arc<Lease>,
        authority: &mut Authority,
        run: impl FnOnce(Pending) -> Result<Completion, Error> + Send + 'static,
    ) -> Result<Binding, Error> {
        self.reserve_job()?;
        let pending = self
            .registry
            .begin(unit, request, lease.clone(), authority)?;
        let binding = pending.binding().clone();
        let (unit, token) = pending.identity();
        self.spawn(unit, token, lease, move || run(pending))?;
        Ok(binding)
    }
    fn reserve_job(&mut self) -> Result<(), Error> {
        // 容量拒绝发生在 Registry 换代之前，不能破坏已有实例。
        if !self.has_capacity() {
            return Err(Error::Capacity);
        }
        self.jobs.try_reserve(1).map_err(|_| Error::Capacity)
    }
    fn spawn(
        &mut self,
        unit: UnitId,
        token: Token,
        lease: Arc<Lease>,
        run: impl FnOnce() -> Result<Completion, Error> + Send + 'static,
    ) -> Result<(), Error> {
        let failure_unit = unit.clone();
        let failure_token = token.clone();
        let ready = Arc::new(AtomicBool::new(false));
        let completed = ready.clone();
        let notify = self.notify.clone();
        let worker = std::thread::Builder::new()
            .name("otter-art-prepare".into())
            .spawn(move || {
                // signal 先声明，run 及其捕获资源在通知前释放；panic 同样发布完成。
                let _signal = CompletionSignal::new(completed, notify);
                let run = run;
                match run() {
                    Ok(completion) => completion,
                    Err(error) => Completion::failed(failure_unit, failure_token, error),
                }
            });
        match worker {
            Ok(worker) => self.jobs.push(Job {
                unit,
                token,
                lease,
                ready,
                worker,
            }),
            Err(_) => {
                self.registry.cancel(&unit);
                return Err(Error::Worker);
            }
        }
        Ok(())
    }
    /// 只 join 已发布完成或已终止的 worker，不在普通 UI 轮询等待网络。
    /// ready 先于 notify 发布，join 可能等待短通知返回，不能将其描述为严格无等待。
    /// 迟到成功/失败均丢弃；它们不得修改当前节点的错误状态或撤销新实例。
    pub fn take_settled(&mut self) -> Option<Report> {
        while let Some(index) = self
            .jobs
            .iter()
            .position(|job| job.ready.load(Ordering::Acquire) || job.worker.is_finished())
        {
            let job = self.jobs.remove(index);
            let completion = job.worker.join().unwrap_or_else(|_| {
                Completion::failed(job.unit.clone(), job.token.clone(), Error::Worker)
            });
            let result = self.registry.accept(completion);
            if matches!(result, Err(Error::Stale)) {
                continue;
            }
            return Some(Report {
                unit: job.unit,
                result,
            });
        }
        None
    }
    pub fn cancel(&mut self, unit: &UnitId) {
        self.registry.cancel(unit);
        // 已替换 worker 的租约也显式撤销，不能只撤销 registry 中的最新项。
        for job in self.jobs.iter().filter(|job| &job.unit == unit) {
            job.lease.revoke();
        }
    }
    pub fn mark_running(&mut self, unit: &UnitId, token: &Token) {
        self.registry.mark_running(unit, token);
    }
    pub fn retain_units(&mut self, keep: impl Fn(&UnitId) -> bool) {
        self.registry.retain_units(&keep);
        for job in &self.jobs {
            if !keep(&job.unit) {
                job.lease.revoke();
            }
        }
    }
    /// 工作区交换同步撤销旧链，但不在 UI join 活线程；清理任务继续占原容量。
    pub fn reset_workspace(&mut self, workspace: String) {
        self.registry = Registry::new(workspace);
        for job in &self.jobs {
            job.lease.revoke();
        }
    }
    /// 退出先撤销全部链再 join；不 detach、不过早释放占槽，也不领取旧工作区结果。
    pub fn shutdown(&mut self) {
        self.registry.clear();
        for job in &self.jobs {
            job.lease.revoke();
        }
        for job in self.jobs.drain(..) {
            let _ = job.worker.join();
        }
    }
}
impl Drop for Tasks {
    fn drop(&mut self) {
        self.shutdown();
    }
}
#[cfg(test)]
#[path = "art_local_description_tasks_tests.rs"]
mod description_tests;
#[cfg(test)]
#[path = "art_local_tasks_tests.rs"]
mod tests;
