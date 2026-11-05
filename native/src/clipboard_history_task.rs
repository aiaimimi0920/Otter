//! 菜单读取任务：先取得进程许可再启动线程，UI 关闭只取消，不等待提供方。
use super::{ReadPermit, Snapshot};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, TryRecvError},
    Arc,
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_FAIL,
};

/// 每个任务独占结果通道；旧菜单销毁后，旧结果无法进入新菜单的任务。
/// 消费者仍须在应用结果前核验窗口、文档和菜单目标身份。
pub struct Task {
    cancel: Arc<AtomicBool>,
    receiver: Option<Receiver<Result<Snapshot>>>,
}

impl Task {
    pub fn start() -> Result<Self> {
        Self::spawn(super::read_with_permit)
    }

    pub(super) fn spawn(
        read: impl FnOnce(&AtomicBool, ReadPermit) -> Result<Snapshot> + Send + 'static,
    ) -> Result<Self> {
        // 必须在 spawn 前占位；阻塞的提供方不能导致重复菜单创建无限线程。
        let permit = ReadPermit::acquire()?;
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let (sender, receiver) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("clipboard-history".into())
            .spawn(move || {
                let result = read(&worker_cancel, permit);
                if !worker_cancel.load(Ordering::Acquire) {
                    // 单个有界结果；接收端消失时内容随发送失败自动释放。
                    let _ = sender.send(result);
                }
            })
            .map_err(|_| Error::new(E_FAIL, "Clipboard history worker could not start"))?;
        Ok(Self {
            cancel,
            receiver: Some(receiver),
        })
    }

    /// 非阻塞且结果只交付一次。None 表示尚未完成，或已经消费/取消。
    pub fn poll(&mut self) -> Option<Result<Snapshot>> {
        let receiver = self.receiver.as_ref()?;
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err(Error::new(
                E_FAIL,
                "Clipboard history worker ended without a result",
            )),
        };
        self.receiver.take();
        Some(result)
    }

    pub fn cancel(&mut self) {
        self.cancel.store(true, Ordering::Release);
        self.receiver.take();
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        // 不 join：提供方可能不响应 Cancel；后台线程继续持有许可及 COM 资源。
        self.cancel();
    }
}

#[cfg(test)]
#[path = "clipboard_history_task_tests.rs"]
mod tests;
