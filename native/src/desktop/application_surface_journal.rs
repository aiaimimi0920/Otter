//! 退出回收后的恢复元数据落盘；文件 I/O 在后台，UI 等待期间继续泵消息。
use super::*;
use crate::surface_recovery_record::Record;
use std::{io, path::PathBuf, thread::JoinHandle};

impl Application {
    pub(in crate::desktop::application) fn persist_surface_recovery(&mut self) -> Result<()> {
        let mut records = self.surface_windows.recovery_records();
        let (pending, events) = self
            .remote_sessions
            .surface_recovery_bundle()
            .map_err(|error| Error::new(E_FAIL, error.to_string()))?;
        records.extend(pending);
        // 无 Surface 活动的隐藏测试/普通退出不探测用户目录，更不创建空日志。
        if records.is_empty() && events.is_empty() {
            return Ok(());
        }
        let override_root = std::env::var_os("OTTER_NATIVE_PROFILE_DIR");
        let local = std::env::var_os("LOCALAPPDATA");
        let selected = crate::settings_profile::select(
            None,
            override_root.as_deref(),
            local.as_deref(),
            false,
        )
        .map_err(|error| Error::new(E_FAIL, error))?
        .ok_or_else(|| Error::new(E_FAIL, "Missing native profile"))?;
        let root = selected
            .path
            .parent()
            .ok_or_else(|| Error::new(E_FAIL, "Invalid native profile"))?
            .to_path_buf();
        save(root, records, events).map(|_| ())
    }
}

struct Writer(Option<JoinHandle<io::Result<PathBuf>>>);
impl Drop for Writer {
    fn drop(&mut self) {
        // 等待 API 失败时仍不遗留后台写线程；不删除可能含有效证据的新文件。
        if let Some(thread) = self.0.take() {
            let _ = thread.join();
        }
    }
}
struct Quit(Option<i32>);
impl Drop for Quit {
    fn drop(&mut self) {
        if let Some(code) = self.0 {
            unsafe { PostQuitMessage(code) };
        }
    }
}
fn save(
    root: PathBuf,
    records: Vec<Record>,
    events: Vec<crate::surface_event_record::Record>,
) -> Result<PathBuf> {
    let thread = std::thread::Builder::new()
        .name("otter-surface-journal".into())
        .spawn(move || {
            // 两类文件分别只增不覆盖；第二步失败也保留第一步证据并整体报错。
            let attachment = if records.is_empty() {
                None
            } else {
                Some(crate::surface_recovery_store::save(&root, &records)?)
            };
            let event = if events.is_empty() {
                None
            } else {
                Some(crate::surface_recovery_store::save_events(&root, &events)?)
            };
            event
                .or(attachment)
                .ok_or_else(|| io::ErrorKind::InvalidInput.into())
        })
        .map_err(|error| Error::new(E_FAIL, error.to_string()))?;
    let mut writer = Writer(Some(thread));
    let mut quit = Quit(None);
    while !writer.0.as_ref().unwrap().is_finished() {
        let handle = HANDLE(writer.0.as_ref().unwrap().as_raw_handle());
        let mut message = MSG::default();
        unsafe {
            if wait::next(&[handle], &mut message)? == 0 {
                quit.0 = Some(message.wParam.0 as i32);
            } else {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
    writer
        .0
        .take()
        .unwrap()
        .join()
        .map_err(|_| Error::new(E_FAIL, "Surface recovery writer panicked"))?
        .map_err(|error| Error::new(E_FAIL, error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn journal_can_persist_event_only_metadata_without_an_attachment_file() {
        let events = crate::surface_event_record::decode(br#"{"version":1,"records":[{
            "attachment":{"origin":"https://localhost:1234","device":"viewer:test","node":"node:test","art":null,
                "binding":{"instance":"instance:test","attachment":"attachment:test","generation":1,"lifecycle":"active","revision":2},
                "uncertain":false,"inFlight":false},
            "eventId":"event:test","baseRevision":1,"attempted":true,"ack":null,
            "convergence":{"state":"failed"},"presentation":{"state":"notStarted"}
        }]}"#).unwrap();
        let guid = unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap();
        let root = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi")
            .join(format!("surface-event-journal-{guid:?}"));
        let path = save(root.clone(), Vec::new(), events.clone()).unwrap();
        assert_eq!(
            crate::surface_recovery_store::load_events(&path).unwrap(),
            events
        );
        assert!(!root.join("surface-recovery").exists());
    }
    #[test]
    fn journal_worker_publishes_metadata_and_preserves_quit_on_success_and_error() {
        let records = crate::surface_recovery_record::decode(br#"{"version":1,"records":[{"origin":"https://localhost:1234","device":"viewer:test","node":"node:test","art":"art:test","binding":null,"uncertain":true,"inFlight":false}]}"#).unwrap();
        let guid = unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap();
        let root = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi")
            .join(format!("surface-exit-journal-{guid:?}"));
        unsafe {
            PostQuitMessage(37);
        }
        let path = save(root, records.clone(), Vec::new()).unwrap();
        assert_eq!(crate::surface_recovery_store::load(&path).unwrap(), records);
        let mut message = MSG::default();
        assert!(unsafe { PeekMessageW(&mut message, None, WM_QUIT, WM_QUIT, PM_REMOVE) }.as_bool());
        assert_eq!(message.wParam.0, 37);
        unsafe {
            PostQuitMessage(41);
        }
        assert!(save(
            PathBuf::from("relative-invalid-profile"),
            records,
            Vec::new()
        )
        .is_err());
        assert!(unsafe { PeekMessageW(&mut message, None, WM_QUIT, WM_QUIT, PM_REMOVE) }.as_bool());
        assert_eq!(message.wParam.0, 41);
    }
}
