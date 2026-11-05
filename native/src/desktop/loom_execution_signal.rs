//! 后台任务的完成栅栏；最后一个局部 owner 释放后再通知，避免 UI 提前轮询漏掉完成。
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub(super) type Notify = Arc<dyn Fn() + Send + Sync>;

pub(super) struct CompletionSignal {
    ready: Arc<AtomicBool>,
    notify: Notify,
}
impl CompletionSignal {
    /// 必须先于任务输入、许可和网络资源声明，确保逆序析构时最后发布。
    pub(super) fn new(ready: Arc<AtomicBool>, notify: Notify) -> Self {
        Self { ready, notify }
    }
}
impl Drop for CompletionSignal {
    fn drop(&mut self) {
        self.ready.store(true, Ordering::Release);
        // 通知失败或第三方回调 panic 不得把正在展开的 worker 变成双重 panic。
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (self.notify)()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completion_publishes_after_resources_and_before_notification() {
        struct Resource(Arc<AtomicBool>);
        impl Drop for Resource {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Release);
            }
        }
        for panic in [false, true] {
            let ready = Arc::new(AtomicBool::new(false));
            let released = Arc::new(AtomicBool::new(false));
            let (tx, rx) = std::sync::mpsc::channel();
            let observed = ready.clone();
            let resource = released.clone();
            let notify = Arc::new(move || {
                tx.send((
                    observed.load(Ordering::Acquire),
                    resource.load(Ordering::Acquire),
                ))
                .unwrap();
            });
            let worker = std::thread::spawn(move || {
                let _signal = CompletionSignal::new(ready, notify);
                let _resource = Resource(released);
                assert!(!panic, "injected worker panic");
            });
            assert_eq!(
                rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap(),
                (true, true)
            );
            assert_eq!(worker.join().is_err(), panic);
        }
    }
    #[test]
    fn notification_panic_does_not_escape_drop() {
        let ready = Arc::new(AtomicBool::new(false));
        drop(CompletionSignal::new(
            ready.clone(),
            Arc::new(|| panic!("injected notifier panic")),
        ));
        assert!(ready.load(Ordering::Acquire));
    }
}
