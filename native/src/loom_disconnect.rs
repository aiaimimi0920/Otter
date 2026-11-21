//! 主动断开使用一次性的原会话凭据；任务有界且退出时回收，不轮换设备身份。
use super::*;
pub(super) struct Job {
    cancel: Arc<Cancel>,
    thread: Option<JoinHandle<()>>,
    result: mpsc::Receiver<Result<(), Error>>,
}
impl Job {
    pub(super) fn start(
        config: crate::device_authorization_config::Config,
        token: zeroize::Zeroizing<String>,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Self, Error> {
        let cancel = Arc::new(Cancel::new());
        let stop = cancel.clone();
        let (send, result) = mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("otter-disconnect".into())
            .spawn(move || {
                let result = (|| {
                    let client = config.prepare_client(&stop).map_err(|_| Error::Tls)?;
                    let nonce = format!(
                        "{:?}",
                        unsafe { windows::Win32::System::Com::CoCreateGuid() }
                            .map_err(|_| Error::Worker)?
                    );
                    let (status, _) = client
                        .exchange(
                            crate::device_session_client::Request {
                                method: "POST",
                                path: "/v1/otter/connections/disconnect",
                                body: b"{}",
                                device: Some((&token, &nonce)),
                                limit: 4096,
                            },
                            stop.flag(),
                            &mut |h| stop.install(h),
                            Instant::now() + Duration::from_secs(5),
                        )
                        .map_err(|_| Error::Tls)?;
                    stop.clear();
                    if status == 200 || status == 401 {
                        Ok(())
                    } else {
                        Err(Error::Response)
                    }
                })();
                let _ = send.send(result);
                wake();
            })
            .map_err(|_| Error::Worker)?;
        Ok(Self {
            cancel,
            thread: Some(thread),
            result,
        })
    }
    pub(super) fn poll(&mut self) -> Option<Result<(), Error>> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(Error::Worker)),
        }
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.cancel();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
