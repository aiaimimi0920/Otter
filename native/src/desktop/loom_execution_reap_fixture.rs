//! 测试专用的通知尾段门与真实线程退出等待；每次阻塞等待最多五秒。
use super::{Tasks, Work};
use crate::desktop::loom_execution_signal::Notify;
use std::{
    os::windows::io::AsRawHandle,
    sync::{mpsc, Arc, Mutex},
    thread::JoinHandle,
    time::Duration,
};
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForSingleObject,
};

pub(crate) const WAIT: Duration = Duration::from_secs(5);

pub(crate) struct Gate {
    pub notify: Notify,
    pub noticed: mpsc::Receiver<()>,
    pub release: mpsc::Sender<()>,
}
impl Gate {
    pub fn new(wake: impl Fn() + Send + Sync + 'static) -> Self {
        let (notify, noticed) = mpsc::channel();
        let (release, waiting) = mpsc::channel();
        let waiting = Mutex::new(waiting);
        Self {
            notify: Arc::new(move || {
                wake();
                notify.send(()).unwrap();
                // 旧的提前 join 实现也会有界退出并使主线程断言失败。
                let _ = waiting.lock().unwrap().recv_timeout(WAIT);
            }),
            noticed,
            release,
        }
    }
}

pub(crate) fn wait_finished<T>(worker: &JoinHandle<T>) {
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(worker.as_raw_handle()), 5_000) },
        WAIT_OBJECT_0,
        "execution worker did not exit"
    );
}

impl Tasks {
    pub(crate) fn test_wait_finished(&self, index: usize) {
        match &self.entries[index].job {
            Work::Remote(job) => job.test_wait_finished(),
            Work::Shader(job) => job.test_wait_finished(),
        }
    }
    pub(crate) fn test_reap_deadline(&self) -> Option<std::time::Instant> {
        self.deadline()
    }
}
