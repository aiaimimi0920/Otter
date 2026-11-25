//! WGC 回调只发容量 1 的通知；终止状态独立存储，队列已满也不会丢失取消。
use super::Error;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{sync_channel, Receiver, SyncSender},
    Arc,
};
pub(super) struct Signal {
    sender: SyncSender<()>,
    pub cancel: AtomicBool,
    closed: AtomicBool,
}
impl Signal {
    pub fn new() -> (Arc<Self>, Receiver<()>) {
        let (sender, receiver) = sync_channel(1);
        (
            Arc::new(Self {
                sender,
                cancel: AtomicBool::new(false),
                closed: AtomicBool::new(false),
            }),
            receiver,
        )
    }
    pub fn frame(&self) {
        let _ = self.sender.try_send(());
    }
    pub fn stop(&self) {
        self.cancel.store(true, Ordering::Release);
        self.frame();
    }
    pub fn source_closed(&self) {
        self.closed.store(true, Ordering::Release);
        self.cancel.store(true, Ordering::Release);
        self.frame();
    }
    pub fn check(&self) -> Result<(), Error> {
        if self.closed.load(Ordering::Acquire) {
            Err(Error::SourceClosed)
        } else if self.cancel.load(Ordering::Acquire) {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
    pub fn wait(&self, receiver: &Receiver<()>) -> Result<(), Error> {
        self.check()?;
        receiver.recv().map_err(|_| Error::Cancelled)?;
        self.check()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notifications_are_coalesced_and_full_queue_does_not_hide_stop() {
        let (signal, receiver) = Signal::new();
        for _ in 0..1000 {
            signal.frame();
        }
        receiver.try_recv().unwrap();
        assert!(receiver.try_recv().is_err());
        signal.frame();
        signal.stop();
        assert!(matches!(signal.wait(&receiver), Err(Error::Cancelled)));
        signal.source_closed();
        assert!(matches!(signal.wait(&receiver), Err(Error::SourceClosed)));
    }
    #[test]
    fn stop_wakes_an_idle_worker_without_periodic_polling() {
        let (signal, receiver) = Signal::new();
        let worker_signal = signal.clone();
        let (sender, result) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            sender
                .send(matches!(
                    worker_signal.wait(&receiver),
                    Err(Error::Cancelled)
                ))
                .unwrap();
        });
        signal.stop();
        assert!(result
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap());
        worker.join().unwrap();
    }
    #[test]
    fn dropping_last_frame_consumer_wakes_capture_waiter() {
        let frames = crate::live_frames::Source::<u8>::new();
        let consumer = frames.subscribe().unwrap();
        let (signal, receiver) = Signal::new();
        let idle = signal.clone();
        frames.on_idle(Arc::new(move || idle.stop())).unwrap();
        let (sender, result) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            sender
                .send(matches!(signal.wait(&receiver), Err(Error::Cancelled)))
                .unwrap();
        });
        drop(consumer);
        assert!(result
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap());
        worker.join().unwrap();
    }
}
