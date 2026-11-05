//! 联合门的可信测试宿主；只批准两次精确票据，不冒充真实用户原生窗口操作。
use crate::live_relay_http::art_prepare::service::progress::Channel;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
pub(super) struct Host {
    pub channel: Arc<Channel>,
    stop: Arc<AtomicBool>,
    count: Arc<AtomicUsize>,
    worker: Option<JoinHandle<()>>,
}
impl Host {
    pub fn new() -> Self {
        let channel = Arc::new(Channel::new(Arc::new(|| {})));
        let stop = Arc::new(AtomicBool::new(false));
        let count = Arc::new(AtomicUsize::new(0));
        let (observer, stopping, seen) = (channel.clone(), stop.clone(), count.clone());
        let worker = std::thread::spawn(move || {
            let due = Instant::now() + Duration::from_secs(20);
            while !stopping.load(Ordering::Acquire) && Instant::now() < due {
                if let Some(ticket) = observer.confirmations.take() {
                    assert!(ticket.text.contains("publisher.example/local-service"));
                    assert!(seen.fetch_add(1, Ordering::AcqRel) < 2);
                    assert!(ticket.resolve(true));
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        });
        Self {
            channel,
            stop,
            count,
            worker: Some(worker),
        }
    }
    pub fn finish(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.worker.take().unwrap().join().unwrap();
        assert_eq!(self.count.load(Ordering::Acquire), 2);
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.channel.close();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
