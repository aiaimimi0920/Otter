//! 设备授权后台任务 owner：有界任务数、取消、完成句柄和非阻塞结果回收。
use crate::{
    device_approval, device_identity_store::Store, device_pairing, device_request_cancel::Cancel,
    device_session_attempt::Session, device_session_client::Client,
};
use std::{
    io,
    os::windows::io::{AsHandle, BorrowedHandle},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

static ACTIVE: AtomicUsize = AtomicUsize::new(0);
struct Slot;
impl Slot {
    fn acquire() -> io::Result<Self> {
        ACTIVE
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < 4).then_some(n + 1)
            })
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "Device authorization task limit reached",
                )
            })?;
        Ok(Self)
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::AcqRel);
    }
}

/// 分开显式配对、首次批准等待和续签，禁止续签失败后偷偷改为重新配对。
pub enum Task {
    ClearOffline {
        config: crate::device_authorization_config::Config,
    },
    Configured {
        config: crate::device_authorization_config::Config,
        action: crate::device_authorization_config::Action,
    },
    RequestPairing {
        client: Client,
        store: Store,
        name: String,
    },
    AwaitApproval {
        client: Client,
        store: Store,
    },
    Renew {
        client: Client,
        store: Store,
    },
}
impl Task {
    pub(super) fn origin(&self) -> &str {
        match self {
            Self::Configured { config, .. } | Self::ClearOffline { config } => config.origin(),
            Self::RequestPairing { client, .. }
            | Self::AwaitApproval { client, .. }
            | Self::Renew { client, .. } => client.origin(),
        }
    }
}
// 包含凭据的结果不实现 Debug/Clone/Serialize。
pub enum Output {
    OfflineCleared,
    Pairing { device: String, changed: bool },
    Session(Session),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Preparation(crate::device_authorization_config::Error),
    Stopped,
    Panicked,
    Store(crate::device_identity_store::Error),
    Pairing(device_pairing::Error),
    Approval(device_approval::Error),
    Session(crate::device_session_client::Error),
}
pub struct Runtime {
    cancel: Arc<Cancel>,
    thread: Option<JoinHandle<Result<Output, Error>>>,
    // 完成但未回收的 token/线程句柄也占用名额，不允许无限堆积已完成任务。
    slot: Option<Slot>,
}
struct Clear(Arc<Cancel>);
impl Drop for Clear {
    fn drop(&mut self) {
        self.0.clear();
    }
}
impl Runtime {
    pub fn start(task: Task) -> io::Result<Self> {
        Self::spawn(move |cancel| run(task, cancel))
    }
    fn spawn(
        work: impl FnOnce(&Cancel) -> Result<Output, Error> + Send + 'static,
    ) -> io::Result<Self> {
        let slot = Slot::acquire()?;
        let cancel = Arc::new(Cancel::new());
        let retired = Clear(cancel.clone());
        let thread = std::thread::Builder::new()
            .name("otter-device-auth".into())
            .spawn(move || {
                let _retired = retired;
                work(&_retired.0)
            })?;
        Ok(Self {
            cancel,
            thread: Some(thread),
            slot: Some(slot),
        })
    }
    pub fn request_stop(&self) {
        self.cancel.cancel();
    }
    pub fn wait_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.thread.as_ref().map(|thread| thread.as_handle())
    }
    pub fn try_join(&mut self) -> Option<Result<Output, Error>> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        let result = self.thread.take()?.join().unwrap_or(Err(Error::Panicked));
        self.slot.take();
        // 即使后台已成功返回，关闭/替换任务后也不能交出旧凭据；被丢弃 token 会清零。
        if self.cancel.is_stopped() {
            Some(Err(Error::Stopped))
        } else {
            Some(result)
        }
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.request_stop();
        // 正常 UI 路径必须等待完成句柄并 try_join；Drop 只是防孤儿线程的阻塞兜底。
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
fn run(task: Task, cancel: &Cancel) -> Result<Output, Error> {
    // 明确减权不同于可取消的联网请求：关闭只丢弃回执，不能取消已经接受的持久清除意图。
    if let Task::ClearOffline { config } = task {
        config.clear_offline().map_err(Error::Preparation)?;
        return Ok(Output::OfflineCleared);
    }
    if cancel.is_stopped() {
        return Err(Error::Stopped);
    }
    let output = match task {
        Task::ClearOffline { .. } => unreachable!("handled before cancellation"),
        Task::Configured { config, action } => {
            let prepared = config.prepare(action, cancel).map_err(Error::Preparation)?;
            return run(prepared, cancel);
        }
        Task::RequestPairing {
            client,
            store,
            name,
        } => {
            let mut loaded = store.load_or_create().map_err(Error::Store)?;
            let changed = device_pairing::register_and_save(
                &client,
                &store,
                &mut loaded,
                &name,
                cancel.flag(),
                |handle| cancel.install(handle),
            )
            .map_err(Error::Pairing)?;
            let device = loaded
                .identity()
                .device_id()
                .ok_or(Error::Store(crate::device_identity_store::Error::Identity(
                    crate::device_identity::Error::Identity,
                )))?
                .to_owned();
            Output::Pairing { device, changed }
        }
        Task::AwaitApproval { client, store } => {
            let loaded = store.load_existing().map_err(Error::Store)?;
            Output::Session(
                device_approval::wait(&client, loaded.identity(), cancel)
                    .map_err(Error::Approval)?,
            )
        }
        Task::Renew { client, store } => {
            let loaded = store.load_existing().map_err(Error::Store)?;
            Output::Session(
                client
                    .issue_once(loaded.identity(), cancel.flag(), |handle| {
                        cancel.install(handle)
                    })
                    .map_err(Error::Session)?,
            )
        }
    };
    if cancel.is_stopped() {
        Err(Error::Stopped)
    } else {
        Ok(output)
    }
}

#[cfg(test)]
#[path = "device_authorization_runtime_tests.rs"]
mod tests;
