//! Persist recovery metadata after exit reaping; file I/O runs in the background while UI continues pumping messages during waits.
use super::*;
use crate::{surface_event_record::Record as EventRecord, surface_recovery_record::Record};
use std::{io, path::PathBuf, thread::JoinHandle};

impl Application {
    pub(in crate::desktop::application) fn persist_surface_recovery(&mut self) -> Result<()> {
        let current = persist(
            self.surface_windows.recovery_records(),
            self.remote_sessions.surface_recovery_bundle(),
            save_in_profile,
        );
        // Retired hints do not consume the file budget for unsettled records; failure of either batch does not block the other's attempt.
        finish_retired(
            current,
            self.surface_windows.retired_records(),
            save_in_profile,
        )
    }
}

fn finish_retired(
    current: Result<()>,
    retired: &[Record],
    write: impl FnOnce(Vec<Record>, Vec<EventRecord>) -> Result<()>,
) -> Result<()> {
    if retired.is_empty() {
        return current;
    }
    match (current, write(retired.to_vec(), Vec::new())) {
        (current, Ok(())) => current,
        (Ok(()), Err(error)) => Err(Error::new(
            error.code(),
            format!("Surface retired attachment journal: {error}"),
        )),
        (Err(current), Err(error)) => Err(Error::new(
            current.code(),
            format!("{current}; Surface retired attachment journal: {error}"),
        )),
    }
}

fn persist(
    mut records: Vec<Record>,
    remote: io::Result<(Vec<Record>, Vec<EventRecord>)>,
    write: impl FnOnce(Vec<Record>, Vec<EventRecord>) -> Result<()>,
) -> Result<()> {
    let events = match remote {
        Ok((pending, events)) => {
            records.extend(pending);
            events
        }
        Err(snapshot) => {
            // Remote batches remain indivisible; window hints are independent snapshots preserving uncertain/inFlight unchanged.
            let snapshot = format!("Surface remote recovery snapshot: {snapshot}");
            let message = if records.is_empty() {
                snapshot
            } else {
                match write(records, Vec::new()) {
                    Ok(()) => snapshot,
                    Err(error) => format!("{snapshot}; Surface window recovery write: {error}"),
                }
            };
            return Err(Error::new(E_FAIL, message));
        }
    };
    // Without Surface activity, neither probe user directories nor create empty journals.
    if records.is_empty() && events.is_empty() {
        return Ok(());
    }
    write(records, events)
}

fn save_in_profile(records: Vec<Record>, events: Vec<EventRecord>) -> Result<()> {
    let override_root = std::env::var_os("OTTER_NATIVE_PROFILE_DIR");
    let local = std::env::var_os("LOCALAPPDATA");
    let selected =
        crate::settings_profile::select(None, override_root.as_deref(), local.as_deref(), false)
            .map_err(|error| Error::new(E_FAIL, error))?
            .ok_or_else(|| Error::new(E_FAIL, "Missing native profile"))?;
    let root = selected
        .path
        .parent()
        .ok_or_else(|| Error::new(E_FAIL, "Invalid native profile"))?
        .to_path_buf();
    save(root, records, events).map(|_| ())
}

struct Writer(Option<JoinHandle<io::Result<PathBuf>>>);
impl Drop for Writer {
    fn drop(&mut self) {
        // Wait-API failure must not leave a background writer behind; preserve new files that may contain valid evidence.
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
            // Both file categories have their own quotas and append independently; partial writes must still report errors.
            let attachment = (!records.is_empty())
                .then(|| crate::surface_recovery_store::save(&root, &records))
                .transpose();
            let event = (!events.is_empty())
                .then(|| crate::surface_recovery_store::save_events(&root, &events))
                .transpose();
            match (attachment, event) {
                (Ok(attachment), Ok(event)) => event
                    .or(attachment)
                    .ok_or_else(|| io::ErrorKind::InvalidInput.into()),
                (Err(attachment), Err(event)) => Err(io::Error::new(
                    attachment.kind(),
                    format!(
                        "Surface attachment journal: {attachment}; Surface event journal: {event}"
                    ),
                )),
                (Err(error), Ok(_)) => Err(io::Error::new(
                    error.kind(),
                    format!("Surface attachment journal: {error}"),
                )),
                (Ok(_), Err(error)) => Err(io::Error::new(
                    error.kind(),
                    format!("Surface event journal: {error}"),
                )),
            }
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
    fn event_records() -> Vec<crate::surface_event_record::Record> {
        crate::surface_event_record::decode(br#"{"version":1,"records":[{
            "attachment":{"origin":"https://localhost:1234","device":"viewer:test","node":"node:test","art":null,
                "binding":{"instance":"instance:test","attachment":"attachment:test","generation":1,"lifecycle":"active","revision":2},
                "uncertain":false,"inFlight":false},
            "eventId":"event:test","baseRevision":1,"attempted":true,"ack":null,
            "convergence":{"state":"failed"},"presentation":{"state":"notStarted"}
        }]}"#).unwrap()
    }
    #[test]
    fn journal_can_persist_event_only_metadata_without_an_attachment_file() {
        let events = event_records();
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
    fn journal_persists_window_hints_when_remote_snapshot_fails() {
        let records = crate::surface_recovery_record::decode(br#"{"version":1,"records":[{"origin":"https://localhost:1234","device":"viewer:test","node":"node:test","art":"art:test","binding":{"instance":"instance:test","attachment":"attachment:test","generation":1,"lifecycle":"active","revision":2},"uncertain":true,"inFlight":true}]}"#).unwrap();
        let remote = || Err(io::Error::other("incomplete remote batch"));
        let guid = unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap();
        let root = std::env::temp_dir().join(format!("otter-surface-partial-{guid:?}"));
        let error = persist(records.clone(), remote(), |attachments, events| {
            save(root.clone(), attachments, events).map(|_| ())
        })
        .unwrap_err()
        .to_string();
        assert!(error.contains("incomplete remote batch"), "{error}");
        assert_eq!(
            crate::surface_recovery_store::load(&root.join("surface-recovery/surface-000.json"))
                .unwrap(),
            records
        );
        assert!(!root.join("surface-event-recovery").exists());

        let error = persist(records, remote(), |attachments, events| {
            save(
                PathBuf::from("relative-invalid-profile"),
                attachments,
                events,
            )
            .map(|_| ())
        })
        .unwrap_err()
        .to_string();
        assert!(error.contains("incomplete remote batch"), "{error}");
        assert!(error.contains("Surface attachment journal:"), "{error}");
        let error = persist(Vec::new(), remote(), |_, _| panic!("no window hints"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("incomplete remote batch"), "{error}");
        persist(Vec::new(), Ok((Vec::new(), Vec::new())), |_, _| {
            panic!("empty snapshot")
        })
        .unwrap();
    }
    #[test]
    fn retired_journal_runs_after_current_failure_and_preserves_both_errors() {
        let retired = crate::surface_recovery_record::decode(br#"{"version":1,"records":[{"origin":"https://localhost:1234","device":"viewer:test","node":"node:test","art":"art:test","binding":null,"uncertain":false,"inFlight":false}]}"#).unwrap().remove(0);
        let error = finish_retired(
            Err(Error::new(E_FAIL, "current failed")),
            &[retired],
            |records, events| {
                assert_eq!(records.len(), 1);
                assert!(events.is_empty());
                Err(Error::new(E_FAIL, "retired failed"))
            },
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("current failed") && error.contains("retired failed"),
            "{error}"
        );
    }
    #[test]
    fn journal_attempts_both_streams_when_either_directory_is_unwritable() {
        use crate::surface_recovery_store::{load, load_events};
        use std::fs;
        let attachments = crate::surface_recovery_record::decode(br#"{"version":1,"records":[{"origin":"https://localhost:1234","device":"viewer:test","node":"node:test","art":null,"binding":null,"uncertain":true,"inFlight":false}]}"#).unwrap();
        let events = event_records();
        for (block_attachment, block_event) in [(true, false), (false, true), (true, true)] {
            let guid = unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap();
            let root = std::env::temp_dir().join(format!("otter-surface-journal-{guid:?}"));
            fs::create_dir(&root).unwrap();
            let attachment_dir = root.join("surface-recovery");
            let event_dir = root.join("surface-event-recovery");
            for dir in [
                block_attachment.then_some(&attachment_dir),
                block_event.then_some(&event_dir),
            ]
            .into_iter()
            .flatten()
            {
                fs::write(dir, b"occupied").unwrap();
            }
            let error = save(root, attachments.clone(), events.clone())
                .unwrap_err()
                .to_string();
            assert_eq!(
                error.contains("Surface attachment journal:"),
                block_attachment,
                "{error}"
            );
            assert_eq!(
                error.contains("Surface event journal:"),
                block_event,
                "{error}"
            );
            if block_attachment {
                assert_eq!(fs::read(&attachment_dir).unwrap(), b"occupied");
            } else {
                assert_eq!(
                    load(&attachment_dir.join("surface-000.json")).unwrap(),
                    attachments
                );
            }
            if block_event {
                assert_eq!(fs::read(&event_dir).unwrap(), b"occupied");
            } else {
                assert_eq!(
                    load_events(&event_dir.join("surface-000.json")).unwrap(),
                    events
                );
            }
        }
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
