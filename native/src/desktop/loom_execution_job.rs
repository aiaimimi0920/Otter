//! 有界后台 Art 任务；更新合并为单槽，完成结果持有待释放租约，不访问窗口。
use super::{
    loom_execution_session::{Failure, Final, Session, Update},
    loom_input_job::PreparedBatch,
};
use crate::{
    graph_requests::Request, loom_bridge_identity::Identity, loom_bridge_stream::Interrupt,
    workspace_id::UnitId,
};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
struct Permit;
impl Drop for Permit {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::AcqRel);
    }
}
pub struct Execution {
    /// 本地可信安装与冻结参数；不来自 execute JSON，也不更换远端身份/清理 owner。
    pub shader: Option<super::shader_prepare::Runtime>,
    pub identity: Identity,
    pub unit: UnitId,
    pub request: Request,
    pub message: String,
    pub inputs: PreparedBatch,
    pub timeout: Duration,
}
pub struct Completion {
    // 可选的大上下文不膨胀每个共享任务的 Completion 枚举；配方数据仍通过 Arc 共享。
    pub(super) shader_recipe: Option<Box<super::shader_recipe_cache::Acquired>>,
    pub identity: Identity,
    pub unit: UnitId,
    pub request: Request,
    pub result: Result<Final, Failure>,
    pub remote_cancel:
        Option<Result<crate::loom_art_control::Outcome, crate::loom_art_control::Error>>,
    pub cleanup_retries: u8,
}
#[path = "loom_execution_cleanup.rs"]
mod cleanup;
use super::loom_execution_signal::{CompletionSignal, Notify};
#[derive(Debug)]
pub enum Error {
    Busy,
    InvalidTarget,
    WorkerLimit,
    Spawn(std::io::Error),
    WorkerFailed,
}
#[derive(Default)]
struct Mailbox {
    progress: Option<f64>,
    preview: Option<Update>,
    interrupt: Option<Interrupt>,
}
struct Running {
    ready: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
    mailbox: Arc<Mutex<Mailbox>>,
    worker: JoinHandle<Completion>,
}
#[derive(Default)]
pub struct Job {
    running: Option<Running>,
    completed_mailbox: Option<Arc<Mutex<Mailbox>>>,
}
impl Job {
    pub fn is_busy(&self) -> bool {
        self.running.is_some()
    }
    /// 注册、输入新鲜度检查及旧请求取消由应用 owner 在调用前处理；失败不能遗留活动请求。
    pub fn start(&mut self, execution: Execution) -> Result<(), Error> {
        self.start_with_notify(execution, Arc::new(|| {}))
    }
    /// 通知只唤醒 owner，不得访问窗口模型、阻塞或 panic；更新在释放邮箱锁后通知。
    pub fn start_with_notify(
        &mut self,
        mut execution: Execution,
        notify: Notify,
    ) -> Result<(), Error> {
        let runtime = execution.shader.take();
        self.start_pipeline(execution, notify, runtime)
    }
    fn start_pipeline(
        &mut self,
        execution: Execution,
        notify: Notify,
        runtime: Option<super::shader_prepare::Runtime>,
    ) -> Result<(), Error> {
        if self.is_busy() {
            return Err(Error::Busy);
        }
        self.completed_mailbox = None;
        if execution.inputs.target != execution.unit {
            return Err(Error::InvalidTarget);
        }
        ACTIVE
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < 2).then_some(count + 1)
            })
            .map_err(|_| Error::WorkerLimit)?;
        let permit = Permit;
        let cancel = Arc::new(AtomicBool::new(false));
        let mailbox = Arc::new(Mutex::new(Mailbox::default()));
        let signal = cancel.clone();
        let updates = mailbox.clone();
        let ready = Arc::new(AtomicBool::new(false));
        let completed = ready.clone();
        let worker = thread::Builder::new()
            .name("otter-loom-execution".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify.clone());
                let _permit = permit;
                let inputs = execution.inputs;
                // 提前取消也要在完成信号前释放可信 bundle 和其它捕获资源。
                let runtime = runtime;
                let session = Session {
                    identity: &execution.identity,
                    unit: &execution.unit,
                    request: &execution.request,
                    cancel: &signal,
                    timeout: execution.timeout,
                };
                let mut connected = false;
                let mut result = session.run(
                    execution.message,
                    |interrupt| {
                        connected = true;
                        let mut mailbox = updates.lock().unwrap_or_else(|error| error.into_inner());
                        if signal.load(Ordering::Acquire) {
                            let _ = interrupt.cancel();
                        }
                        mailbox.interrupt = Some(interrupt);
                    },
                    |update| {
                        if signal.load(Ordering::Acquire) {
                            return false;
                        }
                        let mut mailbox = updates.lock().unwrap_or_else(|error| error.into_inner());
                        match update {
                            Update::Progress(value) => mailbox.progress = Some(value),
                            preview => mailbox.preview = Some(preview),
                        }
                        drop(mailbox);
                        notify();
                        true
                    },
                );
                updates
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .interrupt = None;
                // 成功完成/明确远端失败不重复取消；本地异常只在曾连接时发送，避免凭空取消。
                let remote_cancel = if connected
                    && result.is_err()
                    && !matches!(&result, Err(Failure::Remote(_)))
                {
                    Some(crate::loom_art_control::cancel_authenticated(
                        &execution.identity,
                        &execution.unit,
                        &execution.request,
                    ))
                } else {
                    None
                };
                // 已成功终态后，本地失败/取消不再取消远端已完成取码；失败租约仍走原补偿。
                let mut shader_recipe = None;
                if let (Ok(final_result), Some(runtime)) = (&mut result, runtime) {
                    match super::shader_fetch_render::render(
                        final_result,
                        &inputs,
                        runtime,
                        &signal,
                    ) {
                        Ok(recipe) => shader_recipe = recipe.map(Box::new),
                        Err(error) => result = Err(Failure::Shader(error)),
                    }
                }
                let mut completion = Completion {
                    shader_recipe,
                    identity: execution.identity,
                    unit: execution.unit,
                    request: execution.request,
                    result,
                    remote_cancel,
                    cleanup_retries: 0,
                };
                completion.retry_cleanup();
                completion
            })
            .map_err(Error::Spawn)?;
        self.running = Some(Running {
            ready,
            cancel,
            mailbox,
            worker,
        });
        Ok(())
    }
    pub fn cancel(&self) {
        if let Some(running) = &self.running {
            running.cancel.store(true, Ordering::Release);
            let mut mailbox = running
                .mailbox
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if let Some(interrupt) = &mailbox.interrupt {
                let _ = interrupt.cancel();
            }
            mailbox.progress = None;
            mailbox.preview = None;
        }
    }
    /// 只有已释放远端租约的预览会进入单槽，覆盖旧预览仅回收本地像素。
    pub fn take_updates(&self) -> (Option<f64>, Option<Update>) {
        let Some(mailbox) = self
            .running
            .as_ref()
            .map(|running| &running.mailbox)
            .or(self.completed_mailbox.as_ref())
        else {
            return (None, None);
        };
        let mut mailbox = mailbox.lock().unwrap_or_else(|error| error.into_inner());
        if self
            .running
            .as_ref()
            .is_some_and(|running| running.cancel.load(Ordering::Acquire))
        {
            mailbox.progress = None;
            mailbox.preview = None;
        }
        (mailbox.progress.take(), mailbox.preview.take())
    }
    /// 完成栅栏发布后不再执行网络清理；join 仍可能等待通知回调及线程最终退出调度。
    pub fn poll(&mut self) -> Option<Result<Completion, Error>> {
        if !self.running.as_ref().is_some_and(|running| {
            running.ready.load(Ordering::Acquire) || running.worker.is_finished()
        }) {
            return None;
        }
        Some(self.join())
    }
    /// 应用关闭/替换任务必须显式调用并处理返回的 PendingRelease，不能仅丢弃 Job。
    pub fn shutdown(&mut self) -> Option<Result<Completion, Error>> {
        self.running.as_ref()?;
        self.cancel();
        Some(self.join())
    }
    fn join(&mut self) -> Result<Completion, Error> {
        let running = self.running.take().expect("running task");
        let mut completion = running.worker.join().map_err(|_| Error::WorkerFailed)?;
        if running.cancel.load(Ordering::Acquire) && completion.result.is_ok() {
            completion.result = Err(Failure::Cancelled);
        }
        if !running.cancel.load(Ordering::Acquire) {
            self.completed_mailbox = Some(running.mailbox);
        }
        Ok(completion)
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        // 安全网不泄漏线程；正常应用路径必须 shutdown 并把失败租约移交重试 owner。
        if let Some(Ok(completion)) = self.shutdown() {
            if completion.cleanup_pending() {
                eprintln!("otter-native: execution dropped with unresolved remote cleanup");
            }
        }
    }
}
