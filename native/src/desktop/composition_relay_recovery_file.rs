//! 用户显式选择文件后后台读取；共享远程任务容量和完成等待，不在菜单回调读盘。
use super::*;
use crate::surface_recovery_record::Record;
use std::{os::windows::io::AsHandle, path::PathBuf, thread::JoinHandle};
pub(super) struct Job {
    thread: Option<JoinHandle<io::Result<Vec<Record>>>>,
    discarded: bool,
}
impl Job {
    pub(super) fn handle(&self) -> BorrowedHandle<'_> {
        self.thread.as_ref().unwrap().as_handle()
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
impl Sessions {
    pub fn reading_recovery_file(&self) -> bool {
        self.recovery_file_job.is_some()
    }
    pub fn querying_recovery(&self) -> bool {
        !self.recovery_jobs.is_empty()
    }
    pub fn read_recovery_file(&mut self, path: PathBuf) -> io::Result<()> {
        if self.closed || self.recovery_file_job.is_some() || self.occupied() >= MAX_SESSIONS {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        if !path.is_absolute() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let thread = std::thread::Builder::new()
            .name("otter-recovery-file".into())
            .spawn(move || crate::surface_recovery_store::load(&path))?;
        self.recovery_file_job = Some(Job {
            thread: Some(thread),
            discarded: false,
        });
        Ok(())
    }
    pub(super) fn cancel_recovery_file(&mut self) {
        self.recovery_file = None;
        if let Some(job) = &mut self.recovery_file_job {
            job.discarded = true;
        }
    }
    pub(super) fn poll_recovery_file(&mut self) {
        if !self
            .recovery_file_job
            .as_ref()
            .is_some_and(|job| job.thread.as_ref().unwrap().is_finished())
        {
            return;
        }
        let mut job = self.recovery_file_job.take().unwrap();
        let result = job
            .thread
            .take()
            .unwrap()
            .join()
            .unwrap_or_else(|_| Err(io::Error::other("Recovery file reader panicked")));
        if !job.discarded && !self.closed {
            self.recovery_file = Some(result);
        }
    }
    pub fn take_recovery_file(&mut self) -> Option<io::Result<Vec<Record>>> {
        self.recovery_file.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> PathBuf {
        let guid = unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap();
        let path = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi")
            .join(format!("surface-recovery-read-{guid:?}.json"));
        std::fs::write(&path, br#"{"version":1,"records":[]}"#).unwrap();
        path
    }
    fn drain(sessions: &mut Sessions) {
        let end = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while sessions.reading_recovery_file() {
            assert!(std::time::Instant::now() < end);
            sessions.poll_surfaces();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    #[test]
    fn selected_file_uses_shared_capacity_and_delivers_once() {
        let path = fixture();
        let mut sessions = Sessions::default();
        sessions.read_recovery_file(path.clone()).unwrap();
        assert_eq!(sessions.occupied(), 1);
        assert!(!sessions.finished());
        assert_eq!(sessions.wait_handles().count(), 1);
        assert!(sessions.read_recovery_file(path.clone()).is_err());
        drain(&mut sessions);
        assert_eq!(sessions.occupied(), 1);
        assert!(sessions.take_recovery_file().unwrap().unwrap().is_empty());
        assert!(sessions.take_recovery_file().is_none());
        assert_eq!(sessions.occupied(), 0);
        assert!(sessions.finished());
        assert!(path.exists());
    }
    #[test]
    fn cancel_and_exit_discard_local_delivery_but_reap_worker_and_keep_file() {
        for close in [false, true] {
            let path = fixture();
            let mut sessions = Sessions::default();
            sessions.read_recovery_file(path.clone()).unwrap();
            if close {
                sessions.close();
            } else {
                sessions.cancel_surface_requests();
            }
            drain(&mut sessions);
            assert!(sessions.take_recovery_file().is_none());
            assert!(sessions.finished());
            assert_eq!(sessions.wait_handles().count(), 0);
            assert!(path.exists());
        }
    }
}
